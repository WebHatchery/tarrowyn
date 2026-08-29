use super::super::{ServerConfig, WorldRepository};

#[test]
fn invalid_regional_route_bounds_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.routes[0].risk_percent = 101;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_regional_settlement_bounds_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.settlements[0].governance = 101;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
