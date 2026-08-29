use super::super::{push_event, MAX_EVENTS};
use super::{guest, repo};
use tarrowyn_protocol::WorldEvent;

#[test]
fn event_streams_reject_cursors_before_the_retained_window() {
    let repo = repo();
    let session = guest(&repo, "stale-cursor");
    let (oldest, current) = {
        let mut state = repo.state.lock().expect("world repository lock poisoned");
        for _ in 0..=MAX_EVENTS {
            let clock = state.clock.clone();
            push_event(&mut state, WorldEvent::Clock(clock));
        }
        (
            state.events.front().expect("retained event window").cursor,
            state.cursor,
        )
    };
    assert_eq!(current - oldest + 1, MAX_EVENTS as u64);
    assert!(oldest > 1);

    let stale = repo
        .events(&session.account_token, oldest - 2)
        .expect_err("a cursor before retained history must fail closed");
    assert_eq!(stale.status, 409);
    assert_eq!(stale.error.code, "cursor_stale");
    let regional_stale = repo
        .events_region(&session.account_token, oldest - 2)
        .expect_err("regional cursors share the retained history boundary");
    assert_eq!(regional_stale.status, 409);
    assert_eq!(regional_stale.error.code, "cursor_stale");

    let boundary = repo
        .events(&session.account_token, oldest - 1)
        .expect("the immediately preceding cursor remains valid")
        .data;
    assert_eq!(
        boundary.events.first().expect("boundary event").cursor,
        oldest
    );
    assert_eq!(boundary.cursor, current);
}

#[test]
fn event_cursor_stays_at_the_numeric_ceiling() {
    let repository = repo();
    let mut state = repository.state.lock().expect("world repository lock");
    state.cursor = u64::MAX;
    let clock = state.clock.clone();

    super::super::push_event(&mut state, WorldEvent::Clock(clock));

    assert_eq!(state.cursor, u64::MAX);
    assert_eq!(state.events.back().expect("ceiling event").cursor, u64::MAX);
}
