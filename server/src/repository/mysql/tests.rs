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
