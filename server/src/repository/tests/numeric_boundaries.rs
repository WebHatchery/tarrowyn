use super::repo;
use crate::config::ServerConfig;
use crate::repository::models::RepositoryState;

#[test]
fn world_tick_and_day_hold_at_their_numeric_ceiling() {
    let repository = repo();
    let mut state = repository.state.lock().unwrap();
    state.tick = u64::MAX;
    state.clock.day = u32::MAX;
    state.clock.seconds = state.clock.day_length_seconds - 0.1;
    drop(state);

    repository.tick();

    let state = repository.state.lock().unwrap();
    assert_eq!(state.tick, u64::MAX);
    assert_eq!(state.clock.day, u32::MAX);
    assert!(state.clock.seconds < state.clock.day_length_seconds);
}

#[test]
fn restored_clock_seconds_stay_inside_the_configured_day() {
    let repository = repo();
    let mut stored = repository.state.lock().unwrap().to_stored();
    stored.clock.seconds = stored.clock.day_length_seconds * 3.0 + 1.25;

    let restored = RepositoryState::from_stored(stored, &ServerConfig::default());

    assert_eq!(restored.clock.seconds, 1.25);
}
