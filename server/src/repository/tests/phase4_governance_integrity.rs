use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{GovernanceAction, GovernanceRequest, GuestSessionRequest, ProposalStatus};

fn session(repository: &WorldRepository, key: &str) -> tarrowyn_protocol::GuestSessionResponse {
    repository
        .guest_session(GuestSessionRequest {
            client_key: Some(key.to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data
}

fn proposal_request(request_id: &str) -> GovernanceRequest {
    GovernanceRequest {
        request_id: request_id.to_owned(),
        action: GovernanceAction::Propose,
        office_id: None,
        proposal_id: None,
        public_action: Some(tarrowyn_protocol::PublicAction::RepairRoad),
        target: None,
        cost: None,
        tax_rate_percent: None,
    }
}

#[test]
fn malformed_phase4_office_text_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.governance.offices[0].authority = "authority\nwith-control".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn malformed_phase4_tax_policy_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .governance
            .taxation
            .as_mut()
            .expect("tax policy")
            .accounting_note = "note\rwith-control".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn incomplete_phase4_completed_proposal_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let player = session(&repository, "phase4-proposal-state");
    repository
        .governance(
            &player.account_token,
            proposal_request("phase4-proposal-request"),
        )
        .expect("proposal");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let proposal = state
            .phase4
            .governance
            .proposals
            .last_mut()
            .expect("proposal");
        proposal.status = ProposalStatus::Completed;
        proposal.completed_tick = None;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
