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
