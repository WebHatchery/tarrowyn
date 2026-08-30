use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    AuthLinkRequest, AuthRefreshRequest, AuthRevokeRequest, GuestSessionRequest,
};

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

#[test]
fn production_sessions_use_unpredictable_credentials() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("session-credentials-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "session-credentials-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "session-credentials-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("linked session")
        .data;

    assert!(linked.session.account_token.starts_with("prod-session-"));
    assert!(linked.session.refresh_token.starts_with("prod-refresh-"));
    assert_eq!(
        linked.session.account_token.len(),
        "prod-session-".len() + 64
    );
    assert_eq!(
        linked.session.refresh_token.len(),
        "prod-refresh-".len() + 64
    );
    assert_ne!(linked.session.account_token, "prod-session-1");
    assert_ne!(linked.session.refresh_token, "prod-refresh-1");
    assert_ne!(linked.session.account_token, linked.session.refresh_token);

    let refreshed = repository
        .auth_refresh(AuthRefreshRequest {
            request_id: "session-credentials-refresh".to_owned(),
            refresh_token: linked.session.refresh_token,
        })
        .expect("refreshed session")
        .data
        .session;
    assert!(refreshed.account_token.starts_with("prod-session-"));
    assert!(refreshed.refresh_token.starts_with("prod-refresh-"));
    assert_ne!(refreshed.account_token, linked.session.account_token);
    assert_ne!(refreshed.refresh_token, "prod-refresh-1");
}

#[test]
fn revocation_removes_refresh_replay_credentials() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("revoke-refresh-replay".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "revoke-refresh-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "revoke-refresh-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("linked session")
        .data;
    let refresh_request = AuthRefreshRequest {
        request_id: "revoke-refresh-request".to_owned(),
        refresh_token: linked.session.refresh_token,
    };
    let refreshed = repository
        .auth_refresh(refresh_request.clone())
        .expect("refreshed session")
        .data;

    repository
        .auth_revoke(
            &refreshed.session.account_token,
            AuthRevokeRequest {
                request_id: "revoke-refresh".to_owned(),
                revoke_all: true,
            },
        )
        .expect("revoked sessions");

    let error = repository.auth_refresh(refresh_request).unwrap_err();
    assert_eq!(error.status, 401);
    assert_eq!(error.error.code, "invalid_refresh");
    let state = repository.state.lock().expect("repository lock");
    assert!(state.phase6.auth_refresh_results.is_empty());
    assert!(state.phase6.auth_refresh_accounts.is_empty());
}

#[test]
fn revocation_counts_only_sessions_it_revokes_after_refresh_rotation() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("revoke-count-after-refresh".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "revoke-count-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "revoke-count-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("linked session")
        .data;
    let refreshed = repository
        .auth_refresh(AuthRefreshRequest {
            request_id: "revoke-count-refresh".to_owned(),
            refresh_token: linked.session.refresh_token,
        })
        .expect("refreshed session")
        .data
        .session;

    let revoked = repository
        .auth_revoke(
            &refreshed.account_token,
            AuthRevokeRequest {
                request_id: "revoke-count-revoke".to_owned(),
                revoke_all: true,
            },
        )
        .expect("revoke session")
        .data;

    assert_eq!(revoked.revoked_sessions, 1);
}

#[test]
fn malformed_production_credentials_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("session-credential-shape-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "session-credential-shape-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "session-credential-shape-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("linked session")
        .data;

    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase6
            .sessions
            .get_mut(&linked.session.account_token)
            .expect("production session")
            .refresh_token = "prod-refresh-not-hex".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.integrity_ok);
    assert!(!health.ready);
}

#[test]
fn malformed_cached_production_credentials_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("cached-session-credential-shape-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "cached-session-credential-shape-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "cached-session-credential-shape-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("linked session")
        .data;
    repository
        .auth_refresh(AuthRefreshRequest {
            request_id: "cached-session-credential-shape-refresh".to_owned(),
            refresh_token: linked.session.refresh_token,
        })
        .expect("refreshed session");

    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase6
            .auth_refresh_results
            .values_mut()
            .next()
            .expect("refresh replay")
            .session
            .account_token = "prod-session-not-hex".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.integrity_ok);
    assert!(!health.ready);
}
