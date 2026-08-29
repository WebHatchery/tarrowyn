use super::super::models::RepositoryState;
use std::collections::HashSet;
use tarrowyn_protocol::RegionalEventStage;

const MAX_EVENT_TEXT_CHARS: usize = 512;
const MAX_EVENT_ID_CHARS: usize = 160;

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

fn bounded(value: &str) -> bool {
    bounded_with_limit(value, MAX_EVENT_TEXT_CHARS)
}

fn bounded_with_limit(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}
