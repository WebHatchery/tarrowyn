use super::super::models::{RepositoryState, StoredState};
use super::super::ServerConfig;
use super::super::WorldRepository;
use super::backup::write;
use std::fs;
use tarrowyn_protocol::GuestSessionRequest;

#[test]
fn backup_replaces_the_snapshot_as_one_complete_json_file() {
    let root = std::env::temp_dir().join(format!(
        "tarrowyn-backup-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos()
    ));
    let path = root.join("nested").join("snapshot.json");
    let config = ServerConfig {
        backup_path: Some(path.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    };
    let mut state = RepositoryState::fresh(&config);
    state.tick = 7;

    write(&mut state, &config);

    let bytes = fs::read(&path).expect("backup should be written");
    let stored: StoredState = serde_json::from_slice(&bytes).expect("backup should be complete");
    assert_eq!(stored.storage_version, super::super::STORAGE_VERSION);
    assert_eq!(state.phase6.last_backup_tick, Some(7));
    assert_eq!(state.phase6.last_backup_path.as_deref(), path.to_str());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn operational_metrics_require_a_configured_support_operator() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("metrics-operator".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let player = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("metrics-player".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;

    let error = repository
        .ops_metrics(&player.account_token)
        .expect_err("ordinary players must not read operational metrics");
    assert_eq!(error.status, 403);
    assert_eq!(error.error.code, "support_operator_required");
    assert!(repository.ops_metrics(&operator.account_token).is_ok());
}
