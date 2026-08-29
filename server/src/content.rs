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

pub(crate) use frontier::{contract_template, threat_template};
pub(crate) use households::{opportunity_template, regional_household_template};
pub(crate) use npcs::household as npc_household;
pub(crate) use recipes::recipe_template;
pub(crate) use settlements::{infrastructure_profiles, settlement_profile, InfrastructureProfile};

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
    effects: Vec<String>,
    cause: String,
    intervention_options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EventTemplate {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) kind: String,
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
    let schema: ContentSchemaManifest = parse_json_labeled(
        "content_schema.json",
        macroquad_toolkit::include_json_str!("../../assets/data/content_schema.json"),
    )
    .map_err(|error| format!("content schema JSON is invalid: {error}"))?;
    let game_config: GameConfigManifest = parse_json_labeled("game_config.json", GAME_CONFIG_JSON)
        .map_err(|error| format!("game config JSON is invalid: {error}"))?;
    let actions: Vec<ActionManifest> = parse_json_labeled(
        "actions.json",
        macroquad_toolkit::include_json_str!("../../assets/data/actions.json"),
    )
    .map_err(|error| format!("actions JSON is invalid: {error}"))?;
    let crops: Vec<CropManifest> = parse_json_labeled(
        "crops.json",
        macroquad_toolkit::include_json_str!("../../assets/data/crops.json"),
    )
    .map_err(|error| format!("crops JSON is invalid: {error}"))?;
    let events: EventsManifest = parse_json_labeled(
        "events.json",
        macroquad_toolkit::include_json_str!("../../assets/data/events.json"),
    )
    .map_err(|error| format!("events JSON is invalid: {error}"))?;
    let items: ItemsManifest = parse_json_labeled(
        "items.json",
        macroquad_toolkit::include_json_str!("../../assets/data/items.json"),
    )
    .map_err(|error| format!("items JSON is invalid: {error}"))?;
    let region: RegionManifest = parse_json_labeled(
        "region.json",
        macroquad_toolkit::include_json_str!("../../assets/data/region.json"),
    )
    .map_err(|error| format!("region JSON is invalid: {error}"))?;

    validate_schema(&schema)?;
    validate_game_config(&game_config, &region)?;
    validate_actions(&actions)?;
    validate_crops(&crops)?;
    frontier::validate()?;
    validate_events(&events)?;
    validate_items(&items)?;
    validate_region(&region, &game_config)?;
    households::validate(&region)?;
    recipes::validate()?;
    let item_ids = items
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<HashSet<_>>();
    settlements::validate(&region, &game_config, &item_ids)?;
    npcs::validate()?;
    crate::repository::validate_skill_catalog()?;
    Ok(())
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
        validate_crops(&crops).expect("crops content must satisfy its schema");
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
        validate_events(&events).expect("events content must satisfy its schema");
        events.events
    });
    let event = events
        .get(event_index as usize % events.len())
        .expect("validated event catalog must not be empty");
    EventTemplate {
        id: event.id.clone(),
        title: event.title.clone(),
        kind: event.kind.clone(),
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
        validate_items(&items).expect("items content must satisfy its schema");
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
        validate_items(&items).expect("items content must satisfy its schema");
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

fn validate_schema(schema: &ContentSchemaManifest) -> Result<(), String> {
    if schema.schema_version == 0 || schema.compatibility.trim().is_empty() {
        return Err("content schema needs a positive version and compatibility rule".to_owned());
    }
    validate_id_list(
        "required manifest",
        schema
            .required_manifests
            .iter()
            .map(|name| name.as_str())
            .collect(),
    )?;
    for required in REQUIRED_MANIFESTS {
        if !schema
            .required_manifests
            .iter()
            .any(|name| name == required)
        {
            return Err(format!(
                "content schema is missing required manifest {required}"
            ));
        }
    }
    Ok(())
}

fn validate_game_config(
    config: &GameConfigManifest,
    region: &RegionManifest,
) -> Result<(), String> {
    if config.game_name.trim().is_empty()
        || config.display_name.trim().is_empty()
        || config.save_slot.trim().is_empty()
        || config.version.trim().is_empty()
        || config.world_width == 0
        || config.world_height == 0
        || !config.day_length_seconds.is_finite()
        || config.day_length_seconds <= 0.0
        || config.starting_gold == 0
        || config.starting_seeds == 0
        || config.starting_skill == 0
    {
        return Err("game config contains an empty or non-positive required value".to_owned());
    }
    if config.day_length_seconds != region.calendar.day_seconds as f32 {
        return Err("game config day length must match the region calendar".to_owned());
    }
    Ok(())
}

fn validate_actions(actions: &[ActionManifest]) -> Result<(), String> {
    validate_id_list(
        "action",
        actions.iter().map(|action| action.id.as_str()).collect(),
    )?;
    if actions.is_empty()
        || actions.iter().any(|action| {
            action.name.trim().is_empty()
                || action.description.trim().is_empty()
                || action.kind.trim().is_empty()
        })
    {
        return Err("actions need IDs, names, descriptions, and kinds".to_owned());
    }
    Ok(())
}

fn validate_crops(crops: &[CropManifest]) -> Result<(), String> {
    validate_id_list("crop", crops.iter().map(|crop| crop.id.as_str()).collect())?;
    if crops.is_empty()
        || crops
            .iter()
            .any(|crop| crop.name.trim().is_empty() || crop.description.trim().is_empty())
    {
        return Err("crops need IDs, names, and descriptions".to_owned());
    }
    for required in ["wheat", "turnip", "moonberry"] {
        if !crops.iter().any(|crop| crop.id == required) {
            return Err(format!("crops are missing the launch crop {required}"));
        }
    }
    if crops
        .iter()
        .any(|crop| !matches!(crop.id.as_str(), "wheat" | "turnip" | "moonberry"))
    {
        return Err("crops contain an ID without a protocol crop kind".to_owned());
    }
    Ok(())
}

fn validate_events(events: &EventsManifest) -> Result<(), String> {
    validate_id_list(
        "event",
        events
            .events
            .iter()
            .map(|event| event.id.as_str())
            .collect(),
    )?;
    if events.events.is_empty()
        || events.events.iter().any(|event| {
            event.title.trim().is_empty()
                || event.kind.trim().is_empty()
                || event.stages.is_empty()
                || event.affected_systems.is_empty()
                || event.effects.is_empty()
                || event.cause.trim().is_empty()
                || event.intervention_options.is_empty()
                || event.effects.iter().any(|effect| effect.trim().is_empty())
                || event
                    .intervention_options
                    .iter()
                    .any(|option| option.trim().is_empty())
                || event.stages.iter().any(|stage| {
                    !matches!(
                        stage.as_str(),
                        "signal" | "escalation" | "intervention" | "resolution" | "aftermath"
                    )
                })
        })
    {
        return Err("events need IDs, kinds, known stages, and affected systems".to_owned());
    }
    Ok(())
}

fn validate_items(items: &ItemsManifest) -> Result<(), String> {
    validate_id_list(
        "item",
        items.items.iter().map(|item| item.id.as_str()).collect(),
    )?;
    let item_ids: HashSet<&str> = items.items.iter().map(|item| item.id.as_str()).collect();
    validate_required_ids(
        "item",
        &item_ids,
        &[
            "wheat",
            "turnips",
            "moonberries",
            "seeds",
            "timber",
            "stone",
            "bandages",
        ],
    )?;
    if items.items.is_empty()
        || items.items.iter().any(|item| {
            item.kind.trim().is_empty() || item.sink.trim().is_empty() || item.base_price == 0
        })
    {
        return Err("items need IDs, kinds, economic sinks, and positive base prices".to_owned());
    }
    Ok(())
}

fn validate_region(
    region: &RegionManifest,
    game_config: &GameConfigManifest,
) -> Result<(), String> {
    region_validation::validate_region(region, game_config)
}

fn validate_required_ids(
    label: &str,
    available: &HashSet<&str>,
    required: &[&str],
) -> Result<(), String> {
    if let Some(missing) = required
        .iter()
        .find(|required_id| !available.contains(**required_id))
    {
        return Err(format!(
            "{label} catalog is missing required launch ID {missing}"
        ));
    }
    Ok(())
}

fn validate_id_list(label: &str, ids: Vec<&str>) -> Result<(), String> {
    if ids.is_empty()
        || ids.iter().any(|id| id.trim().is_empty())
        || ids.iter().collect::<HashSet<_>>().len() != ids.len()
    {
        return Err(format!("{label} IDs must be unique and non-empty"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
