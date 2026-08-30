use super::super::models::RepositoryState;
use crate::config::ServerConfig;
use std::collections::HashSet;
use tarrowyn_protocol::RegionalEventStage;

const MAX_EVENT_TEXT_CHARS: usize = 512;
const MAX_EVENT_ID_CHARS: usize = 160;
const MAX_LOCATION_ID_CHARS: usize = 160;
const MAX_LOCATION_TEXT_CHARS: usize = 240;
const MAX_HOUSEHOLD_TEXT_CHARS: usize = 240;
const MAX_HOUSEHOLD_HISTORY: usize = 64;
const MAX_ROUTE_ID_CHARS: usize = 160;
const MAX_ROUTE_TEXT_CHARS: usize = 240;
const MAX_SETTLEMENT_ID_CHARS: usize = 160;
const MAX_SETTLEMENT_TEXT_CHARS: usize = 240;
const MAX_TRAVEL_TEXT_CHARS: usize = 160;

pub(super) fn ok(state: &RepositoryState, config: &ServerConfig) -> bool {
    let location_ids: HashSet<&str> = state
        .phase5
        .locations
        .iter()
        .map(|location| location.location_id.as_str())
        .collect();
    state.phase5.cursor <= state.cursor
        && state.phase5.event_history_floor <= state.phase5.cursor
        && state.phase5.events.len() <= super::super::MAX_EVENTS
        && state
            .phase5
            .locations
            .iter()
            .all(|location| location_ok(location, config))
        && state
            .phase5
            .routes
            .iter()
            .all(|route| route_ok(route, &location_ids, state.tick))
        && route_action_cooldowns_ok(state)
        && unique_event_ids(state)
        && state.phase5.events.iter().all(|event| {
            event.cursor > state.phase5.event_history_floor
                && event_ok(event, &location_ids, state.cursor, state.tick)
        })
        && !state.phase5.households.is_empty()
        && unique_household_ids(state)
        && state
            .phase5
            .households
            .iter()
            .all(|household| household_ok(household, &location_ids, state.tick))
        && state
            .phase5
            .settlements
            .iter()
            .all(|settlement| settlement_ok(settlement, &location_ids, state.cursor, state.tick))
        && state.phase5.travel.iter().all(|(identity_key, travel)| {
            state.identities.contains_key(identity_key)
                && travel_ok(travel, &location_ids, state.tick)
        })
}

fn route_action_cooldowns_ok(state: &RepositoryState) -> bool {
    state.phase5.route_action_available_at_tick.len() <= state.phase5.routes.len()
        && state.phase5.route_action_available_at_tick.iter().all(
            |(route_id, available_at_tick)| {
                !route_id.trim().is_empty()
                    && state
                        .phase5
                        .routes
                        .iter()
                        .any(|route| route.route_id == *route_id)
                    && *available_at_tick > state.tick
            },
        )
}

fn location_ok(location: &tarrowyn_protocol::LocationRecord, config: &ServerConfig) -> bool {
    bounded_with_limit(&location.location_id, MAX_LOCATION_ID_CHARS)
        && bounded_with_limit(&location.name, MAX_LOCATION_TEXT_CHARS)
        && position_in_world(location.position, config)
        && bounded_with_limit(&location.role, MAX_LOCATION_TEXT_CHARS)
        && !location.resources.is_empty()
        && location
            .resources
            .iter()
            .all(|resource| bounded_with_limit(resource, MAX_LOCATION_TEXT_CHARS))
        && !location.services.is_empty()
        && location
            .services
            .iter()
            .all(|service| bounded_with_limit(service, MAX_LOCATION_TEXT_CHARS))
        && location.condition <= 100
        && bounded_with_limit(&location.access_note, MAX_LOCATION_TEXT_CHARS)
}

