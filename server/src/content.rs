//! Runtime validation for the data-driven regional content manifests.

use macroquad_toolkit::data_loader::parse_json_labeled;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::OnceLock;
use tarrowyn_protocol::{CropKind, LocationKind, Position, RouteStatus};

mod frontier;
mod households;
mod npcs;
mod recipes;
mod region_validation;
mod settlements;
mod validation;

pub(crate) use frontier::{contract_template, threat_template};
pub(crate) use households::{opportunity_template, regional_household_template};
pub(crate) use npcs::household as npc_household;
pub(crate) use recipes::recipe_template;
pub(crate) use settlements::{
    infrastructure_profiles, settlement_ids, settlement_profile, InfrastructureProfile,
};

const REQUIRED_MANIFESTS: &[&str] = &[
    "game_config.json",
    "actions.json",
    "crops.json",
    "contracts.json",
    "events.json",
    "items.json",
    "threats.json",
    "region.json",
    "households.json",
    "infrastructure.json",
    "npc_households.json",
    "recipes.json",
    "settlements.json",
    "skills.json",
];
const MAX_CONTENT_ID_CHARS: usize = 160;

#[derive(Debug, Deserialize)]
struct ContentSchemaManifest {
    schema_version: u32,
    required_manifests: Vec<String>,
    compatibility: String,
}

