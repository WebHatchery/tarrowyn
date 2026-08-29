use super::super::models::RepositoryState;
use std::collections::HashSet;
use tarrowyn_protocol::RegionalEventStage;

const MAX_EVENT_TEXT_CHARS: usize = 512;
const MAX_EVENT_ID_CHARS: usize = 160;
const MAX_HOUSEHOLD_TEXT_CHARS: usize = 240;
const MAX_HOUSEHOLD_HISTORY: usize = 64;
const MAX_TRAVEL_TEXT_CHARS: usize = 160;

pub(super) fn ok(state: &RepositoryState) -> bool {
    let location_ids: HashSet<&str> = state
        .phase5
        .locations
        .iter()
        .map(|location| location.location_id.as_str())
        .collect();
    state.phase5.cursor <= state.cursor
        && state.phase5.event_history_floor <= state.phase5.cursor
        && state.phase5.events.len() <= super::super::MAX_EVENTS
        && unique_event_ids(state)
        && state
            .phase5
            .events
            .iter()
            .all(|event| event_ok(event, &location_ids, state.cursor, state.tick))
        && !state.phase5.households.is_empty()
        && unique_household_ids(state)
        && state
            .phase5
            .households
            .iter()
            .all(|household| household_ok(household, &location_ids, state.tick))
        && state.phase5.travel.iter().all(|(identity_key, travel)| {
            state.identities.contains_key(identity_key)
                && travel_ok(travel, &location_ids, state.tick)
        })
}

fn unique_event_ids(state: &RepositoryState) -> bool {
    let mut ids = HashSet::new();
    state
        .phase5
        .events
        .iter()
        .all(|event| ids.insert(event.event_id.as_str()))
}

fn event_ok(
    event: &tarrowyn_protocol::RegionalEvent,
    location_ids: &HashSet<&str>,
    current_cursor: u64,
    current_tick: u64,
) -> bool {
    let locations_unique = {
        let mut locations = HashSet::new();
        event
            .affected_location_ids
            .iter()
            .all(|location| locations.insert(location.as_str()))
    };
    let options_unique = {
        let mut options = HashSet::new();
        event
            .intervention_options
            .iter()
            .all(|option| bounded(option) && options.insert(option.as_str()))
    };
    let lifecycle_ok = match event.stage {
        RegionalEventStage::Signal | RegionalEventStage::Escalation => {
            event.chosen_intervention.is_none() && event.outcome.is_none()
        }
        RegionalEventStage::Intervention => {
            event.chosen_intervention.is_some() && event.outcome.is_none()
        }
        RegionalEventStage::Resolution | RegionalEventStage::Aftermath => {
            event.chosen_intervention.is_some() && event.outcome.is_some()
        }
    };
    bounded_with_limit(&event.event_id, MAX_EVENT_ID_CHARS)
        && bounded(&event.title)
        && bounded(&event.kind)
        && !event.affected_location_ids.is_empty()
        && locations_unique
        && event
            .affected_location_ids
            .iter()
            .all(|location| location_ids.contains(location.as_str()))
        && !event.effects.is_empty()
        && event.effects.iter().all(|effect| bounded(effect))
        && bounded(&event.cause)
        && !event.intervention_options.is_empty()
        && options_unique
        && event.chosen_intervention.as_deref().is_none_or(|choice| {
            event
                .intervention_options
                .iter()
                .any(|option| option == choice)
        })
        && event.outcome.as_deref().is_none_or(bounded)
        && lifecycle_ok
        && event.started_tick <= event.updated_tick
        && event.updated_tick <= current_tick
        && event.cursor > 0
        && event.cursor <= current_cursor
}

fn unique_household_ids(state: &RepositoryState) -> bool {
    let mut ids = HashSet::new();
    state
        .phase5
        .households
        .iter()
        .all(|household| ids.insert(household.household_id.as_str()))
}

fn household_ok(
    household: &tarrowyn_protocol::RegionalHousehold,
    location_ids: &HashSet<&str>,
    current_tick: u64,
) -> bool {
    let timeline_ok = match household.status.as_str() {
        "considering" => household.departure_tick.is_none() && household.arrival_tick.is_none(),
        "travelling" => household.departure_tick.is_some() && household.arrival_tick.is_none(),
        "arrived" => household
            .departure_tick
            .zip(household.arrival_tick)
            .is_some_and(|(departure, arrival)| departure <= arrival),
        _ => false,
    };
    bounded_household(&household.household_id)
        && bounded_household(&household.household_name)
        && location_ids.contains(household.origin_location_id.as_str())
        && household
            .destination_location_id
            .as_deref()
            .is_none_or(|location| location_ids.contains(location))
        && bounded_household(&household.status)
        && bounded_household(&household.reason)
        && bounded_household(&household.service)
        && !household.history.is_empty()
        && household.history.len() <= MAX_HOUSEHOLD_HISTORY
        && household
            .history
            .iter()
            .all(|entry| bounded_household(entry))
        && household
            .departure_tick
            .is_none_or(|departure| departure <= current_tick)
        && household
            .arrival_tick
            .is_none_or(|arrival| arrival <= current_tick)
        && timeline_ok
}

fn travel_ok(
    travel: &tarrowyn_protocol::TravelState,
    location_ids: &HashSet<&str>,
    current_tick: u64,
) -> bool {
    bounded_travel(&travel.travel_id)
        && bounded_travel(&travel.route_id)
        && bounded_travel(&travel.origin_location_id)
        && bounded_travel(&travel.destination_location_id)
        && travel.origin_location_id != travel.destination_location_id
        && location_ids.contains(travel.origin_location_id.as_str())
        && location_ids.contains(travel.destination_location_id.as_str())
        && travel.departure_tick < travel.eta_tick
        && travel.eta_tick > 0
        && travel.departure_tick <= current_tick
        && travel.progress <= 100
        && travel.risk_percent <= 100
        && !matches!(travel.status, tarrowyn_protocol::TravelStatus::Idle)
        && travel.interruption.as_deref().is_none_or(bounded_travel)
        && travel.recovery_note.as_deref().is_none_or(bounded_travel)
        && (!matches!(travel.status, tarrowyn_protocol::TravelStatus::Interrupted)
            || (travel.interruption.is_some() && travel.recovery_note.is_some()))
}

fn bounded(value: &str) -> bool {
    bounded_with_limit(value, MAX_EVENT_TEXT_CHARS)
}

fn bounded_with_limit(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}

fn bounded_household(value: &str) -> bool {
    bounded_with_limit(value, MAX_HOUSEHOLD_TEXT_CHARS)
}

fn bounded_travel(value: &str) -> bool {
    bounded_with_limit(value, MAX_TRAVEL_TEXT_CHARS)
}
