use super::super::super::WorldRepository;
use super::guest;
use crate::ServerConfig;
use tarrowyn_protocol::{RegionalEvent, RegionalEventStage};

#[test]
fn regional_event_history_is_bounded_and_rejects_stale_cursors() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase5-event-retention");
    let (floor, current) = {
        let mut state = repository.state.lock().expect("repository lock");
        for index in 0..=super::super::state::MAX_REGIONAL_EVENTS {
            state.cursor = state.cursor.saturating_add(1);
            let tick = state.tick;
            let cursor = state.cursor;
            state.phase5.events.push(RegionalEvent {
                event_id: format!("retained-event-{index}"),
                title: "A retained regional event".to_owned(),
                kind: "weather".to_owned(),
                stage: RegionalEventStage::Aftermath,
                affected_location_ids: vec!["hearth".to_owned()],
                effects: vec!["The road remembers".to_owned()],
                cause: "retention test".to_owned(),
                intervention_options: vec!["watch".to_owned()],
                chosen_intervention: None,
                outcome: Some("The record remains visible".to_owned()),
                started_tick: tick,
                updated_tick: tick,
                cursor,
            });
        }
        super::super::trim_event_history(&mut state.phase5);
        (state.phase5.event_history_floor, state.cursor)
    };

    assert_eq!(
        current,
        floor + super::super::state::MAX_REGIONAL_EVENTS as u64
    );
    let stale = repository
        .events_region(&session.account_token, floor - 1)
        .expect_err("a cursor before regional retention must fail closed");
    assert_eq!(stale.status, 409);
    assert_eq!(stale.error.code, "cursor_stale");

    let boundary = repository
        .events_region(&session.account_token, floor)
        .expect("the retention boundary remains readable")
        .data;
    assert_eq!(
        boundary.events.len(),
        super::super::state::MAX_REGIONAL_EVENTS
    );
    assert!(boundary.events.iter().all(|event| event.cursor > floor));
}