#[derive(Debug, Deserialize)]
struct GameConfigManifest {
    game_name: String,
    display_name: String,
    save_slot: String,
    version: String,
    world_width: u32,
    world_height: u32,
    day_length_seconds: f32,
    starting_gold: u32,
    starting_seeds: u32,
    starting_skill: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GameConfigDefaults {
    pub(crate) world_width: u32,
    pub(crate) world_height: u32,
    pub(crate) day_length_seconds: f32,
    pub(crate) starting_gold: u32,
    pub(crate) starting_seeds: u32,
    pub(crate) starting_skill: u32,
}

const GAME_CONFIG_JSON: &str =
    macroquad_toolkit::include_json_str!("../../assets/data/game_config.json");
static GAME_CONFIG: OnceLock<GameConfigManifest> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct ActionManifest {
    id: String,
    name: String,
    description: String,
    kind: String,
}

#[derive(Debug, Deserialize)]
struct CropManifest {
    id: String,
    name: String,
    description: String,
}

const CROPS_JSON: &str = macroquad_toolkit::include_json_str!("../../assets/data/crops.json");
static CROP_CATALOG: OnceLock<Vec<CropManifest>> = OnceLock::new();
const EVENTS_JSON: &str = macroquad_toolkit::include_json_str!("../../assets/data/events.json");
static EVENT_CATALOG: OnceLock<Vec<EventManifest>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct ItemsManifest {
    items: Vec<ItemManifest>,
}

static ITEM_CATALOG: OnceLock<Vec<ItemManifest>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct ItemManifest {
    id: String,
    kind: String,
    sink: String,
    base_price: u32,
}

#[derive(Debug, Deserialize)]
struct EventsManifest {
    events: Vec<EventManifest>,
}

#[derive(Debug, Deserialize)]
struct EventManifest {
    id: String,
    title: String,
    kind: String,
    stages: Vec<String>,
    affected_systems: Vec<String>,
    affected_locations: Vec<String>,
    effects: Vec<String>,
    cause: String,
    intervention_options: Vec<String>,
}

const SUPPORTED_EVENT_INTERVENTIONS: &[&str] = &[
    "repair ferry markers",
    "escort the grain caravan",
    "open the frontier storehouse",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EventTemplate {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) kind: String,
    pub(crate) affected_locations: Vec<String>,
    pub(crate) effects: Vec<String>,
    pub(crate) cause: String,
    pub(crate) intervention_options: Vec<String>,
}

static REGION_CATALOG: OnceLock<RegionManifest> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct RegionManifest {
    region_id: String,
    calendar: CalendarManifest,
    farm_plots: Vec<Position>,
    farm_animal_position: Position,
    foundation_baseline: tarrowyn_protocol::FoundationBaseline,
    locations: Vec<LocationManifest>,
    routes: Vec<RouteManifest>,
}

#[derive(Debug, Deserialize)]
struct CalendarManifest {
    day_seconds: u32,
    season_days: u32,
    year_days: u32,
    seasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct LocationManifest {
    id: String,
    name: String,
    kind: String,
    position: Position,
    role: String,
    resources: Vec<String>,
    services: Vec<String>,
    condition: u8,
    access_note: String,
}

#[derive(Debug, Deserialize)]
struct RouteManifest {
    id: String,
    name: String,
    transport: String,
    origin: String,
    destination: String,
    length: u32,
    risk_percent: u8,
    condition: u8,
    capacity: u32,
    travel_ticks: u64,
    repair_cost: u32,
    status: String,
    note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegionRouteProfile {
    pub(crate) name: String,
    pub(crate) transport: String,
    pub(crate) origin: String,
    pub(crate) destination: String,
    pub(crate) length: u32,
    pub(crate) risk_percent: u8,
    pub(crate) condition: u8,
    pub(crate) capacity: u32,
    pub(crate) travel_ticks: u64,
    pub(crate) repair_cost: u32,
    pub(crate) status: RouteStatus,
    pub(crate) note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegionLocationProfile {
    pub(crate) name: String,
    pub(crate) kind: LocationKind,
    pub(crate) position: Position,
    pub(crate) role: String,
    pub(crate) resources: Vec<String>,
    pub(crate) services: Vec<String>,
    pub(crate) condition: u8,
    pub(crate) access_note: String,
}

pub fn validate() -> Result<(), String> {
    validation::validate()
}

fn validate_id_list(label: &str, ids: Vec<&str>) -> Result<(), String> {
    validation::validate_id_list(label, ids)
}

fn validate_required_ids(
    label: &str,
    available: &HashSet<&str>,
    required: &[&str],
) -> Result<(), String> {
    validation::validate_required_ids(label, available, required)
}

#[cfg(test)]
fn validate_actions(actions: &[ActionManifest]) -> Result<(), String> {
    validation::validate_actions(actions)
}

#[cfg(test)]
fn validate_events(events: &EventsManifest, region: &RegionManifest) -> Result<(), String> {
    validation::validate_events(events, region)
}

#[cfg(test)]
fn validate_region(
    region: &RegionManifest,
    game_config: &GameConfigManifest,
) -> Result<(), String> {
    region_validation::validate_region(region, game_config)
}

pub(crate) fn starting_skill() -> u32 {
    game_config_defaults().starting_skill
}

pub(crate) fn game_config_defaults() -> GameConfigDefaults {
    let config = GAME_CONFIG.get_or_init(|| {
        let config: GameConfigManifest = parse_json_labeled("game_config.json", GAME_CONFIG_JSON)
            .expect("game config content JSON must be valid");
        if config.starting_skill == 0 {
            panic!("game config content must define a positive starting skill");
        }
        config
    });
    GameConfigDefaults {
        world_width: config.world_width,
        world_height: config.world_height,
        day_length_seconds: config.day_length_seconds,
        starting_gold: config.starting_gold,
        starting_seeds: config.starting_seeds,
        starting_skill: config.starting_skill,
    }
}

pub(crate) fn crop_kind_for_seed(seed_index: u32) -> CropKind {
    let crops = CROP_CATALOG.get_or_init(|| {
        let crops: Vec<CropManifest> =
            parse_json_labeled("crops.json", CROPS_JSON).expect("crops content JSON must be valid");
        validation::validate_crops(&crops).expect("crops content must satisfy its schema");
        crops
    });
    let crop = crops
        .get(seed_index as usize % crops.len())
        .expect("validated crop catalog must not be empty");
    match crop.id.as_str() {
        "wheat" => CropKind::Wheat,
        "turnip" => CropKind::Turnip,
        "moonberry" => CropKind::Moonberry,
        _ => panic!("validated crop catalog contains an unsupported crop ID"),
    }
}

pub(crate) fn regional_event_template(event_index: u64) -> EventTemplate {
    let events = EVENT_CATALOG.get_or_init(|| {
        let events: EventsManifest = parse_json_labeled("events.json", EVENTS_JSON)
            .expect("events content JSON must be valid");
        validation::validate_events(&events, region_catalog())
            .expect("events content must satisfy its schema");
        events.events
    });
    let event = events
        .get(event_index as usize % events.len())
        .expect("validated event catalog must not be empty");
    EventTemplate {
        id: event.id.clone(),
        title: event.title.clone(),
        kind: event.kind.clone(),
        affected_locations: event.affected_locations.clone(),
        effects: event.effects.clone(),
        cause: event.cause.clone(),
        intervention_options: event.intervention_options.clone(),
    }
}

pub(crate) fn item_base_price(item_id: &str) -> u32 {
    let items = ITEM_CATALOG.get_or_init(|| {
        let items: ItemsManifest = parse_json_labeled(
            "items.json",
            macroquad_toolkit::include_json_str!("../../assets/data/items.json"),
        )
        .expect("items content JSON must be valid");
        validation::validate_items(&items).expect("items content must satisfy its schema");
        items.items
    });
    items
        .iter()
        .find(|item| item.id == item_id)
        .map(|item| item.base_price)
        .expect("validated item catalog must contain the requested item")
}

pub(super) fn item_ids() -> HashSet<String> {
    let items = ITEM_CATALOG.get_or_init(|| {
        let items: ItemsManifest = parse_json_labeled(
            "items.json",
            macroquad_toolkit::include_json_str!("../../assets/data/items.json"),
        )
        .expect("items content JSON must be valid");
        validation::validate_items(&items).expect("items content must satisfy its schema");
        items.items
    });
    items.iter().map(|item| item.id.clone()).collect()
}

pub(crate) fn season_for_day(day: u32) -> String {
    let region = region_catalog();
    let season_index = (day.saturating_sub(1) / region.calendar.season_days) as usize
        % region.calendar.seasons.len();
    region.calendar.seasons[season_index].clone()
}

pub(crate) fn farm_plot_positions() -> Vec<Position> {
    region_catalog().farm_plots.clone()
}

pub(crate) fn farm_animal_position() -> Position {
    region_catalog().farm_animal_position
}

pub(crate) fn foundation_baseline() -> tarrowyn_protocol::FoundationBaseline {
    region_catalog().foundation_baseline.clone()
}

pub(crate) fn region_id() -> String {
    region_catalog().region_id.clone()
}

pub(crate) fn region_location_ids() -> Vec<String> {
    region_catalog()
        .locations
        .iter()
        .map(|location| location.id.clone())
        .collect()
}

pub(crate) fn region_route_ids() -> Vec<String> {
    region_catalog()
        .routes
        .iter()
        .map(|route| route.id.clone())
        .collect()
}

pub(crate) fn region_route_profile(route_id: &str) -> RegionRouteProfile {
    let route = region_catalog()
        .routes
        .iter()
        .find(|route| route.id == route_id)
        .expect("validated region catalog must contain the requested route");
    RegionRouteProfile {
        name: route.name.clone(),
        transport: route.transport.clone(),
        origin: route.origin.clone(),
        destination: route.destination.clone(),
        length: route.length,
        risk_percent: route.risk_percent,
        condition: route.condition,
        capacity: route.capacity,
        travel_ticks: route.travel_ticks,
        repair_cost: route.repair_cost,
        status: match route.status.as_str() {
            "operational" => RouteStatus::Operational,
            "delayed" => RouteStatus::Delayed,
            "threatened" => RouteStatus::Threatened,
            "repairing" => RouteStatus::Repairing,
            "closed" => RouteStatus::Closed,
            _ => panic!("validated region catalog contains an unsupported route status"),
        },
        note: route.note.clone(),
    }
}

pub(crate) fn region_location_profile(location_id: &str) -> RegionLocationProfile {
    let location = region_catalog()
        .locations
        .iter()
        .find(|location| location.id == location_id)
        .expect("validated region catalog must contain the requested location");
    RegionLocationProfile {
        name: location.name.clone(),
        kind: match location.kind.as_str() {
            "settlement" => LocationKind::Settlement,
            "outpost" => LocationKind::Outpost,
            "frontier" => LocationKind::Frontier,
            _ => panic!("validated region catalog contains an unsupported location kind"),
        },
        position: location.position,
        role: location.role.clone(),
        resources: location.resources.clone(),
        services: location.services.clone(),
        condition: location.condition,
        access_note: location.access_note.clone(),
    }
}

fn region_catalog() -> &'static RegionManifest {
    REGION_CATALOG.get_or_init(|| {
        let region: RegionManifest = parse_json_labeled(
            "region.json",
            macroquad_toolkit::include_json_str!("../../assets/data/region.json"),
        )
        .expect("region content JSON must be valid");
        if region.calendar.season_days == 0 || region.calendar.seasons.is_empty() {
            panic!("region content must define a non-empty calendar");
        }
        region
    })
}

#[cfg(test)]
mod tests;
