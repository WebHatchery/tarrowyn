use super::repo;
use crate::config::ServerConfig;
use crate::repository::models::RepositoryState;
use crate::repository::WorldRepository;
use tarrowyn_protocol::GuestSessionRequest;

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

#[test]
fn extreme_clock_increment_completes_without_unbounded_catch_up() {
    let repository = WorldRepository::new(ServerConfig {
        day_length_seconds: 1.0,
        world_seconds_per_tick: f32::MAX,
        ..ServerConfig::default()
    });

    repository.tick();

    let state = repository.state.lock().unwrap();
    assert_eq!(state.tick, 1);
    assert_eq!(state.clock.day, u32::MAX);
    assert!(state.clock.seconds < state.clock.day_length_seconds);
}

#[test]
fn guest_identity_and_session_ids_stay_at_the_numeric_ceiling() {
    let repository = repo();
    {
        let mut state = repository.state.lock().unwrap();
        state.next_guest = u64::MAX;
        state.next_token = u64::MAX;
    }

    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("guest-id-ceiling".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;

    assert_eq!(session.account_id, format!("dev-account-{}", u64::MAX));
    assert_eq!(session.character_id, format!("dev-character-{}", u64::MAX));
    assert_eq!(session.account_token, format!("dev-session-{}", u64::MAX));
    let state = repository.state.lock().unwrap();
    assert_eq!(state.next_guest, u64::MAX);
    assert_eq!(state.next_token, u64::MAX);
}
