use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::GuestSessionRequest;

#[test]
fn session_must_reference_its_live_identity() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("core-session-identity-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .sessions
            .get_mut(&session.account_token)
            .expect("session")
            .identity_key = "missing-identity".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn session_activity_cannot_point_into_the_future() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("core-session-time-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        let future_tick = state.tick.saturating_add(1);
        state
            .sessions
            .get_mut(&session.account_token)
            .expect("session")
            .last_chat_tick = Some(future_tick);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
