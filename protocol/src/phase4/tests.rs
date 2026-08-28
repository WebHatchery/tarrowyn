use super::*;

#[test]
fn phase_four_wire_types_keep_actions_and_costs_explicit() {
    let request = GovernanceRequest {
        request_id: "town-hall-1".to_owned(),
        action: GovernanceAction::Propose,
        office_id: None,
        proposal_id: None,
        public_action: Some(PublicAction::RepairRoad),
        target: Some("North road safety".to_owned()),
        cost: None,
    };
    let encoded = serde_json::to_string(&request).unwrap();
    assert!(encoded.contains("\"action\":\"propose\""));
    assert!(encoded.contains("\"public_action\":\"repair_road\""));
    assert_eq!(PublicAction::RepairRoad.default_cost(), 8);
}

#[test]
fn phase_four_claims_and_knowledge_round_trip() {
    let response = ClaimLifecycleResponse {
        request_id: "lease-1".to_owned(),
        accepted: true,
        claim: None,
        claims: ClaimsResponse {
            claims: Vec::new(),
            available_plots: vec![Position { x: 2, y: 8 }],
            cursor: 4,
        },
        reason: None,
    };
    let knowledge = KnowledgeRequest {
        request_id: "knowledge-1".to_owned(),
        action: KnowledgeAction::Teach,
        knowledge_id: Some("moonberry-tending".to_owned()),
        target_account_id: Some("dev-account-2".to_owned()),
    };
    let encoded = serde_json::to_string(&(response, knowledge)).unwrap();
    assert!(encoded.contains("available_plots"));
    assert!(encoded.contains("\"action\":\"teach\""));
}

#[test]
fn profession_requests_keep_timing_optional_for_existing_clients() {
    let request: ProfessionRequest = serde_json::from_str(
        r#"{"request_id":"craft-1","action":"complete_order","order_id":"service-order-1"}"#,
    )
    .unwrap();
    assert_eq!(request.timing_score, None);
}
