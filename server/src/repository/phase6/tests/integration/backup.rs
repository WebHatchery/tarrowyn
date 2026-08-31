use super::*;

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

    assert!(write(&mut state, &config));

    let bytes = fs::read(&path).expect("backup should be written");
    let stored: StoredState = serde_json::from_slice(&bytes).expect("backup should be complete");
    assert_eq!(
        stored.storage_version,
        super::super::super::super::STORAGE_VERSION
    );
    assert_eq!(stored.phase6.last_backup_tick, Some(7));
    assert_eq!(stored.phase6.last_backup_path.as_deref(), path.to_str());
    assert_eq!(state.phase6.last_backup_tick, Some(7));
    assert_eq!(state.phase6.last_backup_path.as_deref(), path.to_str());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn failed_scheduled_backup_degrades_readiness_until_recovery() {
    let backup_path =
        std::env::temp_dir().join(format!("tarrowyn-backup-failure-{}", std::process::id()));
    fs::create_dir(&backup_path).expect("backup failure fixture should be creatable");
    let repository = WorldRepository::new(ServerConfig {
        backup_path: Some(backup_path.to_string_lossy().into_owned()),
        backup_interval_ticks: 1,
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("backup-failure-operator".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;

    repository.tick();

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert_eq!(health.status, "degraded");
    assert!(health.persistence_error.is_none());
    assert!(health
        .backup_error
        .as_deref()
        .is_some_and(|message| message.contains("scheduled backup failed")));
    assert!(health.last_backup_tick.is_none());
    let metrics = repository
        .ops_metrics(&operator.account_token)
        .unwrap()
        .data;
    assert!(metrics
        .alert_flags
        .iter()
        .any(|flag| flag == "backup_write_failed"));

    fs::remove_dir(&backup_path).expect("failed backup fixture should be removable");
    repository.tick();

    let recovered = repository.ops_health().data;
    assert!(recovered.ready);
    assert!(recovered.backup_error.is_none());
    assert_eq!(recovered.last_backup_tick, Some(2));
    let _ = fs::remove_file(backup_path);
}
