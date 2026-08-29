use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::ChronicleEntry;

#[test]
fn malformed_settlement_narrative_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.settlements[0].milestones.clear();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_settlement_chronicle_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.settlements[0].chronicle.push(ChronicleEntry {
            event_id: "settlement-history-corrupt".to_owned(),
            kind: "regional".to_owned(),
            title: "Corrupt history".to_owned(),
            text: "This entry has not been assigned a world cursor.".to_owned(),
            created_tick: 0,
            cursor: 0,
        });
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
