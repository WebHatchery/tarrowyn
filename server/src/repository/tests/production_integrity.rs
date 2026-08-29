use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    AuthLinkRequest, AuthRefreshRequest, AuthRevokeRequest, GuestSessionRequest,
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
