use super::super::WorldRepository;
use crate::config::ServerConfig;
use tarrowyn_protocol::{ChatRequest, GuestSessionRequest, MovementIntent};

#[test]
fn persistence_backend_rejects_unknown_driver_before_world_start() {
    let config = ServerConfig {
        db_driver: "sqlite".to_owned(),
        ..ServerConfig::default()
    };
    let error = WorldRepository::try_new(config).err().unwrap();
    assert!(error.contains("use `json` or `mysql`"));
}

#[test]
fn mysql_backend_requires_a_database_name_before_connecting() {
    let config = ServerConfig {
        db_driver: "mysql".to_owned(),
        db_database: String::new(),
        ..ServerConfig::default()
    };
    let error = WorldRepository::try_new(config).err().unwrap();
    assert!(error.contains("DB_DATABASE must be non-empty"));
}

#[test]
fn corrupt_json_snapshot_fails_closed_without_overwriting_the_file() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-corrupt-state-{}.json",
        std::process::id()
    ));
    let contents = b"{ this is not a Tarrowyn snapshot }";
    std::fs::write(&path, contents).unwrap();
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    };

    let error = WorldRepository::try_new(config).err().unwrap();
    assert!(error.contains("invalid state JSON"));
    assert_eq!(std::fs::read(&path).unwrap(), contents);
    let _ = std::fs::remove_file(path);
}

#[test]
fn chat_and_movement_replays_survive_repository_restart() {
    let path =
        std::env::temp_dir().join(format!("tarrowyn-core-replay-{}.json", std::process::id()));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        movement_cooldown_ticks: 0,
        ..ServerConfig::default()
    };
    let first = WorldRepository::new(config.clone());
    let session = first
        .guest_session(GuestSessionRequest {
            client_key: Some("core-replay".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let chat_request = ChatRequest {
        request_id: "core-chat-replay".to_owned(),
        channel: "settlement".to_owned(),
        text: "The same word should appear once.".to_owned(),
    };
    let movement_request = MovementIntent {
        request_id: "core-movement-replay".to_owned(),
        dx: 0,
        dy: 1,
    };
    let chat = first
        .chat(&session.account_token, chat_request.clone())
        .unwrap();
    let movement = first
        .movement(&session.account_token, movement_request.clone())
        .unwrap();
    assert!(chat.data.accepted);
    assert!(movement.data.accepted);
    drop(first);

    let second = WorldRepository::new(config);
    let resumed = second
        .guest_session(GuestSessionRequest {
            client_key: Some("core-replay".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    assert_eq!(
        second
            .chat(&resumed.account_token, chat_request)
            .unwrap()
            .data,
        chat.data
    );
    assert_eq!(
        second
            .movement(&resumed.account_token, movement_request)
            .unwrap()
            .data,
        movement.data
    );
    let _ = std::fs::remove_file(path);
}
