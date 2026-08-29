use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{ClaimLifecycleAction, ClaimLifecycleRequest};
use tarrowyn_protocol::{GovernanceAction, GovernanceRequest, GuestSessionRequest, PublicAction};

fn seeded_phase4_claim(repository: &WorldRepository) {
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-claim-state".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "phase4-claim-state-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request");
}

#[test]
fn invalid_phase4_sequence_metadata_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.next_claim_id = 0;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_phase4_governance_cursor_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.governance.cursor = state.cursor.saturating_add(1);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_phase4_proposal_timestamp_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-governance-time".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .governance(
            &session.account_token,
            GovernanceRequest {
                request_id: "phase4-proposal-time".to_owned(),
                action: GovernanceAction::Propose,
                office_id: None,
                proposal_id: None,
                public_action: Some(PublicAction::RepairRoad),
                target: None,
                cost: None,
                tax_rate_percent: None,
            },
        )
        .expect("proposal");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let future_tick = state.tick.saturating_add(1);
        state
            .phase4
            .governance
            .proposals
            .last_mut()
            .expect("proposal")
            .created_tick = future_tick;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_phase4_claim_activity_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    seeded_phase4_claim(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let future_tick = state.tick.saturating_add(1);
        state
            .phase4
            .claims
            .last_mut()
            .expect("claim")
            .last_active_tick = future_tick;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn requested_phase4_claim_cannot_have_building_access() {
    let repository = WorldRepository::new(ServerConfig::default());
    seeded_phase4_claim(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .claims
            .last_mut()
            .expect("claim")
            .building_access = true;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
