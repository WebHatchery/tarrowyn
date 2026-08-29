use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{ContractAction, ContractRequest, GuestSessionRequest};

fn seeded_contract_cache(repository: &WorldRepository) -> (String, String) {
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase3-replay-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .contract(
            &session.account_token,
            ContractRequest {
                request_id: "phase3-replay-request".to_owned(),
                action: ContractAction::Accept,
                contract_id: "brambleback-watch".to_owned(),
            },
        )
        .expect("contract action");
    (session.client_key, "phase3-replay-request".to_owned())
}

#[test]
fn orphaned_phase3_replay_key_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let (identity_key, request_id) = seeded_contract_cache(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let response = state
            .phase3
            .request_results
            .remove(&format!("{identity_key}:{request_id}"))
            .expect("seeded replay result");
        state
            .phase3
            .request_results
            .insert(format!("missing-identity:{request_id}"), response);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn phase3_replay_response_must_match_its_request_key() {
    let repository = WorldRepository::new(ServerConfig::default());
    let (identity_key, request_id) = seeded_contract_cache(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let key = format!("{identity_key}:{request_id}");
        let response = state
            .phase3
            .request_results
            .get_mut(&key)
            .expect("seeded replay result");
        if let super::super::phase3::Phase3Response::Contract(response) = response {
            response.request_id = "different-request".to_owned();
        }
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
