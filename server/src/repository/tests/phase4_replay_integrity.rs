use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{ClaimLifecycleAction, ClaimLifecycleRequest, GuestSessionRequest};

fn seeded_claim_cache(repository: &WorldRepository) -> (String, String) {
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-replay-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "phase4-replay-request".to_owned(),
                action: ClaimLifecycleAction::Inspect,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim inspection");
    (session.account_id, "phase4-replay-request".to_owned())
}

#[test]
fn orphaned_phase4_replay_key_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let (account_id, request_id) = seeded_claim_cache(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let response = state
            .phase4
            .request_results
            .remove(&format!("phase4:{account_id}:{request_id}"))
            .expect("seeded replay result");
        state
            .phase4
            .request_results
            .insert(format!("phase4:missing-account:{request_id}"), response);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn phase4_replay_response_must_match_its_request_key() {
    let repository = WorldRepository::new(ServerConfig::default());
    let (account_id, request_id) = seeded_claim_cache(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let key = format!("phase4:{account_id}:{request_id}");
        let response = state
            .phase4
            .request_results
            .get_mut(&key)
            .expect("seeded replay result");
        if let super::super::phase4::Phase4Response::Claim(response) = response {
            response.request_id = "different-request".to_owned();
        }
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn phase4_replay_cleanup_requires_complete_account_and_request_boundaries() {
    let repository = WorldRepository::new(ServerConfig::default());
    let (account_id, request_id) = seeded_claim_cache(&repository);
    let response = {
        let state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .request_results
            .get(&format!("phase4:{account_id}:{request_id}"))
            .cloned()
            .expect("seeded replay result")
    };

    assert!(super::super::phase4::is_request_cache_for_account(
        &format!("phase4:{account_id}:{request_id}"),
        &account_id,
        &response,
    ));
    assert!(!super::super::phase4::is_request_cache_for_account(
        &format!("phase4:{account_id}:observer:{request_id}"),
        &account_id,
        &response,
    ));
    assert!(super::super::phase4::is_request_cache_for_account(
        &format!("phase4:{account_id}:observer:{request_id}"),
        &format!("{account_id}:observer"),
        &response,
    ));
}
