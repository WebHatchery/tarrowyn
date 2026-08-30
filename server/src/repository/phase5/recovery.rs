//! Travel recovery owned by the regional authority.

use super::super::models::RepositoryState;
use super::*;
use tarrowyn_protocol::TravelStatus;

pub fn clear_stuck_travel(
    state: &mut RepositoryState,
    target_key: Option<String>,
) -> (bool, String, Option<String>) {
    let Some(target_key) = target_key else {
        return (
            false,
            String::new(),
            Some("The target account is not present.".to_owned()),
        );
    };
    let Some(travel) = state.phase5.travel.remove(&target_key) else {
        return (
            false,
            String::new(),
            Some("The target account has no recorded journey to clear.".to_owned()),
        );
    };
    if !matches!(
        travel.status,
        TravelStatus::Travelling | TravelStatus::Interrupted | TravelStatus::Recovering
    ) {
        state.phase5.travel.insert(target_key, travel);
        return (
            false,
            String::new(),
            Some("Only an active or interrupted journey can be cleared.".to_owned()),
        );
    }
    let origin = travel.origin_location_id;
    let origin_position = location_position(state, &origin);
    let online = state
        .sessions
        .values()
        .any(|session| session.identity_key == target_key);
    let presence_event = if let Some(identity) = state.identities.get_mut(&target_key) {
        identity.position = origin_position;
        Some(tarrowyn_protocol::WorldEvent::Presence(
            super::super::presence(identity, state.tick, online),
        ))
    } else {
        None
    };
    if let Some(presence_event) = presence_event {
        super::super::push_event(state, presence_event);
    }
    record_regional(
        state,
        &[origin.as_str()],
        "journey support repair",
        "Support cleared a stuck journey at its recorded origin with cargo and rewards preserved.",
    );
    (
        true,
        "Stuck travel was cleared at its recorded origin with cargo and rewards preserved."
            .to_owned(),
        None,
    )
}
