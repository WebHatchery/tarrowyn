use super::*;
use crate::config::ServerConfig;
use crate::repository::models::{RepositoryState, StoredState};

#[test]
fn fresh_state_exposes_shared_crude_tools_and_bounded_nodes() {
    let activity = fresh();

    assert_eq!(activity.resource_nodes.len(), 2);
    assert!(activity.resource_nodes.iter().all(|node| {
        node.recovery_interval_ticks > 0
            && node
                .deposits
                .iter()
                .all(|deposit| deposit.remaining == deposit.capacity && deposit.capacity > 0)
    }));
    assert_eq!(activity.crude_tool_access.len(), 1);
    assert!(activity.crude_tool_access[0].available_to_all);
}

#[test]
fn depleted_resources_recover_deterministically_without_exceeding_capacity() {
    let mut state = RepositoryState::fresh(&ServerConfig::default());
    let timber = &mut state.foundation_activity.resource_nodes[0].deposits[0];
    timber.remaining = 2;
    state.tick = 18;

    recover_resource_nodes(&mut state);

    let node = &state.foundation_activity.resource_nodes[0];
    assert_eq!(node.deposits[0].remaining, 5);
    assert_eq!(node.last_recovered_tick, 18);

    state.tick = 600;
    recover_resource_nodes(&mut state);
    assert_eq!(
        state.foundation_activity.resource_nodes[0].deposits[0].remaining,
        12
    );
}

#[test]
fn foundation_activity_survives_the_stored_state_boundary() {
    let config = ServerConfig::default();
    let mut state = RepositoryState::fresh(&config);
    state.foundation_activity.resource_nodes[1].deposits[0].remaining = 3;
    state.foundation_activity.resource_nodes[1].last_recovered_tick = 12;

    let encoded = serde_json::to_vec(&state.to_stored()).unwrap();
    let stored: StoredState = serde_json::from_slice(&encoded).unwrap();
    let restored = RepositoryState::from_stored(stored, &config);

    assert_eq!(restored.foundation_activity, state.foundation_activity);
}