fn route_ok(
    route: &tarrowyn_protocol::RouteRecord,
    location_ids: &HashSet<&str>,
    current_tick: u64,
) -> bool {
    bounded_with_limit(&route.route_id, MAX_ROUTE_ID_CHARS)
        && bounded_with_limit(&route.name, MAX_ROUTE_TEXT_CHARS)
        && bounded_with_limit(&route.transport, MAX_ROUTE_TEXT_CHARS)
        && location_ids.contains(route.origin_location_id.as_str())
        && location_ids.contains(route.destination_location_id.as_str())
        && route.origin_location_id != route.destination_location_id
        && route.length > 0
        && route.risk_percent <= 100
        && route.condition <= 100
        && route.capacity > 0
        && route.travel_ticks > 0
        && route.repair_cost > 0
        && route.last_action_tick <= current_tick
        && bounded_with_limit(&route.note, MAX_ROUTE_TEXT_CHARS)
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
    let status_ok = match travel.status {
        tarrowyn_protocol::TravelStatus::Travelling => {
            travel.progress < 100 && current_tick < travel.eta_tick
        }
        tarrowyn_protocol::TravelStatus::Interrupted
        | tarrowyn_protocol::TravelStatus::Recovering => travel.progress < 100,
        tarrowyn_protocol::TravelStatus::Arrived => {
            travel.progress == 100 && current_tick >= travel.eta_tick
        }
        tarrowyn_protocol::TravelStatus::Idle => false,
    };
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
        && status_ok
        && travel.interruption.as_deref().is_none_or(bounded_travel)
        && travel.recovery_note.as_deref().is_none_or(bounded_travel)
        && (!matches!(travel.status, tarrowyn_protocol::TravelStatus::Interrupted)
            || (travel.interruption.is_some() && travel.recovery_note.is_some()))
}

fn settlement_ok(
    settlement: &tarrowyn_protocol::SettlementProjection,
    location_ids: &HashSet<&str>,
    current_cursor: u64,
    current_tick: u64,
) -> bool {
    bounded_with_limit(&settlement.settlement_id, MAX_SETTLEMENT_ID_CHARS)
        && bounded_with_limit(&settlement.name, MAX_SETTLEMENT_TEXT_CHARS)
        && location_ids.contains(settlement.location_id.as_str())
        && (1..=99).contains(&settlement.population)
        && settlement.food <= 100
        && settlement.safety <= 100
        && settlement.infrastructure <= 100
        && settlement.industry <= 100
        && settlement.governance <= 100
        && settlement.player_activity <= 100
        && settlement.price_index_percent > 0
        && !settlement.milestones.is_empty()
        && settlement
            .milestones
            .iter()
            .all(|value| bounded_settlement(value))
        && !settlement.vacancies.is_empty()
        && settlement
            .vacancies
            .iter()
            .all(|value| bounded_settlement(value))
        && !settlement.demand.is_empty()
        && settlement
            .demand
            .iter()
            .all(|value| bounded_settlement(value))
        && !settlement.abundant_goods.is_empty()
        && settlement
            .abundant_goods
            .iter()
            .all(|value| bounded_settlement(value))
        && !settlement.scarce_goods.is_empty()
        && settlement
            .scarce_goods
            .iter()
            .all(|value| bounded_settlement(value))
        && settlement
            .public_works
            .iter()
            .all(|value| bounded_settlement(value))
        && settlement
            .recovery_opportunity
            .as_deref()
            .is_none_or(bounded_settlement)
        && settlement_chronicle_ok(settlement, current_cursor, current_tick)
}

fn settlement_chronicle_ok(
    settlement: &tarrowyn_protocol::SettlementProjection,
    current_cursor: u64,
    current_tick: u64,
) -> bool {
    if settlement.chronicle.len() > super::super::phase5::MAX_SETTLEMENT_CHRONICLE {
        return false;
    }
    let mut event_ids = HashSet::new();
    let mut previous_cursor = 0;
    settlement.chronicle.iter().all(|entry| {
        let ordered = entry.cursor > previous_cursor;
        previous_cursor = entry.cursor;
        ordered
            && bounded_with_limit(&entry.event_id, MAX_SETTLEMENT_ID_CHARS)
            && event_ids.insert(entry.event_id.as_str())
            && bounded(&entry.kind)
            && bounded(&entry.title)
            && bounded(&entry.text)
            && entry.created_tick <= current_tick
            && entry.cursor <= current_cursor
    })
}

fn bounded(value: &str) -> bool {
    bounded_with_limit(value, MAX_EVENT_TEXT_CHARS)
}

fn bounded_with_limit(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}

fn position_in_world(position: tarrowyn_protocol::Position, config: &ServerConfig) -> bool {
    position.x >= 0
        && position.y >= 0
        && (position.x as u32) < config.world_width
        && (position.y as u32) < config.world_height
}

fn bounded_household(value: &str) -> bool {
    bounded_with_limit(value, MAX_HOUSEHOLD_TEXT_CHARS)
}

fn bounded_travel(value: &str) -> bool {
    bounded_with_limit(value, MAX_TRAVEL_TEXT_CHARS)
}

fn bounded_settlement(value: &str) -> bool {
    bounded_with_limit(value, MAX_SETTLEMENT_TEXT_CHARS)
}
