use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{AuthLinkRequest, GuestSessionRequest};

#[test]
fn production_session_refresh_window_covers_access_window() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("session-window-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "session-window-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "session-window-subject".to_owned(),
                display_name: Some("Window traveller".to_owned()),
            },
        )
        .expect("linked session")
        .data;

    {
        let mut state = repository.state.lock().expect("world repository lock");
        let session = state
            .phase6
            .sessions
            .get_mut(&linked.session.account_token)
            .expect("production session should be stored");
        session.refresh_expires_at_tick = session.expires_at_tick - 1;
    }

    let health = repository.ops_health().data;
    assert!(!health.integrity_ok);
    assert!(!health.ready);
}
