//! Regional fixtures and durable state records.

use super::logic::stock_key;
use crate::config::ServerConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tarrowyn_protocol::{
    LocationRecord, MarketOrder, MarketOrderResponse, RegionalEvent, RegionalEventResponse,
    RegionalHousehold, RouteRecord, RouteResponse, SettlementProjection, TravelResponse,
    TravelState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Phase5Response {
    Travel(TravelResponse),
    Route(RouteResponse),
    Market(MarketOrderResponse),
    Event(RegionalEventResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Phase5State {
    pub(crate) next_travel_id: u64,
    pub(crate) next_order_id: u64,
    pub(crate) next_event_id: u64,
    pub(crate) cursor: u64,
    pub(crate) locations: Vec<LocationRecord>,
    pub(crate) routes: Vec<RouteRecord>,
    pub(crate) settlements: Vec<SettlementProjection>,
    pub(crate) travel: HashMap<String, TravelState>,
    pub(crate) market_orders: Vec<MarketOrder>,
    pub(crate) stock: HashMap<String, u32>,
    pub(crate) events: Vec<RegionalEvent>,
    pub(crate) households: Vec<RegionalHousehold>,
    pub(crate) request_results: HashMap<String, Phase5Response>,
}

impl Default for Phase5State {
    fn default() -> Self {
        fresh(&ServerConfig::default())
    }
}

pub(crate) fn fresh(_config: &ServerConfig) -> Phase5State {
    let household = crate::content::regional_household_template("household-maren");
    let locations = vec![
        location("hearth"),
        location("whisperwood-outpost"),
        location("saltmere"),
    ];
    let routes = vec![
        route("north-pack-road"),
        route("saltmere-ferry"),
        route("watch-trail"),
    ];
    let settlements = vec![
        settlement("hearth-settlement"),
        settlement("whisperwood-settlement"),
        settlement("saltmere-settlement"),
    ];
    let mut stock = HashMap::new();
    for (location, commodity, quantity) in [
        ("hearth", "timber", 4),
        ("hearth", "stone", 6),
        ("whisperwood-outpost", "timber", 18),
        ("whisperwood-outpost", "stone", 2),
        ("saltmere", "stone", 20),
        ("saltmere", "bandages", 12),
    ] {
        stock.insert(stock_key(location, commodity), quantity);
    }
    Phase5State {
        next_travel_id: 1,
        next_order_id: 1,
        next_event_id: 1,
        cursor: 0,
        locations,
        routes,
        settlements,
        travel: HashMap::new(),
        market_orders: Vec::new(),
        stock,
        events: Vec::new(),
        households: vec![RegionalHousehold {
            household_id: household.household_id,
            household_name: household.household_name,
            origin_location_id: household.origin_location_id,
            destination_location_id: household.destination_location_id,
            status: household.status,
            reason: household.reason,
            service: household.service,
            departure_tick: None,
            arrival_tick: None,
            history: household.history,
        }],
        request_results: HashMap::new(),
    }
}

fn location(id: &str) -> LocationRecord {
    let profile = crate::content::region_location_profile(id);
    LocationRecord {
        location_id: id.to_owned(),
        name: profile.name,
        kind: profile.kind,
        position: profile.position,
        role: profile.role,
        resources: profile.resources,
        services: profile.services,
        condition: profile.condition,
        access_note: profile.access_note,
    }
}

fn route(id: &str) -> RouteRecord {
    let profile = crate::content::region_route_profile(id);
    RouteRecord {
        route_id: id.to_owned(),
        name: profile.name,
        origin_location_id: profile.origin,
        destination_location_id: profile.destination,
        transport: profile.transport,
        length: profile.length,
        risk_percent: profile.risk_percent,
        condition: profile.condition,
        capacity: profile.capacity,
        travel_ticks: profile.travel_ticks,
        repair_cost: profile.repair_cost,
        status: profile.status,
        last_action_tick: 0,
        note: profile.note,
    }
}

fn settlement(id: &str) -> SettlementProjection {
    let profile = crate::content::settlement_profile(id);
    SettlementProjection {
        settlement_id: id.to_owned(),
        name: profile.name,
        location_id: profile.location,
        population: profile.population,
        food: profile.food,
        safety: profile.safety,
        infrastructure: profile.infrastructure,
        industry: profile.industry,
        governance: profile.governance,
        player_activity: profile.player_activity,
        claim_count: 0,
        available_plot_count: 0,
        public_works: Vec::new(),
        condition: profile.condition,
        milestones: profile.milestones,
        vacancies: profile.vacancies,
        demand: profile.demand,
        abundant_goods: profile.abundant,
        scarce_goods: profile.scarce,
        price_index_percent: profile.price_index_percent,
        chronicle: Vec::new(),
        recovery_opportunity: None,
    }
}
