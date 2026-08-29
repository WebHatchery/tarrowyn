use super::super::{ServerConfig, WorldRepository};

#[test]
fn invalid_core_sequence_metadata_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.next_notice = 0;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_identity_activity_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = super::guest(&repository, "core-identity-time");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let future_tick = state.tick.saturating_add(1);
        state
            .identities
            .get_mut(&session.client_key)
            .expect("identity")
            .last_seen_tick = future_tick;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
