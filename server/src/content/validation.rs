use super::{
    ActionManifest, ContentSchemaManifest, CropManifest, EventsManifest, GameConfigManifest,
    ItemsManifest, RegionManifest,
};
use macroquad_toolkit::data_loader::parse_json_labeled;
use std::collections::HashSet;

pub(super) fn validate() -> Result<(), String> {
    let schema: ContentSchemaManifest = parse_json_labeled(
        "content_schema.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/content_schema.json"),
    )
    .map_err(|error| format!("content schema JSON is invalid: {error}"))?;
    let game_config: GameConfigManifest =
        parse_json_labeled("game_config.json", super::GAME_CONFIG_JSON)
            .map_err(|error| format!("game config JSON is invalid: {error}"))?;
    let actions: Vec<ActionManifest> = parse_json_labeled(
        "actions.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/actions.json"),
    )
    .map_err(|error| format!("actions JSON is invalid: {error}"))?;
    let crops: Vec<CropManifest> = parse_json_labeled(
        "crops.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/crops.json"),
    )
    .map_err(|error| format!("crops JSON is invalid: {error}"))?;
    let events: EventsManifest = parse_json_labeled(
        "events.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/events.json"),
    )
    .map_err(|error| format!("events JSON is invalid: {error}"))?;
    let items: ItemsManifest = parse_json_labeled(
        "items.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/items.json"),
    )
    .map_err(|error| format!("items JSON is invalid: {error}"))?;
    let region: RegionManifest = parse_json_labeled(
        "region.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/region.json"),
    )
    .map_err(|error| format!("region JSON is invalid: {error}"))?;

    validate_schema(&schema)?;
    validate_game_config(&game_config, &region)?;
    validate_actions(&actions)?;
    validate_crops(&crops)?;
    super::frontier::validate(game_config.world_width, game_config.world_height)?;
    validate_events(&events, &region)?;
    validate_items(&items)?;
    super::region_validation::validate_region(&region, &game_config)?;
    super::households::validate(&region)?;
    super::recipes::validate()?;
    let item_ids = items
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<HashSet<_>>();
    super::settlements::validate(&region, &game_config, &item_ids)?;
    super::npcs::validate()?;
    crate::repository::validate_skill_catalog()?;
    Ok(())
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
    for required in super::REQUIRED_MANIFESTS {
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

pub(super) fn validate_actions(actions: &[ActionManifest]) -> Result<(), String> {
    validate_id_list(
        "action",
        actions.iter().map(|action| action.id.as_str()).collect(),
    )?;
    if actions.is_empty()
        || actions.iter().any(|action| {
            action.name.trim().is_empty()
                || action.description.trim().is_empty()
                || !matches!(
                    action.kind.as_str(),
                    "plant" | "tend" | "harvest" | "listen"
                )
        })
    {
        return Err("actions need IDs, names, descriptions, and supported kinds".to_owned());
    }
    for (action_id, action_kind) in [
        ("plant", "plant"),
        ("tend", "tend"),
        ("harvest", "harvest"),
        ("listen", "listen"),
    ] {
        let Some(action) = actions.iter().find(|action| action.id == action_id) else {
            return Err(format!("actions are missing the launch action {action_id}"));
        };
        if action.kind != action_kind {
            return Err(format!(
                "launch action {action_id} must use kind {action_kind}"
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_crops(crops: &[CropManifest]) -> Result<(), String> {
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

pub(super) fn validate_events(
    events: &EventsManifest,
    region: &RegionManifest,
) -> Result<(), String> {
    validate_id_list(
        "event",
        events
            .events
            .iter()
            .map(|event| event.id.as_str())
            .collect(),
    )?;
    let event_ids: HashSet<&str> = events
        .events
        .iter()
        .map(|event| event.id.as_str())
        .collect();
    validate_required_ids("event", &event_ids, &["river-thaw"])?;
    if events.events.is_empty()
        || events.events.iter().any(|event| {
            event.title.trim().is_empty()
                || event.kind.trim().is_empty()
                || event.stages.is_empty()
                || event.affected_systems.is_empty()
                || event
                    .affected_systems
                    .iter()
                    .any(|system| system.trim().is_empty())
                || event.affected_locations.is_empty()
                || event.effects.is_empty()
                || event.cause.trim().is_empty()
                || event.intervention_options.is_empty()
                || event.effects.iter().any(|effect| effect.trim().is_empty())
                || event
                    .intervention_options
                    .iter()
                    .any(|option| option.trim().is_empty())
                || event
                    .intervention_options
                    .iter()
                    .any(|option| !super::SUPPORTED_EVENT_INTERVENTIONS.contains(&option.as_str()))
                || event.stages.iter().any(|stage| {
                    !matches!(
                        stage.as_str(),
                        "signal" | "escalation" | "intervention" | "resolution" | "aftermath"
                    )
                })
                || event.affected_locations.iter().any(|location| {
                    location.trim().is_empty()
                        || !region
                            .locations
                            .iter()
                            .any(|candidate| candidate.id == *location)
                })
        })
    {
        return Err(
            "events need IDs, kinds, known stages, supported interventions, affected systems, and known locations"
                .to_owned(),
        );
    }
    for event in &events.events {
        let mut locations = HashSet::new();
        if event
            .affected_locations
            .iter()
            .any(|location| !locations.insert(location.as_str()))
        {
            return Err(format!(
                "event {} cannot repeat an affected location",
                event.id
            ));
        }
        if event
            .intervention_options
            .iter()
            .any(|option| !intervention_scope_is_valid(option, &event.affected_locations))
        {
            return Err(format!(
                "event {} must include the affected location for each intervention",
                event.id
            ));
        }
    }
    Ok(())
}

fn intervention_scope_is_valid(intervention: &str, affected_locations: &[String]) -> bool {
    let required_locations = match intervention {
        "repair ferry markers" => ["hearth", "saltmere"].as_slice(),
        "escort the grain caravan" => ["hearth", "whisperwood-outpost"].as_slice(),
        "open the frontier storehouse" => ["whisperwood-outpost"].as_slice(),
        _ => return true,
    };
    required_locations.iter().any(|required| {
        affected_locations
            .iter()
            .any(|affected| affected == required)
    })
}

pub(super) fn validate_items(items: &ItemsManifest) -> Result<(), String> {
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

pub(super) fn validate_required_ids(
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

pub(super) fn validate_id_list(label: &str, ids: Vec<&str>) -> Result<(), String> {
    if ids.is_empty()
        || ids.iter().any(|id| {
            id.trim().is_empty()
                || id.chars().count() > super::MAX_CONTENT_ID_CHARS
                || id.chars().any(char::is_control)
        })
        || ids.iter().collect::<HashSet<_>>().len() != ids.len()
    {
        return Err(format!("{label} IDs must be unique and non-empty"));
    }
    Ok(())
}
