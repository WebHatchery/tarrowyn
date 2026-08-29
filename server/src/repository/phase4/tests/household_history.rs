use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{HouseholdLifeStatus, InfrastructureStatus};

#[test]
fn departed_npc_does_not_repeat_the_same_departure_chronicle() {
    let repository = WorldRepository::new(ServerConfig {
        household_decision_interval_ticks: 1,
        ..ServerConfig::default()
    });
    {
        let mut state = repository.state.lock().unwrap();
        state.phase4.households[0].status = HouseholdLifeStatus::Departed;
        state
            .phase4
            .infrastructure
            .iter_mut()
            .find(|record| record.infrastructure_id == "north-road")
            .expect("the launch road should exist")
            .status = InfrastructureStatus::Failed;
    }

    repository.tick();
    repository.tick();

    let state = repository.state.lock().unwrap();
    let departure_records = state
        .phase3
        .chronicle
        .iter()
        .filter(|entry| entry.kind == "household departure")
        .count();
    assert_eq!(departure_records, 0);
}
