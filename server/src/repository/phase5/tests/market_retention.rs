use super::super::super::{ServerConfig, WorldRepository};
use super::guest;
use tarrowyn_protocol::{
    CommodityKind, MarketOrder, MarketOrderAction, MarketOrderRequest, MarketOrderStatus,
};

fn market_order(index: usize, status: MarketOrderStatus) -> MarketOrder {
    MarketOrder {
        order_id: format!("market-order-{index}"),
        owner_account_id: "account".to_owned(),
        owner_name: "Resident".to_owned(),
        origin_location_id: "hearth".to_owned(),
        destination_location_id: "whisperwood-outpost".to_owned(),
        commodity: CommodityKind::Seeds,
        quantity: 1,
        unit_price: 2,
        total_price: 2,
        status,
        created_tick: index as u64,
        settled_tick: (status != MarketOrderStatus::Open).then_some(index as u64),
        route_id: "north-pack-road".to_owned(),
        fallback_used: false,
    }
}

fn create_request(request_id: &str) -> MarketOrderRequest {
    MarketOrderRequest {
        request_id: request_id.to_owned(),
        action: MarketOrderAction::Create,
        order_id: None,
        destination_location_id: Some("whisperwood-outpost".to_owned()),
        commodity: Some(CommodityKind::Seeds),
        quantity: Some(1),
    }
}

#[test]
fn market_order_history_evicts_settled_records_and_preserves_live_escrow() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase5-market-retention");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.market_orders = (0..128)
            .map(|index| market_order(index, MarketOrderStatus::Fulfilled))
            .collect();
    }

    let created = repository
        .market_order(
            &session.account_token,
            create_request("market-after-history"),
        )
        .expect("settled history should make room")
        .data;
    assert!(created.accepted);
    {
        let state = repository.state.lock().expect("repository lock");
        assert_eq!(state.phase5.market_orders.len(), 128);
        assert!(!state
            .phase5
            .market_orders
            .iter()
            .any(|order| order.order_id == "market-order-0"));
    }

    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.market_orders = (0..128)
            .map(|index| market_order(index, MarketOrderStatus::Open))
            .collect();
    }
    let blocked = repository
        .market_order(&session.account_token, create_request("market-while-full"))
        .expect("full market ledger should return a readable response")
        .data;
    assert!(!blocked.accepted);
    assert!(blocked
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("ledger is full")));
    assert_eq!(
        repository
            .inventory(&session.account_token)
            .unwrap()
            .data
            .inventory
            .seeds,
        5
    );
}

#[test]
fn over_capacity_market_history_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase5-market-integrity");
    repository
        .market_order(
            &session.account_token,
            create_request("market-integrity-seed"),
        )
        .expect("market order");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let order = state
            .phase5
            .market_orders
            .first()
            .cloned()
            .expect("market order");
        state.phase5.market_orders.resize(129, order);
    }

    assert!(!repository.ops_health().data.integrity_ok);
}
