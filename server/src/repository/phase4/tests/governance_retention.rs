use super::super::super::{ServerConfig, WorldRepository};
use super::{governance_request, guest};
use tarrowyn_protocol::{ProposalStatus, PublicAction, PublicProposal};

fn proposal(index: usize, status: ProposalStatus) -> PublicProposal {
    PublicProposal {
        proposal_id: format!("proposal-{index}"),
        proposer_account_id: "account".to_owned(),
        proposer_name: "Resident".to_owned(),
        action: PublicAction::RepairRoad,
        target: "the north road".to_owned(),
        cost: 1,
        status,
        created_tick: index as u64,
        approved_by: None,
        completed_tick: (status == ProposalStatus::Completed).then_some(index as u64),
    }
}

#[test]
fn governance_proposals_trim_closed_history_and_bound_active_work() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-proposal-retention");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.governance.proposals = (0..64)
            .map(|index| proposal(index, ProposalStatus::Completed))
            .collect();
    }

    let mut request = governance_request(
        tarrowyn_protocol::GovernanceAction::Propose,
        "proposal-after-closed-history",
    );
    request.public_action = Some(PublicAction::RepairRoad);
    let created = repository
        .governance(&session.account_token, request)
        .expect("proposal should be accepted")
        .data;
    assert!(created.accepted);
    assert_eq!(created.governance.proposals.len(), 64);
    assert!(!created
        .governance
        .proposals
        .iter()
        .any(|proposal| proposal.proposal_id == "proposal-0"));

    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.governance.proposals = (0..64)
            .map(|index| proposal(index, ProposalStatus::Proposed))
            .collect();
    }
    let mut blocked_request = governance_request(
        tarrowyn_protocol::GovernanceAction::Propose,
        "proposal-while-full",
    );
    blocked_request.public_action = Some(PublicAction::RepairRoad);
    let blocked = repository
        .governance(&session.account_token, blocked_request)
        .expect("full proposal ledger should return a readable response")
        .data;
    assert!(!blocked.accepted);
    assert!(blocked
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("ledger is full")));
    assert_eq!(blocked.governance.proposals.len(), 64);
}
