use super::*;

#[test]
fn out_of_range_phase4_combat_health_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let identity_key = seeded_phase4_combat(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .combat
            .get_mut(&identity_key)
            .expect("combat")
            .enemy_health = 4;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn inconsistent_phase4_combat_status_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let identity_key = seeded_phase4_combat(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let combat = state.phase4.combat.get_mut(&identity_key).expect("combat");
        combat.status = LocalCombatStatus::Victorious;
        combat.enemy_health = 1;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn out_of_bounds_phase4_animal_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.animals.first_mut().expect("animal").position.x = 99;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_phase4_animal_care_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        let future_day = state.clock.day.saturating_add(1);
        state
            .phase4
            .animals
            .first_mut()
            .expect("animal")
            .last_cared_day = future_day;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
