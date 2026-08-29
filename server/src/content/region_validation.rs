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
