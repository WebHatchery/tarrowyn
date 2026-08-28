//! Runtime validation for the data-driven regional content manifests.

use serde::Deserialize;
use std::collections::HashSet;
use std::sync::OnceLock;
use tarrowyn_protocol::CropKind;

const REQUIRED_MANIFESTS: &[&str] = &[
    "game_config.json",
    "actions.json",
    "crops.json",
    "events.json",
    "items.json",
    "region.json",
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
    starting_skill: u32,
}

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

const CROPS_JSON: &str = include_str!("../../assets/data/crops.json");
static CROP_CATALOG: OnceLock<Vec<CropManifest>> = OnceLock::new();
const EVENTS_JSON: &str = include_str!("../../assets/data/events.json");
static EVENT_CATALOG: OnceLock<Vec<EventManifest>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct ItemsManifest {
    items: Vec<ItemManifest>,
}

#[derive(Debug, Deserialize)]
struct ItemManifest {
    id: String,
    kind: String,
    sink: String,
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

#[derive(Debug, Deserialize)]
struct SettlementsManifest {
    settlements: Vec<SettlementManifest>,
}

static SETTLEMENT_CATALOG: OnceLock<Vec<SettlementManifest>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct SettlementManifest {
    id: String,
    location: String,
    abundant: Vec<String>,
    scarce: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettlementSupplyProfile {
    pub(crate) abundant: Vec<String>,
    pub(crate) scarce: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RegionManifest {
    region_id: String,
    calendar: CalendarManifest,
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
    role: String,
    resources: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RouteManifest {
    id: String,
    transport: String,
    origin: String,
    destination: String,
}

pub fn validate() -> Result<(), String> {
    let schema: ContentSchemaManifest =
        serde_json::from_str(include_str!("../../assets/data/content_schema.json"))
            .map_err(|error| format!("content schema JSON is invalid: {error}"))?;
    let game_config: GameConfigManifest =
        serde_json::from_str(include_str!("../../assets/data/game_config.json"))
            .map_err(|error| format!("game config JSON is invalid: {error}"))?;
    let actions: Vec<ActionManifest> =
        serde_json::from_str(include_str!("../../assets/data/actions.json"))
            .map_err(|error| format!("actions JSON is invalid: {error}"))?;
    let crops: Vec<CropManifest> =
        serde_json::from_str(include_str!("../../assets/data/crops.json"))
            .map_err(|error| format!("crops JSON is invalid: {error}"))?;
    let events: EventsManifest =
        serde_json::from_str(include_str!("../../assets/data/events.json"))
            .map_err(|error| format!("events JSON is invalid: {error}"))?;
    let items: ItemsManifest = serde_json::from_str(include_str!("../../assets/data/items.json"))
        .map_err(|error| format!("items JSON is invalid: {error}"))?;
    let region: RegionManifest =
        serde_json::from_str(include_str!("../../assets/data/region.json"))
            .map_err(|error| format!("region JSON is invalid: {error}"))?;
    let settlements: SettlementsManifest =
        serde_json::from_str(include_str!("../../assets/data/settlements.json"))
            .map_err(|error| format!("settlements JSON is invalid: {error}"))?;

    validate_schema(&schema)?;
    validate_game_config(&game_config, &region)?;
    validate_actions(&actions)?;
    validate_crops(&crops)?;
    validate_events(&events)?;
    validate_items(&items)?;
    validate_region(&region)?;
    validate_settlements(&settlements, &region)?;
    crate::repository::validate_skill_catalog()?;
    Ok(())
}

pub(crate) fn crop_kind_for_seed(seed_index: u32) -> CropKind {
    let crops = CROP_CATALOG.get_or_init(|| {
        let crops: Vec<CropManifest> =
            serde_json::from_str(CROPS_JSON).expect("crops content JSON must be valid");
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
        let events: EventsManifest =
            serde_json::from_str(EVENTS_JSON).expect("events content JSON must be valid");
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

pub(crate) fn settlement_supply_profile(settlement_id: &str) -> SettlementSupplyProfile {
    let settlements = SETTLEMENT_CATALOG.get_or_init(|| {
        let settlements: SettlementsManifest =
            serde_json::from_str(include_str!("../../assets/data/settlements.json"))
                .expect("settlements content JSON must be valid");
        validate_id_list(
            "settlement",
            settlements
                .settlements
                .iter()
                .map(|settlement| settlement.id.as_str())
                .collect(),
        )
        .expect("settlements content IDs must be valid");
        settlements.settlements
    });
    let settlement = settlements
        .iter()
        .find(|settlement| settlement.id == settlement_id)
        .expect("validated settlement catalog must contain the requested settlement");
    SettlementSupplyProfile {
        abundant: settlement.abundant.clone(),
        scarce: settlement.scarce.clone(),
    }
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
    if items.items.is_empty()
        || items
            .items
            .iter()
            .any(|item| item.kind.trim().is_empty() || item.sink.trim().is_empty())
    {
        return Err("items need IDs, kinds, and economic sinks".to_owned());
    }
    Ok(())
}

fn validate_region(region: &RegionManifest) -> Result<(), String> {
    if region.region_id != "hearthlands" || region.calendar.day_seconds == 0 {
        return Err("region needs the authoritative ID and a positive day length".to_owned());
    }
    validate_id_list(
        "location",
        region
            .locations
            .iter()
            .map(|location| location.id.as_str())
            .collect(),
    )?;
    validate_id_list(
        "route",
        region
            .routes
            .iter()
            .map(|route| route.id.as_str())
            .collect(),
    )?;
    if region.locations.len() < 3
        || region.locations.iter().any(|location| {
            location.role.trim().is_empty()
                || location.resources.is_empty()
                || location
                    .resources
                    .iter()
                    .any(|resource| resource.trim().is_empty())
        })
        || region.routes.iter().any(|route| {
            route.transport.trim().is_empty()
                || route.origin.trim().is_empty()
                || route.destination.trim().is_empty()
        })
    {
        return Err("region locations and routes contain incomplete records".to_owned());
    }
    if region.calendar.season_days == 0
        || region.calendar.seasons.len() != 4
        || region
            .calendar
            .seasons
            .iter()
            .any(|season| season.trim().is_empty())
        || region.calendar.seasons.iter().collect::<HashSet<_>>().len()
            != region.calendar.seasons.len()
        || region.calendar.year_days
            != region
                .calendar
                .season_days
                .saturating_mul(region.calendar.seasons.len() as u32)
    {
        return Err("region calendar must define four compatible non-zero seasons".to_owned());
    }
    let location_ids: HashSet<&str> = region
        .locations
        .iter()
        .map(|location| location.id.as_str())
        .collect();
    if region.routes.iter().any(|route| {
        !location_ids.contains(route.origin.as_str())
            || !location_ids.contains(route.destination.as_str())
    }) {
        return Err("region route references an unknown location".to_owned());
    }
    Ok(())
}

fn validate_settlements(
    settlements: &SettlementsManifest,
    region: &RegionManifest,
) -> Result<(), String> {
    validate_id_list(
        "settlement",
        settlements
            .settlements
            .iter()
            .map(|settlement| settlement.id.as_str())
            .collect(),
    )?;
    let location_ids: HashSet<&str> = region
        .locations
        .iter()
        .map(|location| location.id.as_str())
        .collect();
    if settlements.settlements.is_empty()
        || settlements.settlements.iter().any(|settlement| {
            settlement.location.trim().is_empty()
                || settlement.abundant.is_empty()
                || settlement.scarce.is_empty()
                || !location_ids.contains(settlement.location.as_str())
        })
    {
        return Err("settlements need unique IDs, known locations, and supply notes".to_owned());
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
