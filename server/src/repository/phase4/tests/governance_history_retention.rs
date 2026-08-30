use super::super::super::{ServerConfig, WorldRepository};
use super::{governance_request, guest};
use tarrowyn_protocol::{
    GovernanceAction, InfrastructureKind, InfrastructureRecord, InfrastructureStatus, Position,
    PublicAction, TaxCollection,
};

fn complete_public_action(
    repository: &WorldRepository,
    token: &str,
    action: PublicAction,
    prefix: &str,
) {
    let mut propose = governance_request(GovernanceAction::Propose, &format!("{prefix}-propose"));
    propose.public_action = Some(action);
    let proposal_id = repository
        .governance(token, propose)
        .expect("proposal")
        .data
        .governance
        .proposals
        .last()
        .expect("created proposal")
        .proposal_id
        .clone();

    let mut approve = governance_request(GovernanceAction::Approve, &format!("{prefix}-approve"));
    approve.proposal_id = Some(proposal_id.clone());
    assert!(repository.governance(token, approve).unwrap().data.accepted);

    let mut complete =
        governance_request(GovernanceAction::Complete, &format!("{prefix}-complete"));
    complete.proposal_id = Some(proposal_id);
    assert!(
        repository
            .governance(token, complete)
            .unwrap()
            .data
            .accepted
    );
}

fn infrastructure(index: usize) -> InfrastructureRecord {
    InfrastructureRecord {
        infrastructure_id: format!("fixture-infrastructure-{index}"),
        name: "Fixture road".to_owned(),
        kind: InfrastructureKind::Road,
        position: Position {
            x: index as i32,
            y: 0,
        },
        condition: 100,
        upkeep_per_day: 1,
        service_quality: 80,
        status: InfrastructureStatus::Operational,
        last_maintained_tick: index as u64,
        failure_note: None,
    }
}

#[test]
fn public_work_cannot_create_a_duplicate_infrastructure_record() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-public-work-once");
    let mut office = governance_request(GovernanceAction::ClaimOffice, "public-work-office");
    office.office_id = Some("steward".to_owned());
    assert!(
        repository
            .governance(&session.account_token, office)
            .unwrap()
            .data
            .accepted
    );

    complete_public_action(
        &repository,
        &session.account_token,
        PublicAction::CommissionPublicWork,
        "first-work",
    );
    let mut propose = governance_request(GovernanceAction::Propose, "second-work-propose");
    propose.public_action = Some(PublicAction::CommissionPublicWork);
    let proposal_id = repository
        .governance(&session.account_token, propose)
        .unwrap()
        .data
        .governance
        .proposals
        .last()
        .unwrap()
        .proposal_id
        .clone();
    let mut approve = governance_request(GovernanceAction::Approve, "second-work-approve");
    approve.proposal_id = Some(proposal_id.clone());
    assert!(
        repository
            .governance(&session.account_token, approve)
            .unwrap()
            .data
            .accepted
    );
    let mut complete = governance_request(GovernanceAction::Complete, "second-work-complete");
    complete.proposal_id = Some(proposal_id);
    let rejected = repository
        .governance(&session.account_token, complete)
        .unwrap()
        .data;

    assert!(!rejected.accepted);
    assert!(rejected
        .reason
        .unwrap()
        .contains("already been commissioned"));
    assert_eq!(rejected.governance.public_treasury, 36);
    assert_eq!(
        rejected
            .governance
            .proposals
            .iter()
            .filter(
                |proposal| proposal.action == PublicAction::CommissionPublicWork
                    && proposal.status == tarrowyn_protocol::ProposalStatus::Completed
            )
            .count(),
        1
    );
    assert_eq!(
        repository
            .infrastructure(&session.account_token)
            .unwrap()
            .data
            .records
            .iter()
            .filter(|record| record.infrastructure_id == "hearth-workshop")
            .count(),
        1
    );
}

fn tax_collection(index: usize) -> TaxCollection {
    TaxCollection {
        collection_id: format!("tax-{index}"),
        payer_account_id: "payer".to_owned(),
        payer_name: "Resident".to_owned(),
        amount: 1,
        rate_percent: 1,
        territory: "the Hearth".to_owned(),
        day: index as u32,
        created_tick: index as u64,
    }
}

#[test]
fn governance_recent_decisions_and_public_work_keep_newest_records() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-history-retention");
    let mut claim = governance_request(GovernanceAction::ClaimOffice, "history-office");
    claim.office_id = Some("steward".to_owned());
    assert!(
        repository
            .governance(&session.account_token, claim)
            .unwrap()
            .data
            .accepted
    );

    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.next_decision_id = 64;
        state.phase4.governance.decisions = (0..64)
            .map(|index| tarrowyn_protocol::GovernanceDecision {
                decision_id: format!("decision-{index}"),
                actor_account_id: "account".to_owned(),
                actor_name: "Resident".to_owned(),
                action: PublicAction::RepairRoad,
                proposal_id: format!("proposal-{index}"),
                cost: 1,
                service_affected: "the north road".to_owned(),
                created_tick: index as u64,
            })
            .collect();
    }

    complete_public_action(
        &repository,
        &session.account_token,
        PublicAction::RepairRoad,
        "decision",
    );
    let governance = repository
        .governance(
            &session.account_token,
            governance_request(GovernanceAction::Inspect, "decision-inspect"),
        )
        .unwrap()
        .data
        .governance;
    assert_eq!(governance.decisions.len(), 64);
    assert!(!governance
        .decisions
        .iter()
        .any(|decision| decision.decision_id == "decision-0"));
    assert!(governance
        .decisions
        .iter()
        .any(|decision| decision.decision_id == "decision-64"));

    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.infrastructure = (0..32).map(infrastructure).collect();
    }
    complete_public_action(
        &repository,
        &session.account_token,
        PublicAction::CommissionPublicWork,
        "work",
    );
    let records = repository
        .infrastructure(&session.account_token)
        .unwrap()
        .data
        .records;
    assert_eq!(records.len(), 32);
    assert!(!records
        .iter()
        .any(|record| record.infrastructure_id == "fixture-infrastructure-0"));
    assert!(records
        .iter()
        .any(|record| record.infrastructure_id == "hearth-workshop"));
}

