use super::super::models::RepositoryState;
use super::super::WorldRepository;
use crate::config::ServerConfig;
use tarrowyn_protocol::{AuthLinkRequest, AuthRefreshRequest, GuestSessionRequest};

#[test]
fn legacy_refresh_replays_rebuild_owners_and_drop_orphaned_sessions() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("legacy-refresh-migration".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "legacy-refresh-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "legacy-refresh-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("linked session")
        .data;
    let second_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("legacy-refresh-migration-second".to_owned()),
            reset: false,
        })
        .expect("second guest session")
        .data;
    let second_linked = repository
        .auth_link(
            &second_guest.account_token,
            AuthLinkRequest {
                request_id: "legacy-refresh-link-second".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "legacy-refresh-subject-second".to_owned(),
                display_name: None,
            },
        )
        .expect("second linked session")
        .data;
    let _ = repository
        .auth_refresh(AuthRefreshRequest {
            request_id: "legacy-refresh-one".to_owned(),
            refresh_token: linked.session.refresh_token,
        })
        .expect("first refresh");
    let second_refresh = repository
        .auth_refresh(AuthRefreshRequest {
            request_id: "legacy-refresh-two".to_owned(),
            refresh_token: second_linked.session.refresh_token,
        })
        .expect("second account refresh")
        .data;

    let mut stored = repository
        .state
        .lock()
        .expect("repository lock")
        .to_stored();
    stored.phase6.auth_refresh_accounts.clear();
    stored
        .phase6
        .sessions
        .remove(&second_refresh.session.account_token);

    let restored = RepositoryState::from_stored(stored, &ServerConfig::default());

    assert_eq!(restored.phase6.auth_refresh_results.len(), 1);
    assert_eq!(restored.phase6.auth_refresh_accounts.len(), 1);
    assert_eq!(
        restored
            .phase6
            .auth_refresh_accounts
            .values()
            .next()
            .map(String::as_str),
        Some("account-1")
    );
}

#[test]
fn legacy_refresh_replays_drop_revoked_issuing_sessions() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("legacy-refresh-revoked".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "legacy-refresh-revoked-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "legacy-refresh-revoked-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("linked session")
        .data;
    let refreshed = repository
        .auth_refresh(AuthRefreshRequest {
            request_id: "legacy-refresh-revoked-request".to_owned(),
            refresh_token: linked.session.refresh_token,
        })
        .expect("refresh")
        .data;

    let mut stored = repository
        .state
        .lock()
        .expect("repository lock")
        .to_stored();
    stored
        .phase6
        .sessions
        .get_mut(&refreshed.session.account_token)
        .expect("issued session")
        .revoked = true;

    let restored = RepositoryState::from_stored(stored, &ServerConfig::default());

    assert!(restored.phase6.auth_refresh_results.is_empty());
    assert!(restored.phase6.auth_refresh_accounts.is_empty());
}
