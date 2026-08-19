//! Runtime validation for the data-driven regional content manifests.

use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
struct RegionManifest {
    region_id: String,
    calendar: CalendarManifest,
    locations: Vec<IdRecord>,
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
struct IdRecord {
    id: String,
}

#[derive(Debug, Deserialize)]
struct RouteManifest {
    id: String,
    origin: String,
    destination: String,
}

pub fn validate() -> Result<(), String> {
    let manifest: RegionManifest =
        serde_json::from_str(include_str!("../../assets/data/region.json"))
            .map_err(|error| format!("region content JSON is invalid: {error}"))?;
    if manifest.region_id != "hearthlands" {
        return Err("the authoritative region ID must be hearthlands".to_owned());
    }
    if manifest.calendar.day_seconds == 0
        || manifest.calendar.season_days == 0
        || manifest.calendar.year_days != manifest.calendar.season_days * 4
        || manifest.calendar.seasons.len() != 4
    {
        return Err("calendar content must define four non-zero, compatible seasons".to_owned());
    }
    let location_ids: HashSet<&str> = manifest
        .locations
        .iter()
        .map(|location| location.id.as_str())
        .collect();
    if location_ids.len() < 3 || location_ids.len() != manifest.locations.len() {
        return Err("region content needs at least three unique locations".to_owned());
    }
    let route_ids: HashSet<&str> = manifest
        .routes
        .iter()
        .map(|route| route.id.as_str())
        .collect();
    if route_ids.len() != manifest.routes.len() {
        return Err("region content routes must have unique IDs".to_owned());
    }
    if manifest.routes.iter().any(|route| {
        !location_ids.contains(route.origin.as_str())
            || !location_ids.contains(route.destination.as_str())
    }) {
        return Err("region content contains a route to an unknown location".to_owned());
    }
    Ok(())
}
