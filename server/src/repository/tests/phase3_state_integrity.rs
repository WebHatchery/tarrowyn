use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    ClaimAction, ClaimRequest, ContractAction, ContractRequest, GuestSessionRequest,
};

fn seeded_claim(repository: &WorldRepository) -> String {
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase3-claim-state".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .claim(
            &session.account_token,
            ClaimRequest {
                request_id: "phase3-claim-request".to_owned(),
                action: ClaimAction::Request,
            },
        )
        .expect("claim request");
    session.client_key
}

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

#[test]
fn future_phase3_claim_activity_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let identity_key = seeded_claim(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase3.claim.as_mut().expect("claim").last_active_tick = state.tick.saturating_add(1);
        assert!(state.identities.contains_key(&identity_key));
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn zero_phase3_claim_reclaim_window_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    seeded_claim(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase3
            .claim
            .as_mut()
            .expect("claim")
            .reclaim_after_ticks = 0;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
