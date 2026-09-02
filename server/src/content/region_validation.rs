use super::*;

pub(super) fn validate_region(
    region: &RegionManifest,
    game_config: &GameConfigManifest,
) -> Result<(), String> {
    if region.region_id != "hearthlands" || region.calendar.day_seconds == 0 {
        return Err("region needs the authoritative ID and a positive day length".to_owned());
    }
    let farm_plot_keys: HashSet<(i32, i32)> = region
        .farm_plots
        .iter()
        .map(|position| (position.x, position.y))
        .collect();
    if region.farm_plots.is_empty()
        || farm_plot_keys.len() != region.farm_plots.len()
        || region.farm_plots.iter().any(|position| {
            position.x < 0
                || position.y < 0
                || position.x as u32 >= game_config.world_width
                || position.y as u32 >= game_config.world_height
        })
    {
        return Err("region farm plots must be unique, non-empty, and inside the world".to_owned());
    }
    if region.farm_animal_position.x < 0
        || region.farm_animal_position.y < 0
        || region.farm_animal_position.x as u32 >= game_config.world_width
        || region.farm_animal_position.y as u32 >= game_config.world_height
        || farm_plot_keys.contains(&(region.farm_animal_position.x, region.farm_animal_position.y))
        || !region
            .farm_plots
            .iter()
            .any(|plot| region.farm_animal_position.manhattan_distance(*plot) == 1)
    {
        return Err(
            "region farm animal must be inside the world and one tile from a plot".to_owned(),
        );
    }
    validate_foundation_baseline(region, game_config)?;
    if region.locations.iter().any(|location| {
        location.position.x < 0
            || location.position.y < 0
            || location.position.x as u32 >= game_config.world_width
            || location.position.y as u32 >= game_config.world_height
    }) {
        return Err("region locations must be inside the world".to_owned());
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
            location.name.trim().is_empty()
                || !matches!(
                    location.kind.as_str(),
                    "settlement" | "outpost" | "frontier"
                )
                || location.role.trim().is_empty()
                || location.resources.is_empty()
                || location
                    .resources
                    .iter()
                    .any(|resource| resource.trim().is_empty())
                || location.services.is_empty()
                || location
                    .services
                    .iter()
                    .any(|service| service.trim().is_empty())
                || location.condition > 100
                || location.access_note.trim().is_empty()
        })
        || region.routes.iter().any(|route| {
            route.name.trim().is_empty()
                || route.transport.trim().is_empty()
                || route.origin.trim().is_empty()
                || route.destination.trim().is_empty()
                || route.origin == route.destination
                || route.length == 0
                || route.risk_percent > 100
                || route.condition > 100
                || route.capacity == 0
                || route.travel_ticks == 0
                || route.repair_cost == 0
                || !matches!(
                    route.status.as_str(),
                    "operational" | "delayed" | "threatened" | "repairing" | "closed"
                )
                || route.note.trim().is_empty()
        })
    {
        return Err(
            "region locations and routes contain incomplete, distinct, or invalid records"
                .to_owned(),
        );
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
    validate_required_ids(
        "location",
        &location_ids,
        &["hearth", "whisperwood-outpost", "saltmere"],
    )?;
    if region.routes.iter().any(|route| {
        !location_ids.contains(route.origin.as_str())
            || !location_ids.contains(route.destination.as_str())
    }) {
        return Err("region route references an unknown location".to_owned());
    }
    let route_ids: HashSet<&str> = region
        .routes
        .iter()
        .map(|route| route.id.as_str())
        .collect();
    validate_required_ids(
        "route",
        &route_ids,
        &["north-pack-road", "saltmere-ferry", "watch-trail"],
    )?;
    for (route_id, origin, destination) in [
        ("north-pack-road", "hearth", "whisperwood-outpost"),
        ("saltmere-ferry", "hearth", "saltmere"),
        ("watch-trail", "whisperwood-outpost", "saltmere"),
    ] {
        let route = region
            .routes
            .iter()
            .find(|route| route.id == route_id)
            .expect("required launch route exists after validation");
        if route.origin != origin || route.destination != destination {
            return Err(format!(
                "launch route {route_id} must connect {origin} to {destination}"
            ));
        }
    }
    Ok(())
}

fn validate_foundation_baseline(
    region: &RegionManifest,
    game_config: &GameConfigManifest,
) -> Result<(), String> {
    let baseline = &region.foundation_baseline;
    if baseline.fixture_id != "first-beacon-baseline-v1"
        || baseline.schema_version != 1
        || baseline.settlement_id != "hearth-settlement"
    {
        return Err(
            "foundation baseline must keep its F0 fixture, schema, and settlement IDs".to_owned(),
        );
    }
    validate_id_list(
        "foundation landmark",
        baseline
            .landmarks
            .iter()
            .map(|landmark| landmark.id.as_str())
            .collect(),
    )?;
    validate_id_list(
        "foundation interaction",
        baseline
            .interactions
            .iter()
            .map(|interaction| interaction.id.as_str())
            .collect(),
    )?;
    let required_landmarks = [
        "first-beacon",
        "first-beacon-tents",
        "first-beacon-fire",
        "builder-mara",
        "first-beacon-noticeboard",
        "first-beacon-cache",
        "first-beacon-tool-rack",
        "first-beacon-fields",
        "whisperwood-edge",
        "first-beacon-mine",
        "first-beacon-forge",
        "storehouse-site",
    ];
    let landmark_ids: HashSet<&str> = baseline
        .landmarks
        .iter()
        .map(|landmark| landmark.id.as_str())
        .collect();
    validate_required_ids("foundation landmark", &landmark_ids, &required_landmarks)?;
    if baseline.landmarks.iter().any(|landmark| {
        landmark.kind.trim().is_empty()
            || landmark.name.trim().is_empty()
            || landmark.note.trim().is_empty()
            || !landmark.visible
            || landmark.position.x < 0
            || landmark.position.y < 0
            || landmark.position.x as u32 >= game_config.world_width
            || landmark.position.y as u32 >= game_config.world_height
    }) {
        return Err(
            "foundation landmarks must be visible, complete, and inside the world".to_owned(),
        );
    }
    let beacon = baseline
        .landmarks
        .iter()
        .find(|landmark| landmark.id == "first-beacon")
        .expect("required beacon exists after validation");
    if !beacon.permanent || beacon.position != (Position { x: 8, y: 6 }) {
        return Err(
            "the permanent First Beacon must remain at the authoritative arrival point".to_owned(),
        );
    }
    if baseline.interactions.iter().any(|interaction| {
        interaction.action.trim().is_empty()
            || interaction.authority != "server"
            || interaction.note.trim().is_empty()
            || !landmark_ids.contains(interaction.landmark_id.as_str())
    }) {
        return Err(
            "foundation interactions must be complete server-owned landmark references".to_owned(),
        );
    }
    if baseline.interactions.len() != baseline.landmarks.len()
        || baseline.landmarks.iter().any(|landmark| {
            !baseline
                .interactions
                .iter()
                .any(|interaction| interaction.landmark_id == landmark.id)
        })
    {
        return Err("every foundation landmark must have an interaction record".to_owned());
    }
    Ok(())
}
