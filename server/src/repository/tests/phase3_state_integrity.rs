use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{ContractAction, ContractRequest, GuestSessionRequest};

#[test]
fn invalid_phase3_sequence_metadata_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase3.next_event_id = 0;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn orphaned_phase3_contract_progress_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase3-state-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .contract(
            &session.account_token,
            ContractRequest {
                request_id: "phase3-state-contract".to_owned(),
                action: ContractAction::Accept,
                contract_id: "brambleback-watch".to_owned(),
            },
        )
        .expect("contract action");

    {
        let mut state = repository.state.lock().expect("repository lock");
        let progress = state
            .phase3
            .contracts
            .remove(&session.client_key)
            .expect("contract progress");
        state
            .phase3
            .contracts
            .insert("missing-identity".to_owned(), progress);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
