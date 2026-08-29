use super::super::WorldRepository;
use crate::ServerConfig;
use tarrowyn_protocol::{
    ClaimLifecycleAction, ClaimLifecycleRequest, GuestSessionRequest, MarketOrderAction,
    MarketOrderRequest, ProfessionAction, ProfessionKind, ProfessionRequest, TravelAction,
    TravelRequest,
};

#[test]
fn guest_reset_replaces_private_state_and_releases_world_ownership() {
    let repository = WorldRepository::new(ServerConfig::default());
    let first = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("reset-private-state".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    {
        let mut state = repository.state.lock().unwrap();
        state
            .phase3
            .expedition_credentials
            .push(first.account_id.clone());
    }
    let order = repository
        .profession_order(
            &first.account_token,
            ProfessionRequest {
                request_id: "reset-order".to_owned(),
                action: ProfessionAction::CreateOrder,
                order_id: None,
                profession: Some(ProfessionKind::Carpenter),
                capability_id: None,
                service: None,
                timing_score: None,
            },
        )
        .unwrap()
        .data
        .order
        .expect("the first guest should create an order");
    let claim = repository
        .claim_lifecycle(
            &first.account_token,
            ClaimLifecycleRequest {
                request_id: "reset-claim".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data
        .claim
        .expect("the first guest should receive a claim");
    let travel = repository
        .travel(
            &first.account_token,
            TravelRequest {
                request_id: "reset-travel".to_owned(),
                action: TravelAction::Start,
                route_id: Some("north-pack-road".to_owned()),
                travel_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(travel.accepted);
    let market = repository
        .market_order(
            &first.account_token,
            MarketOrderRequest {
                request_id: "reset-market".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("whisperwood-outpost".to_owned()),
                commodity: Some(tarrowyn_protocol::CommodityKind::Seeds),
                quantity: Some(1),
            },
        )
        .unwrap()
        .data;
    assert!(market.accepted);

    let second = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("reset-private-state".to_owned()),
            reset: true,
        })
        .unwrap()
        .data;
    assert_ne!(first.account_id, second.account_id);
    let professions = repository.professions(&second.account_token).unwrap().data;
    assert_eq!(professions.materials.wood, 3);
    assert_eq!(professions.materials.iron, 2);
    assert_eq!(professions.materials.tools, 1);
    assert!(repository
        .region(&second.account_token)
        .unwrap()
        .data
        .travel
        .is_none());
    let state = repository.state.lock().unwrap();
    assert!(!state.phase3.contracts.contains_key(&first.client_key));
    assert!(!state
        .phase3
        .expedition_credentials
        .iter()
        .any(|id| id == &first.account_id));
    assert!(!state.phase5.travel.contains_key(&first.client_key));
    assert!(!state.phase4.request_results.keys().any(|key| {
        key.starts_with(&format!("phase4:{}:", first.account_id))
            || key.starts_with(&format!("skill-practice:{}:", first.account_id))
    }));
    assert!(!state
        .phase5
        .request_results
        .keys()
        .any(|key| key.starts_with(&format!("phase5:{}:", first.client_key))));
    let claim = state
        .phase4
        .claims
        .iter()
        .find(|candidate| candidate.claim_id == claim.claim_id)
        .expect("the released claim remains as registry history");
    assert_eq!(
        claim.status,
        tarrowyn_protocol::ClaimLifecycleStatus::Reclaimed
    );
    assert!(claim.owner_account_id.is_none());
    let order = state
        .phase4
        .orders
        .iter()
        .find(|candidate| candidate.order_id == order.order_id)
        .expect("the reset order remains as history");
    assert_eq!(
        order.status,
        tarrowyn_protocol::ServiceOrderStatus::Cancelled
    );
    let market = state
        .phase5
        .market_orders
        .iter()
        .find(|candidate| {
            market
                .order
                .as_ref()
                .is_some_and(|created| created.order_id == candidate.order_id)
        })
        .expect("the reset market order remains as history");
    assert_eq!(market.owner_account_id, "former-resident");
    assert_eq!(
        state
            .identities
            .get(&second.client_key)
            .expect("the replacement identity should remain")
            .account_id,
        second.account_id
    );
}
