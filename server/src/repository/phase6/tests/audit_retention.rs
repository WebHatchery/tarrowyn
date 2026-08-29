use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::AuditRecord;

#[test]
fn audit_retention_keeps_the_newest_records() {
    let repository = WorldRepository::new(ServerConfig::default());
    let mut state = repository.state.lock().expect("repository lock");
    state.phase6.audits = (0..(super::super::MAX_AUDITS + 1))
        .map(|index| AuditRecord {
            audit_id: format!("audit-{index}"),
            actor_account_id: "account".to_owned(),
            action: "test".to_owned(),
            target: "retention".to_owned(),
            outcome: "accepted".to_owned(),
            tick: index as u64,
            note: "The newest audit remains available.".to_owned(),
        })
        .collect();
    super::super::trim_audits(&mut state.phase6.audits);

    assert_eq!(state.phase6.audits.len(), super::super::MAX_AUDITS);
    assert_eq!(state.phase6.audits.front().unwrap().audit_id, "audit-1");
    assert_eq!(state.phase6.audits.back().unwrap().audit_id, "audit-512");
}

#[test]
fn audit_appends_keep_the_window_bounded_before_a_tick() {
    let repository = WorldRepository::new(ServerConfig::default());
    let mut state = repository.state.lock().expect("repository lock");
    for _ in 0..=super::super::MAX_AUDITS {
        super::super::audit(
            &mut state,
            "account",
            "test",
            "retention",
            "accepted",
            "The audit window stays bounded after each append.",
        );
    }

    assert_eq!(state.phase6.audits.len(), super::super::MAX_AUDITS);
    assert_eq!(state.phase6.audits.front().unwrap().audit_id, "audit-2");
}
