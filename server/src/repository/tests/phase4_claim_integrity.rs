use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{ClaimLifecycleAction, ClaimLifecycleRequest, GuestSessionRequest};

fn claim(repository: &WorldRepository) -> tarrowyn_protocol::ClaimRecord {
    let player = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-claim-metadata".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let requested = repository
        .claim_lifecycle(
            &player.account_token,
            ClaimLifecycleRequest {
                request_id: "phase4-claim-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request")
        .data
        .claim
        .expect("requested claim");
    repository
        .claim_lifecycle(
            &player.account_token,
            ClaimLifecycleRequest {
                request_id: "phase4-claim-approve".to_owned(),
                action: ClaimLifecycleAction::Approve,
                claim_id: Some(requested.claim_id.clone()),
                target_account_id: None,
            },
        )
        .expect("claim approval")
        .data
        .claim
        .expect("approved claim")
}

#[test]
fn malformed_phase4_claim_note_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    claim(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .claims
            .last_mut()
            .expect("claim")
            .inspection_note = "note\nwith-control".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn missing_phase4_claim_owner_name_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    claim(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.claims.last_mut().expect("claim").owner_name = None;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn reversed_phase4_claim_lease_time_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    claim(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let claim = state.phase4.claims.last_mut().expect("claim");
        claim.expires_at_unix_seconds = claim.started_at_unix_seconds;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
