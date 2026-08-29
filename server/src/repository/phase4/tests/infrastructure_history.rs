use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::InfrastructureStatus;

#[test]
fn failed_infrastructure_does_not_repeat_the_same_failure_chronicle() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().unwrap();
        state.tick = 11;
        state.phase4.governance.public_treasury = 0;
        let road = state
            .phase4
            .infrastructure
            .first_mut()
            .expect("the infrastructure ledger should have a first record");
        road.condition = 25;
        road.status = InfrastructureStatus::NeedsRepair;
    }

    repository.tick();
    for _ in 0..12 {
        repository.tick();
    }

    let state = repository.state.lock().unwrap();
    let failure_records = state
        .phase3
        .chronicle
        .iter()
        .filter(|entry| entry.kind == "infrastructure failure")
        .count();
    assert_eq!(failure_records, 1);
}
