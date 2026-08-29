use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::InfrastructureStatus;

#[test]
fn infrastructure_upkeep_cannot_wrap_into_a_funded_treasury() {
    let repository = WorldRepository::new(ServerConfig::default());
    let mut state = repository.state.lock().unwrap();
    state.tick = 12;
    state.phase4.governance.public_treasury = u32::MAX - 1;
    for record in &mut state.phase4.infrastructure {
        record.upkeep_per_day = u32::MAX;
        record.condition = 100;
        record.status = InfrastructureStatus::Operational;
    }

    super::super::governance::tick(&mut state, &ServerConfig::default());

    assert_eq!(state.phase4.governance.public_treasury, u32::MAX - 1);
    assert!(state
        .phase4
        .infrastructure
        .iter()
        .all(|record| record.condition == 95));
}
