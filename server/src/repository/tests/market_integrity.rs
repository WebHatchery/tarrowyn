use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{CommodityKind, MarketOrder, MarketOrderStatus};

fn open_order() -> MarketOrder {
    MarketOrder {
        order_id: "integrity-market-lifecycle".to_owned(),
        owner_account_id: "former-resident".to_owned(),
        owner_name: "Former resident".to_owned(),
        origin_location_id: "hearth".to_owned(),
        destination_location_id: "saltmere".to_owned(),
        commodity: CommodityKind::Stone,
        quantity: 1,
        unit_price: 3,
        total_price: 3,
        status: MarketOrderStatus::Open,
        created_tick: 0,
        settled_tick: None,
        route_id: "saltmere-ferry".to_owned(),
        fallback_used: false,
    }
}

#[test]
fn oversized_market_quantity_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        let mut order = open_order();
        order.quantity = 100;
        order.total_price = 300;
        state.phase5.market_orders.push(order);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn open_market_order_cannot_have_a_settled_tick() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        let mut order = open_order();
        order.settled_tick = Some(0);
        state.phase5.market_orders.push(order);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