#[test]
fn recent_tax_ledger_keeps_the_newest_receipt() {
    let repository = WorldRepository::new(ServerConfig {
        starting_gold: 100,
        day_length_seconds: 1.0,
        world_seconds_per_tick: 1.0,
        ..ServerConfig::default()
    });
    let session = guest(&repository, "phase4-tax-retention");
    let mut claim = governance_request(GovernanceAction::ClaimOffice, "tax-retention-office");
    claim.office_id = Some("steward".to_owned());
    assert!(
        repository
            .governance(&session.account_token, claim)
            .unwrap()
            .data
            .accepted
    );
    let mut set_tax = governance_request(GovernanceAction::SetTaxRate, "tax-retention-rate");
    set_tax.tax_rate_percent = Some(1);
    assert!(
        repository
            .governance(&session.account_token, set_tax)
            .unwrap()
            .data
            .accepted
    );

    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.next_tax_id = 64;
        state.phase4.governance.tax_ledger = (0..64).map(tax_collection).collect();
    }
    repository.tick();

    let governance = repository
        .governance(
            &session.account_token,
            governance_request(GovernanceAction::Inspect, "tax-retention-inspect"),
        )
        .unwrap()
        .data
        .governance;
    assert_eq!(governance.tax_ledger.len(), 64);
    assert!(!governance
        .tax_ledger
        .iter()
        .any(|receipt| receipt.collection_id == "tax-0"));
    assert!(governance
        .tax_ledger
        .iter()
        .any(|receipt| receipt.collection_id == "tax-64"));
}

#[test]
fn proposal_id_stays_at_the_numeric_ceiling() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "proposal-id-ceiling");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.next_proposal_id = u64::MAX;
    }

    let mut request = governance_request(GovernanceAction::Propose, "proposal-id-ceiling-request");
    request.public_action = Some(PublicAction::RepairRoad);
    let response = repository
        .governance(&session.account_token, request)
        .expect("proposal response")
        .data;

    assert!(response.accepted);
    assert_eq!(
        response
            .governance
            .proposals
            .last()
            .expect("created proposal")
            .proposal_id,
        format!("public-work-{}", u64::MAX)
    );
    let state = repository.state.lock().expect("repository lock");
    assert_eq!(state.phase4.next_proposal_id, u64::MAX);
}

#[test]
fn decision_id_stays_at_the_numeric_ceiling() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "decision-id-ceiling");
    let mut office =
        governance_request(GovernanceAction::ClaimOffice, "decision-id-ceiling-office");
    office.office_id = Some("steward".to_owned());
    assert!(
        repository
            .governance(&session.account_token, office)
            .expect("office response")
            .data
            .accepted
    );
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.next_decision_id = u64::MAX;
    }

    complete_public_action(
        &repository,
        &session.account_token,
        PublicAction::RepairRoad,
        "decision-id-ceiling",
    );

    let state = repository.state.lock().expect("repository lock");
    assert_eq!(state.phase4.next_decision_id, u64::MAX);
    assert!(state
        .phase4
        .governance
        .decisions
        .iter()
        .any(|decision| decision.decision_id == format!("decision-{}", u64::MAX)));
}

#[test]
fn tax_collection_id_stays_at_the_numeric_ceiling() {
    let repository = WorldRepository::new(ServerConfig {
        starting_gold: 100,
        day_length_seconds: 1.0,
        world_seconds_per_tick: 1.0,
        ..ServerConfig::default()
    });
    let session = guest(&repository, "tax-id-ceiling");
    let mut office = governance_request(GovernanceAction::ClaimOffice, "tax-id-ceiling-office");
    office.office_id = Some("steward".to_owned());
    assert!(
        repository
            .governance(&session.account_token, office)
            .expect("office response")
            .data
            .accepted
    );
    let mut set_tax = governance_request(GovernanceAction::SetTaxRate, "tax-id-ceiling-rate");
    set_tax.tax_rate_percent = Some(1);
    assert!(
        repository
            .governance(&session.account_token, set_tax)
            .expect("tax response")
            .data
            .accepted
    );
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.next_tax_id = u64::MAX;
    }

    repository.tick();

    let state = repository.state.lock().expect("repository lock");
    assert_eq!(state.phase4.next_tax_id, u64::MAX);
    assert!(state
        .phase4
        .governance
        .tax_ledger
        .iter()
        .any(|receipt| receipt.collection_id == format!("tax-{}", u64::MAX)));
}
