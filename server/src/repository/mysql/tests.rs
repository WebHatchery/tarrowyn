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
