use super::*;

#[test]
fn newer_schema_versions_fail_closed() {
    assert_eq!(unsupported_migration_version(&[MIGRATION_VERSION]), None);
    assert_eq!(unsupported_migration_version(&[0, MIGRATION_VERSION]), None);
    assert_eq!(
        unsupported_migration_version(&[MIGRATION_VERSION + 1]),
        Some(2)
    );
}

#[test]
fn snapshot_metadata_must_match_the_json_document() {
    let stored = RepositoryState::fresh(&ServerConfig::default()).to_stored();

    assert!(snapshot_metadata_matches(
        stored.storage_version,
        stored.tick,
        stored.cursor,
        &stored,
    ));
    assert!(!snapshot_metadata_matches(
        stored.storage_version,
        stored.tick.saturating_add(1),
        stored.cursor,
        &stored,
    ));
    assert!(!snapshot_metadata_matches(
        stored.storage_version,
        stored.tick,
        stored.cursor.saturating_add(1),
        &stored,
    ));
}

#[test]
fn identity_index_must_match_snapshot_identities() {
    let repository = crate::repository::WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    repository
        .guest_session(tarrowyn_protocol::GuestSessionRequest {
            client_key: Some("mysql-index-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session");
    let state = repository.state.lock().expect("repository lock");
    let stored = state.to_stored();
    let expected = stored
        .identities
        .values()
        .map(|identity| (identity.account_id.clone(), identity.character_id.clone()))
        .collect::<Vec<_>>();

    assert!(identity_index_matches(&stored, &expected));
    assert!(!identity_index_matches(
        &stored,
        &[("stale-account".to_owned(), "stale-character".to_owned())],
    ));
}
