use super::super::WorldRepository;
use crate::config::ServerConfig;
use tarrowyn_protocol::{
    ChatRequest, ChatResponse, GuestSessionRequest, MovementIntent, MovementResponse, Position,
    SupportRepairResponse,
};

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
fn newer_json_snapshot_fails_closed_without_downgrading_the_file() {
    let path =
        std::env::temp_dir().join(format!("tarrowyn-newer-state-{}.json", std::process::id()));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    };
    let repository = WorldRepository::new(config.clone());
    repository
        .guest_session(GuestSessionRequest {
            client_key: Some("newer-state".to_owned()),
            reset: false,
        })
        .unwrap();
    drop(repository);

    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    snapshot["storage_version"] = serde_json::json!(u32::MAX);
    let encoded = serde_json::to_vec_pretty(&snapshot).unwrap();
    std::fs::write(&path, &encoded).unwrap();

    let error = WorldRepository::try_new(config).err().unwrap();
    assert!(error.contains("newer server version"));
    assert_eq!(std::fs::read(&path).unwrap(), encoded);
    let _ = std::fs::remove_file(path);
}

#[test]
fn persistence_failure_degrades_operator_readiness() {
    let state_path = std::env::temp_dir().join(format!(
        "tarrowyn-persistence-failure-{}.json",
        std::process::id()
    ));
    let repository = WorldRepository::new(ServerConfig {
        persistence_path: Some(state_path.to_string_lossy().into_owned()),
        backup_path: None,
        ..ServerConfig::default()
    });
    std::fs::remove_file(&state_path).unwrap();
    std::fs::create_dir(&state_path).unwrap();

    repository
        .guest_session(GuestSessionRequest {
            client_key: Some("persistence-failure".to_owned()),
            reset: false,
        })
        .unwrap();

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert_eq!(health.status, "degraded");
    assert!(health.integrity_ok);
    assert!(health
        .persistence_error
        .as_deref()
        .is_some_and(|message| message.contains("persistence write failed")));

    let _ = std::fs::remove_dir(&state_path);
}

#[test]
fn relative_json_paths_write_state_and_backup_files() {
    let suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos()
    );
    let state_path = format!("tarrowyn-relative-state-{suffix}.json");
    let backup_path = format!("tarrowyn-relative-backup-{suffix}.json");
    let _ = std::fs::remove_file(&state_path);
    let _ = std::fs::remove_file(&backup_path);
    let repository = WorldRepository::new(ServerConfig {
        persistence_path: Some(state_path.clone()),
        backup_path: Some(backup_path.clone()),
        backup_interval_ticks: 1,
        ..ServerConfig::default()
    });

    repository
        .guest_session(GuestSessionRequest {
            client_key: Some(format!("relative-path-{suffix}")),
            reset: false,
        })
        .expect("relative state path should persist");
    repository.tick();

    assert!(std::path::Path::new(&state_path).is_file());
    assert!(std::path::Path::new(&backup_path).is_file());
    let _ = std::fs::remove_file(state_path);
    let _ = std::fs::remove_file(backup_path);
}

#[test]
fn replay_caches_are_trimmed_on_the_world_tick() {
    let backup_path = std::env::temp_dir().join(format!(
        "tarrowyn-replay-cache-backup-{}.json",
        std::process::id()
    ));
    let repository = WorldRepository::new(ServerConfig {
        backup_path: Some(backup_path.to_string_lossy().into_owned()),
        backup_interval_ticks: 1,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("replay-cache-limit".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    {
        let mut state = repository.state.lock().unwrap();
        let identity = state.identities.get_mut("replay-cache-limit").unwrap();
        for index in 0..513 {
            let request_id = format!("replay-{index}");
            identity.movement_results.insert(
                request_id.clone(),
                MovementResponse {
                    request_id: request_id.clone(),
                    accepted: false,
                    position: Position { x: 8, y: 6 },
                    reason: None,
                },
            );
            identity.chat_results.insert(
                request_id.clone(),
                ChatResponse {
                    request_id,
                    accepted: false,
                    message: None,
                    reason: None,
                },
            );
        }
        for index in 0..513 {
            state.phase6.request_results.insert(
                format!("repair-{index}"),
                SupportRepairResponse {
                    request_id: format!("repair-{index}"),
                    audit_id: format!("audit-{index}"),
                    accepted: false,
                    summary: String::new(),
                    reason: None,
                },
            );
            state
                .phase6
                .auth_revoke_guest_tokens
                .insert(index.to_string(), "replay-cache-limit".to_owned());
        }
    }

    repository.tick();

    let state = repository.state.lock().unwrap();
    let live_cursor = state.cursor;
    let identity = state.identities.get("replay-cache-limit").unwrap();
    assert!(identity.movement_results.len() <= 512);
    assert!(identity.chat_results.len() <= 512);
    assert!(state.phase6.request_results.len() <= 512);
    drop(state);
    let backup: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&backup_path).unwrap()).unwrap();
    assert_eq!(backup["cursor"].as_u64(), Some(live_cursor));
    assert!(
        backup["identities"]["replay-cache-limit"]["movement_results"]
            .as_object()
            .unwrap()
            .len()
            <= 512
    );
    assert!(
        backup["phase6"]["request_results"]
            .as_object()
            .unwrap()
            .len()
            <= 512
    );
    assert!(
        backup["phase6"]["auth_revoke_guest_tokens"]
            .as_object()
            .unwrap()
            .len()
            <= 512
    );

    let mut oversized = backup;
    let movement_results = oversized["identities"]["replay-cache-limit"]["movement_results"]
        .as_object_mut()
        .unwrap();
    for index in 0..513 {
        movement_results.insert(
            format!("loaded-{index}"),
            serde_json::json!({
                "request_id": format!("loaded-{index}"),
                "accepted": false,
                "position": { "x": 8, "y": 6 },
                "reason": null,
            }),
        );
    }
    let request_results = oversized["phase6"]["request_results"]
        .as_object_mut()
        .unwrap();
    for index in 0..513 {
        request_results.insert(
            format!("loaded-repair-{index}"),
            serde_json::json!({
                "request_id": format!("loaded-repair-{index}"),
                "audit_id": format!("loaded-audit-{index}"),
                "accepted": false,
                "summary": "",
                "reason": null,
            }),
        );
    }
    let revoke_guest_tokens = oversized["phase6"]["auth_revoke_guest_tokens"]
        .as_object_mut()
        .unwrap();
    for index in 0..513 {
        revoke_guest_tokens.insert(index.to_string(), serde_json::json!("replay-cache-limit"));
    }
    std::fs::write(&backup_path, serde_json::to_vec_pretty(&oversized).unwrap()).unwrap();
    let loaded = WorldRepository::new(ServerConfig {
        persistence_path: Some(backup_path.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    });
    let loaded_state = loaded.state.lock().unwrap();
    let loaded_identity = loaded_state.identities.get("replay-cache-limit").unwrap();
    assert!(loaded_identity.movement_results.len() <= 512);
    assert!(loaded_state.phase6.request_results.len() <= 512);
    assert!(loaded_state.phase6.auth_revoke_guest_tokens.len() <= 512);
    drop(loaded_state);
    drop(loaded);
    let _ = session;
    let _ = std::fs::remove_file(backup_path);
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
