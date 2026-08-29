use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{CommodityKind, MarketOrder, MarketOrderStatus, TravelState, TravelStatus};

#[test]
fn invalid_regional_route_bounds_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.routes[0].risk_percent = 101;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_regional_settlement_bounds_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.settlements[0].governance = 101;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn missing_regional_collections_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.routes.clear();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_regional_topology_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.routes[0].origin_location_id = "missing-location".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn duplicate_settlement_locations_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        let location_id = state.phase5.settlements[0].location_id.clone();
        state.phase5.settlements[1].location_id = location_id;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_market_order_reference_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.market_orders.push(MarketOrder {
            order_id: "integrity-market-order".to_owned(),
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
            route_id: "missing-route".to_owned(),
            fallback_used: false,
        });
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_travel_reference_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = super::guest(&repository, "integrity-travel");
    let identity_key = {
        let state = repository.state.lock().expect("repository lock");
        state
            .identities
            .iter()
            .find(|(_, identity)| identity.character_id == session.character_id)
            .map(|(key, _)| key.clone())
            .expect("guest identity")
    };
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.travel.insert(
            identity_key,
            TravelState {
                travel_id: "integrity-travel".to_owned(),
                route_id: "missing-route".to_owned(),
                origin_location_id: "hearth".to_owned(),
                destination_location_id: "saltmere".to_owned(),
                departure_tick: 0,
                eta_tick: 7,
                progress: 0,
                risk_percent: 12,
                status: TravelStatus::Travelling,
                interruption: None,
                recovery_note: None,
            },
        );
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
