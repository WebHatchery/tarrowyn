use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{AuthLinkRequest, GuestSessionRequest};

#[test]
fn production_replay_caches_reject_an_overflowing_valid_map() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("production-replay-overflow".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "production-replay-overflow-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "production-replay-overflow-subject".to_owned(),
                display_name: Some("Replay overflow resident".to_owned()),
            },
        )
        .expect("account link")
        .data;
    let identity_key = {
        let state = repository.state.lock().expect("repository lock");
        state
            .phase6
            .accounts
            .get(&linked.account_id)
            .expect("production account")
            .identity_key
            .clone()
    };

    {
        let mut state = repository.state.lock().expect("repository lock");
        for index in 0..513 {
            let request_id = format!("production-replay-overflow-{index}");
            let mut response = linked.clone();
            response.request_id = request_id.clone();
            state
                .phase6
                .auth_link_results
                .insert(format!("{identity_key}:{request_id}"), response);
        }
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn auth_link_replay_cleanup_does_not_confuse_identity_prefixes() {
    let repository = WorldRepository::new(ServerConfig::default());
    let owner = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("replay-prefix".to_owned()),
            reset: false,
        })
        .expect("owner guest session")
        .data;
    let observer = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("replay-prefix:observer".to_owned()),
            reset: false,
        })
        .expect("observer guest session")
        .data;
    let owner_linked = repository
        .auth_link(
            &owner.account_token,
            AuthLinkRequest {
                request_id: "replay-prefix-owner-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "replay-prefix-owner-subject".to_owned(),
                display_name: Some("Prefix owner".to_owned()),
            },
        )
        .expect("owner account link")
        .data;
    repository
        .auth_link(
            &observer.account_token,
            AuthLinkRequest {
                request_id: "replay-prefix-observer-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "replay-prefix-observer-subject".to_owned(),
                display_name: Some("Prefix observer".to_owned()),
            },
        )
        .expect("observer account link");

    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase6
            .auth_link_results
            .remove(&format!("replay-prefix:{}", "replay-prefix-owner-link"));
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);

    repository.tick();

    let state = repository.state.lock().expect("repository lock");
    assert!(!state
        .phase6
        .auth_link_tokens
        .contains_key(&owner.account_token));
    assert!(state
        .phase6
        .auth_link_results
        .keys()
        .any(|key| key == "replay-prefix:observer:replay-prefix-observer-link"));
    assert!(owner_linked.linked_guest);
}
