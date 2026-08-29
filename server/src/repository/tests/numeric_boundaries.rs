use super::repo;

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
