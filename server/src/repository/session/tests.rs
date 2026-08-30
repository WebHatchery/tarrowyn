use crate::{ServerConfig, WorldRepository};
use tarrowyn_protocol::GuestSessionRequest;

#[test]
fn authentication_evicts_a_session_with_a_missing_character() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("orphaned-session".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("state lock");
        state.identities.remove(&session.client_key);
    }

    let error = repository.world(&session.account_token).unwrap_err();
    assert_eq!(error.status, 401);
    assert_eq!(error.error.code, "unauthorized");

    let state = repository.state.lock().expect("state lock");
    assert!(!state.sessions.contains_key(&session.account_token));
}

#[test]
fn guest_session_expires_at_the_configured_tick_boundary() {
    let repository = WorldRepository::new(ServerConfig {
        session_ttl_seconds: 1,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("exact-session-expiry".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    {
        let mut state = repository.state.lock().expect("state lock");
        state.tick = repository.config.session_ttl_ticks();
    }

    let error = repository.world(&session.account_token).unwrap_err();
    assert_eq!(error.status, 401);
    assert_eq!(error.error.code, "unauthorized");
}
