use super::super::super::{ServerConfig, WorldRepository};
use super::super::logic::expire_market_orders;
use tarrowyn_protocol::{
    CommodityKind, GuestSessionRequest, MarketOrder, MarketOrderAction, MarketOrderRequest,
    MarketOrderStatus,
};

#[test]
fn accepted_market_order_history_reaches_both_route_endpoints() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase5-market-history".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    let order = repository
        .market_order(
            &session.account_token,
            MarketOrderRequest {
                request_id: "market-history-create".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("whisperwood-outpost".to_owned()),
                commodity: Some(CommodityKind::Seeds),
                quantity: Some(1),
            },
        )
        .expect("market order")
        .data;
    assert!(order.accepted);

    let state = repository.state.lock().unwrap();
    for location_id in ["hearth", "whisperwood-outpost"] {
        let settlement = state
            .phase5
            .settlements
            .iter()
            .find(|settlement| settlement.location_id == location_id)
            .expect("route endpoint settlement");
        assert!(settlement
            .chronicle
            .iter()
            .any(|entry| entry.kind == "regional market"));
    }
}

#[test]
fn expired_market_order_history_reaches_its_recorded_endpoints() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.tick = 49;
        state.phase5.market_orders.push(MarketOrder {
            order_id: "expired-saltmere-order".to_owned(),
            owner_account_id: "account".to_owned(),
            owner_name: "Resident".to_owned(),
            origin_location_id: "saltmere".to_owned(),
            destination_location_id: "hearth".to_owned(),
            commodity: CommodityKind::Stone,
            quantity: 1,
            unit_price: 3,
            total_price: 3,
            status: MarketOrderStatus::Open,
            created_tick: 0,
            settled_tick: None,
            route_id: "saltmere-ferry".to_owned(),
            fallback_used: false,
        });

        expire_market_orders(&mut state);
    }

    let state = repository.state.lock().expect("repository lock");
    for location_id in ["saltmere", "hearth"] {
        let settlement = state
            .phase5
            .settlements
            .iter()
            .find(|settlement| settlement.location_id == location_id)
            .expect("recorded order endpoint settlement");
        assert!(settlement
            .chronicle
            .iter()
            .any(|entry| entry.kind == "market fulfilment failed"));
    }
    let outpost = state
        .phase5
        .settlements
        .iter()
        .find(|settlement| settlement.location_id == "whisperwood-outpost")
        .expect("unrelated settlement");
    assert!(!outpost
        .chronicle
        .iter()
        .any(|entry| entry.kind == "market fulfilment failed"));
}

#[test]
fn market_order_owner_cannot_fulfil_their_own_shipment() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase5-market-self-fulfil".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let order = repository
        .market_order(
            &session.account_token,
            MarketOrderRequest {
                request_id: "self-fulfil-create".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("saltmere".to_owned()),
                commodity: Some(CommodityKind::Seeds),
                quantity: Some(1),
            },
        )
        .expect("market order")
        .data
        .order
        .expect("created order");
    let order_id = order.order_id.clone();
    let before = {
        let mut state = repository.state.lock().expect("repository lock");
        let saltmere = state
            .phase5
            .locations
            .iter()
            .find(|location| location.location_id == "saltmere")
            .expect("saltmere location")
            .position;
        let identity = state
            .identities
            .get_mut(&session.client_key)
            .expect("identity exists");
        identity.position = saltmere;
        (identity.gold, identity.inventory.seeds)
    };

    let response = repository
        .market_order(
            &session.account_token,
            MarketOrderRequest {
                request_id: "self-fulfil-at-destination".to_owned(),
                action: MarketOrderAction::Fulfil,
                order_id: Some(order_id.clone()),
                destination_location_id: None,
                commodity: None,
                quantity: None,
            },
        )
        .expect("self fulfil response")
        .data;

    assert!(!response.accepted);
    assert_eq!(
        response.reason.as_deref(),
        Some("The order owner cannot fulfil their own shipment.")
    );
    let state = repository.state.lock().expect("repository lock");
    let identity = state
        .identities
        .get(&session.client_key)
        .expect("identity exists");
    assert_eq!((identity.gold, identity.inventory.seeds), before);
    assert_eq!(
        state
            .phase5
            .market_orders
            .iter()
            .find(|order| order.order_id == order_id)
            .expect("order remains recorded")
            .status,
        MarketOrderStatus::Open
    );
}
