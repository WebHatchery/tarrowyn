use super::super::{ServerConfig, WorldRepository};

#[test]
fn malformed_phase4_infrastructure_note_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .infrastructure
            .first_mut()
            .expect("infrastructure")
            .failure_note = Some("note\nwith-control".to_owned());
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn zero_phase4_infrastructure_upkeep_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .infrastructure
            .first_mut()
            .expect("infrastructure")
            .upkeep_per_day = 0;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
