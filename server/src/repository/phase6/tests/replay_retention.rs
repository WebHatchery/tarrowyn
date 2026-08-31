use super::super::super::{ServerConfig, WorldRepository};
use std::time::Duration;
use tarrowyn_protocol::{AuthLinkRequest, AuthRefreshRequest, GuestSessionRequest};

#[test]
fn expired_auth_replays_forget_their_credential_payloads_on_tick() {
    let repository = WorldRepository::new(ServerConfig {
        backup_interval_ticks: 0,
        backup_path: None,
        production_session_ttl_seconds: 1,
        refresh_ttl_seconds: 2,
        tick_interval: Duration::from_secs(1),
        ..ServerConfig::default()
    });
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("replay-retention".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "replay-retention-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "replay-retention-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("linked session")
        .data;
    repository
        .auth_refresh(AuthRefreshRequest {
            request_id: "replay-retention-refresh".to_owned(),
            refresh_token: linked.session.refresh_token,
        })
        .expect("refresh response");

    repository.tick();
    {
        let state = repository.state.lock().expect("repository lock");
        assert!(state.phase6.auth_link_results.is_empty());
        assert!(state.phase6.auth_link_tokens.is_empty());
        assert_eq!(state.phase6.auth_refresh_results.len(), 1);
    }

    repository.tick();
    let state = repository.state.lock().expect("repository lock");
    assert!(state.phase6.auth_refresh_results.is_empty());
    assert!(state.phase6.auth_refresh_accounts.is_empty());
}
