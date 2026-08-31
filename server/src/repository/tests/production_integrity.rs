use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    AccountDeletionRequest, AuthLinkRequest, AuthRefreshRequest, AuthRevokeRequest,
    GuestSessionRequest,
};

fn link(repository: &WorldRepository, client_key: &str, request_id: &str) -> (String, String) {
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some(client_key.to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let response = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: request_id.to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: format!("{client_key}-subject"),
                display_name: Some(format!("{client_key} resident")),
            },
        )
        .expect("account link")
        .data;
    (response.account_id, response.session.refresh_token)
}

#[test]
fn swapped_production_link_replay_results_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let (first_account, _) = link(&repository, "production-cache-one", "cache-link-one");
    let (second_account, _) = link(&repository, "production-cache-two", "cache-link-two");
    let (first_identity, second_identity) = {
        let state = repository.state.lock().expect("repository lock");
        (
            state
                .phase6
                .accounts
                .get(&first_account)
                .expect("first account")
                .identity_key
                .clone(),
            state
                .phase6
                .accounts
                .get(&second_account)
                .expect("second account")
                .identity_key
                .clone(),
        )
    };
    {
        let mut state = repository.state.lock().expect("repository lock");
        let first_key = format!("{first_identity}:cache-link-one");
        let second_key = format!("{second_identity}:cache-link-two");
        let first_response = state
            .phase6
            .auth_link_results
            .remove(&first_key)
            .expect("first cached link");
        let second_response = state
            .phase6
            .auth_link_results
            .remove(&second_key)
            .expect("second cached link");
        state
            .phase6
            .auth_link_results
            .insert(first_key, second_response);
        state
            .phase6
            .auth_link_results
            .insert(second_key, first_response);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn swapped_production_refresh_replay_results_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let (first_account, first_refresh) = link(&repository, "refresh-cache-one", "refresh-link-one");
    let (second_account, second_refresh) =
        link(&repository, "refresh-cache-two", "refresh-link-two");
    let first = repository
        .auth_refresh(AuthRefreshRequest {
            request_id: "shared-refresh-request".to_owned(),
            refresh_token: first_refresh,
        })
        .expect("first refresh")
        .data;
    let second = repository
        .auth_refresh(AuthRefreshRequest {
            request_id: "shared-refresh-request".to_owned(),
            refresh_token: second_refresh,
        })
        .expect("second refresh")
        .data;

    {
        let mut state = repository.state.lock().expect("repository lock");
        let entries = state
            .phase6
            .auth_refresh_results
            .iter()
            .filter(|(_, response)| {
                response.session.account_token == first.session.account_token
                    || response.session.account_token == second.session.account_token
            })
            .map(|(key, response)| (key.clone(), response.clone()))
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 2);
        let (first_key, first_response) = entries
            .iter()
            .find(|(_, response)| response.session.account_token == first.session.account_token)
            .cloned()
            .expect("first refresh cache");
        let (second_key, second_response) = entries
            .iter()
            .find(|(_, response)| response.session.account_token == second.session.account_token)
            .cloned()
            .expect("second refresh cache");
        state
            .phase6
            .auth_refresh_results
            .insert(first_key, second_response);
        state
            .phase6
            .auth_refresh_results
            .insert(second_key, first_response);
        assert!(state.phase6.accounts.contains_key(&first_account));
        assert!(state.phase6.accounts.contains_key(&second_account));
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn malformed_production_revoke_replay_key_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("revoke-replay-key-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .auth_revoke(
            &guest.account_token,
            AuthRevokeRequest {
                request_id: "revoke-replay-key".to_owned(),
                revoke_all: false,
            },
        )
        .expect("revoke session");

    {
        let mut state = repository.state.lock().expect("repository lock");
        let key = state
            .phase6
            .auth_revoke_results
            .keys()
            .next()
            .cloned()
            .expect("revoke cache");
        let response = state
            .phase6
            .auth_revoke_results
            .remove(&key)
            .expect("revoke cache response");
        state
            .phase6
            .auth_revoke_results
            .insert("missing-identity:revoke-replay-key".to_owned(), response);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_production_account_link_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = super::guest(&repository, "integrity-production-account");
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "integrity-production-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "integrity-production-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("production account link");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase6
            .accounts
            .get_mut(&linked.data.account_id)
            .expect("production account")
            .identity_key = "missing-identity".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn missing_production_session_mirror_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = super::guest(&repository, "integrity-production-session");
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "integrity-production-session-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "integrity-production-session-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("production account link")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.sessions.remove(&linked.session.account_token);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn orphaned_production_refresh_replay_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = super::guest(&repository, "integrity-orphaned-refresh");
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "integrity-orphaned-refresh-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "integrity-orphaned-refresh-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("production account link")
        .data;
    let refreshed = repository
        .auth_refresh(AuthRefreshRequest {
            request_id: "integrity-orphaned-refresh-request".to_owned(),
            refresh_token: linked.session.refresh_token,
        })
        .expect("production refresh")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase6
            .sessions
            .remove(&refreshed.session.account_token);
        state.sessions.remove(&refreshed.session.account_token);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn orphaned_production_link_replay_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = super::guest(&repository, "integrity-orphaned-link");
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "integrity-orphaned-link-request".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "integrity-orphaned-link-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("production account link")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase6.sessions.remove(&linked.session.account_token);
        state.sessions.remove(&linked.session.account_token);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn live_session_cannot_remain_an_auth_link_replay_tombstone() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = super::guest(&repository, "integrity-live-link-tombstone");
    let other_guest = super::guest(&repository, "integrity-live-link-observer");
    repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "integrity-live-link-request".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "integrity-live-link-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("production account link");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let observer_session = state
            .sessions
            .get(&other_guest.account_token)
            .cloned()
            .expect("observer session");
        state
            .sessions
            .insert(guest.account_token.clone(), observer_session);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_audit_outcome_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = super::guest(&repository, "integrity-production-audit");
    repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "integrity-production-audit-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "integrity-production-audit-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("production account link");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase6.audits.back_mut().expect("link audit").outcome = "unknown".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn orphan_moderation_timestamp_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase6
            .report_created_at
            .insert("orphan-report".to_owned(), 1);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn malformed_account_deletion_queue_key_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = super::guest(&repository, "integrity-production-deletion");
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "integrity-production-deletion-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "integrity-production-deletion-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("production account link")
        .data;
    repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "integrity-production-deletion-request".to_owned(),
                account_id: linked.account_id,
            },
        )
        .expect("account deletion request");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let original_key = state
            .phase6
            .deletion_requests
            .keys()
            .next()
            .cloned()
            .expect("pending deletion");
        let pending = state
            .phase6
            .deletion_requests
            .remove(&original_key)
            .expect("pending deletion");
        state
            .phase6
            .deletion_requests
            .insert("malformed-deletion-key".to_owned(), pending);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
