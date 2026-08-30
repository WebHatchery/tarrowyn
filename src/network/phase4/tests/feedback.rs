use super::*;

#[test]
fn claim_success_message_explains_status_and_recovery_path() {
    let active = claim_for_test(
        "active-lease",
        Some("account-1"),
        tarrowyn_protocol::ClaimLifecycleStatus::Active,
    );
    assert_eq!(
        super::super::claim_success_message(Some(&active)),
        "Lease active at plot (1, 1); building access is open for 90 days."
    );

    let abandoned = claim_for_test(
        "abandoned-lease",
        Some("account-1"),
        tarrowyn_protocol::ClaimLifecycleStatus::Abandoned,
    );
    assert_eq!(
        super::super::claim_success_message(Some(&abandoned)),
        "Lease abandoned at plot (1, 1); use the Registry control to reclaim it after the grace period."
    );
}

#[test]
fn profession_success_message_explains_order_result() {
    let mut order = service_order_for_test(tarrowyn_protocol::ServiceOrderStatus::Completed);
    order.quality = 87;
    order.reward_gold = 12;
    order.benefit = "The field tool is restored.".to_owned();
    assert_eq!(
        super::super::profession_success_message(Some(&order), None),
        "Service order completed: Repair a field tool at 87% quality; 12 gold paid. The field tool is restored."
    );

    order.status = tarrowyn_protocol::ServiceOrderStatus::Accepted;
    order.provider_name = Some("Mara".to_owned());
    assert_eq!(
        super::super::profession_success_message(Some(&order), None),
        "Service order accepted: Repair a field tool; Mara is responsible for the 12 gold reward."
    );
}

#[test]
fn profession_success_message_names_a_learned_capability() {
    let request = tarrowyn_protocol::ProfessionRequest {
        request_id: "learn-capability".to_owned(),
        action: tarrowyn_protocol::ProfessionAction::LearnCapability,
        order_id: None,
        profession: Some(tarrowyn_protocol::ProfessionKind::Carpenter),
        capability_id: None,
        service: None,
        timing_score: None,
    };

    assert_eq!(
        super::super::profession_success_message(None, Some(&request)),
        "Carpenter capability recorded; its credential is now in the profession ledger."
    );
}

#[test]
fn governance_success_message_explains_completed_public_action() {
    let response = tarrowyn_protocol::GovernanceResponse {
        request_id: "governance-complete".to_owned(),
        accepted: true,
        governance: tarrowyn_protocol::GovernanceState {
            settlement_id: "hearth".to_owned(),
            offices: Vec::new(),
            proposals: vec![tarrowyn_protocol::PublicProposal {
                proposal_id: "public-work-1".to_owned(),
                proposer_account_id: "account-1".to_owned(),
                proposer_name: "Resident".to_owned(),
                action: tarrowyn_protocol::PublicAction::RepairRoad,
                target: "North road safety".to_owned(),
                cost: 8,
                status: tarrowyn_protocol::ProposalStatus::Completed,
                created_tick: 1,
                approved_by: Some("account-1".to_owned()),
                completed_tick: Some(3),
            }],
            decisions: Vec::new(),
            public_treasury: 12,
            administration_quality: 50,
            service_funding_until_tick: 0,
            taxation: None,
            tax_ledger: Vec::new(),
            cursor: 3,
        },
        reason: None,
    };
    let request = tarrowyn_protocol::GovernanceRequest {
        request_id: "governance-complete".to_owned(),
        action: tarrowyn_protocol::GovernanceAction::Complete,
        office_id: None,
        proposal_id: Some("public-work-1".to_owned()),
        public_action: None,
        target: None,
        cost: None,
        tax_rate_percent: None,
    };

    assert_eq!(
        super::super::governance_success_message(&response, Some(&request)),
        "Public action completed: repair the north road for North road safety; 8 public gold spent."
    );
}

fn service_order_for_test(
    status: tarrowyn_protocol::ServiceOrderStatus,
) -> tarrowyn_protocol::ServiceOrder {
    tarrowyn_protocol::ServiceOrder {
        order_id: "service-order-test".to_owned(),
        requester_account_id: "account-1".to_owned(),
        requester_name: "Resident".to_owned(),
        provider_account_id: Some("account-2".to_owned()),
        provider_name: Some("Provider".to_owned()),
        service: "Repair a field tool".to_owned(),
        required_profession: ProfessionKind::Carpenter,
        materials: tarrowyn_protocol::MaterialStock::default(),
        tools_required: 1,
        reward_gold: 4,
        benefit: "A sound field tool".to_owned(),
        status,
        quality: 0,
        created_tick: 1,
        completed_tick: None,
    }
}
