//! Regional fixtures and durable state records.

use super::logic::stock_key;
use crate::config::ServerConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tarrowyn_protocol::{
    LocationKind, LocationRecord, MarketOrder, MarketOrderResponse, Position, RegionalEvent,
    RegionalEventResponse, RegionalHousehold, RouteRecord, RouteResponse, RouteStatus,
    SettlementCondition, SettlementProjection, TravelResponse, TravelState,
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
        location(
            "hearth",
            "The Hearth",
            LocationKind::Settlement,
            Position { x: 8, y: 6 },
            &["town hall", "market", "healer"],
            76,
            "The open settlement is the reliable entry point for new players.",
        ),
        location(
            "whisperwood-outpost",
            "Whisperwood Watch",
            LocationKind::Outpost,
            Position { x: 12, y: 4 },
            &["scout vacancy", "frontier contracts"],
            38,
            "A pioneer can enter through a watch, repair, or caravan role.",
        ),
        location(
            "saltmere",
            "Saltmere Landing",
            LocationKind::Settlement,
            Position { x: 3, y: 9 },
            &["boat service", "stonecutting"],
            62,
            "The landing is open to carriers and a service vacancy even when busy.",
        ),
    ];
    let routes = vec![
        route(
            "north-pack-road",
            "North pack road",
            6,
            28,
            72,
            3,
            6,
            4,
            RouteStatus::Threatened,
            "A threatened road can be repaired, escorted, or delayed without deleting a journey.",
        ),
        route(
            "saltmere-ferry",
            "Saltmere ferry",
            7,
            12,
            84,
            2,
            7,
            3,
            RouteStatus::Operational,
            "The ferry is slower than a finished road but keeps the landing supplied.",
        ),
        route(
            "watch-trail",
            "Watch trail",
            9,
            34,
            55,
            1,
            9,
            5,
            RouteStatus::Delayed,
            "The frontier trail is a recovery opportunity for scouts and repair crews.",
        ),
    ];
    let settlements = vec![
        settlement(
            "hearth-settlement",
            "The Hearth",
            "hearth",
            36,
            72,
            70,
            80,
            62,
            80,
            65,
            SettlementCondition::Stable,
            &["shared fields endure", "town hall keeps public records"],
            &["caravan quartermaster", "field-tool repair"],
            &["bandages", "stone"],
            108,
        ),
        settlement(
            "whisperwood-settlement",
            "Whisperwood Watch",
            "whisperwood-outpost",
            8,
            42,
            36,
            38,
            74,
            42,
            14,
            SettlementCondition::Quiet,
            &["the first watchtower stands"],
            &["bridge warden", "healer", "wood hauler"],
            &["food", "bandages", "tools"],
            126,
        ),
        settlement(
            "saltmere-settlement",
            "Saltmere Landing",
            "saltmere",
            18,
            61,
            76,
            65,
            38,
            60,
            24,
            SettlementCondition::Stable,
            &["the ferry ledger is open"],
            &["ferry hand", "herbal gatherer"],
            &["seeds", "timber"],
            94,
        ),
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

fn location(
    id: &str,
    name: &str,
    kind: LocationKind,
    position: Position,
    services: &[&str],
    condition: u8,
    access_note: &str,
) -> LocationRecord {
    let profile = crate::content::region_location_profile(id);
    LocationRecord {
        location_id: id.to_owned(),
        name: name.to_owned(),
        kind,
        position,
        role: profile.role,
        resources: profile.resources,
        services: services.iter().map(|value| (*value).to_owned()).collect(),
        condition,
        access_note: access_note.to_owned(),
    }
}

#[allow(clippy::too_many_arguments)]
fn route(
    id: &str,
    name: &str,
    length: u32,
    risk: u8,
    condition: u8,
    capacity: u32,
    travel_ticks: u64,
    repair_cost: u32,
    status: RouteStatus,
    note: &str,
) -> RouteRecord {
    let profile = crate::content::region_route_profile(id);
    RouteRecord {
        route_id: id.to_owned(),
        name: name.to_owned(),
        origin_location_id: profile.origin,
        destination_location_id: profile.destination,
        transport: profile.transport,
        length,
        risk_percent: risk,
        condition,
        capacity,
        travel_ticks,
        repair_cost,
        status,
        last_action_tick: 0,
        note: note.to_owned(),
    }
}

#[allow(clippy::too_many_arguments)]
fn settlement(
    id: &str,
    name: &str,
    location: &str,
    population: u32,
    food: u8,
    safety: u8,
    infrastructure: u8,
    industry: u8,
    governance: u8,
    activity: u8,
    condition: SettlementCondition,
    milestones: &[&str],
    vacancies: &[&str],
    demand: &[&str],
    price: u16,
) -> SettlementProjection {
    let supply = crate::content::settlement_supply_profile(id);
    SettlementProjection {
        settlement_id: id.to_owned(),
        name: name.to_owned(),
        location_id: location.to_owned(),
        population,
        food,
        safety,
        infrastructure,
        industry,
        governance,
        player_activity: activity,
        condition,
        milestones: milestones.iter().map(|value| (*value).to_owned()).collect(),
        vacancies: vacancies.iter().map(|value| (*value).to_owned()).collect(),
        demand: demand.iter().map(|value| (*value).to_owned()).collect(),
        abundant_goods: supply.abundant,
        scarce_goods: supply.scarce,
        price_index_percent: price,
        chronicle: Vec::new(),
        recovery_opportunity: None,
    }
}
