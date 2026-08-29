use super::super::WorldRepository;
use crate::ServerConfig;
use std::time::Duration;
use tarrowyn_protocol::{AuthLinkRequest, AuthRefreshRequest, GuestSessionRequest};

#[test]
fn expired_access_keeps_refresh_valid_and_cleans_rotated_sessions() {
    let repository = WorldRepository::new(ServerConfig {
        tick_interval: Duration::from_millis(1),
        production_session_ttl_seconds: 1,
        refresh_ttl_seconds: 3,
        ..ServerConfig::default()
    });
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("session-retention-guest".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "session-retention-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "session-retention-subject".to_owned(),
                display_name: None,
            },
        )
        .unwrap()
        .data;
    let old_access = linked.session.account_token.clone();
    let old_refresh = linked.session.refresh_token.clone();
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.tick = linked.session.expires_at_tick;
    }

    assert!(repository.account(&old_access).is_err());
    let refreshed = repository
        .auth_refresh(AuthRefreshRequest {
            request_id: "session-retention-refresh".to_owned(),
            refresh_token: old_refresh,
        })
        .unwrap()
        .data;
    assert!(repository.account(&refreshed.session.account_token).is_ok());

    repository.tick();

    let state = repository.state.lock().expect("repository lock");
    assert_eq!(state.phase6.sessions.len(), 1);
    assert!(state
        .phase6
        .sessions
        .contains_key(&refreshed.session.account_token));
}
