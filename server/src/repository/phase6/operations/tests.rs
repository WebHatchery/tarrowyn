use super::super::super::models::RepositoryState;
use crate::config::ServerConfig;
use tarrowyn_protocol::{
    CommodityKind, MarketOrder, MarketOrderStatus, RegionalEvent, RegionalEventStage,
};

#[test]
fn operational_alerts_cover_tick_regional_and_economy_boundaries() {
    let mut state = RepositoryState::fresh(&ServerConfig::default());
    let quiet = super::alert_flags(&state, &ServerConfig::default(), false, false, false);
    assert!(quiet.is_empty());

    state
        .phase5
        .events
        .extend((0..129).map(|index| RegionalEvent {
            event_id: format!("alert-event-{index}"),
            title: "A gathering cloud".to_owned(),
            kind: "weather".to_owned(),
            stage: RegionalEventStage::Signal,
            affected_location_ids: vec!["hearth".to_owned()],
            effects: vec!["visibility falls".to_owned()],
            cause: "test pressure".to_owned(),
            intervention_options: vec!["watch".to_owned()],
            chosen_intervention: None,
            outcome: None,
            started_tick: 1,
            updated_tick: 1,
            cursor: index,
        }));
    state.phase5.market_orders.push(MarketOrder {
        order_id: "alert-order".to_owned(),
        owner_account_id: "account".to_owned(),
        owner_name: "Traveller".to_owned(),
        origin_location_id: "hearth".to_owned(),
        destination_location_id: "saltmere".to_owned(),
        commodity: CommodityKind::Wheat,
        quantity: 0,
        unit_price: 4,
        total_price: 0,
        status: MarketOrderStatus::Open,
        created_tick: 1,
        settled_tick: None,
        route_id: "saltmere-ferry".to_owned(),
        fallback_used: false,
    });

    let flags = super::alert_flags(&state, &ServerConfig::default(), false, false, true);
    assert!(flags.iter().any(|flag| flag == "tick_drift"));
    assert!(flags.iter().any(|flag| flag == "regional_event_backlog"));
    assert!(flags.iter().any(|flag| flag == "economy_anomaly"));
}

#[test]
fn operational_health_names_the_failed_integrity_boundary() {
    let repository = super::super::super::WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.routes[0].condition = 101;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
    assert!(health
        .integrity_failures
        .iter()
        .any(|failure| failure == "route_bounds"));
}
