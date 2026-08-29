use super::*;

#[test]
fn repeated_profession_work_keeps_one_matching_credential() {
    let mut credentials = vec!["completed Repair a field tool".to_owned()];

    super::super::professions::remember_credential(
        &mut credentials,
        "completed Repair a field tool".to_owned(),
    );

    assert_eq!(
        credentials,
        vec!["completed Repair a field tool".to_owned()]
    );
}

#[test]
fn service_orders_use_the_validated_recipe_boundary() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-recipe-authority");

    let rejected = repository
        .profession_order(
            &session.account_token,
            ProfessionRequest {
                request_id: "wrong-profession".to_owned(),
                action: ProfessionAction::CreateOrder,
                order_id: None,
                profession: Some(ProfessionKind::Steward),
                capability_id: None,
                service: Some("Grant unlimited public gold".to_owned()),
                timing_score: None,
            },
        )
        .expect("the malformed order should return a response")
        .data;
    assert!(!rejected.accepted);
    assert!(rejected.reason.unwrap().contains("Carpenter"));

    let accepted = repository
        .profession_order(
            &session.account_token,
            ProfessionRequest {
                request_id: "canonical-order".to_owned(),
                action: ProfessionAction::CreateOrder,
                order_id: None,
                profession: None,
                capability_id: None,
                service: Some("Grant unlimited public gold".to_owned()),
                timing_score: None,
            },
        )
        .expect("the canonical recipe should be accepted")
        .data;
    let order = accepted.order.expect("the order should be returned");
    assert_eq!(order.required_profession, ProfessionKind::Carpenter);
    assert_eq!(order.service, "Repair a field tool");
    assert_eq!(order.reward_gold, 5);
    assert!(order.benefit.contains("reliable tool"));
}
