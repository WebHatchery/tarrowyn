use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{AuthLinkRequest, GuestSessionRequest};

fn link(repository: &WorldRepository, client_key: &str, request_id: &str) -> String {
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some(client_key.to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
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
        .data
        .account_id
}

#[test]
fn swapped_production_link_replay_results_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let first_account = link(&repository, "production-cache-one", "cache-link-one");
    let second_account = link(&repository, "production-cache-two", "cache-link-two");
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
