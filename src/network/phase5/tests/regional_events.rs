use super::*;

#[test]
fn event_button_waits_during_the_resolution_window() {
    let mut client = Phase5Client::new();
    client.events = Some(RegionalEventsResponse {
        events: vec![regional_event(
            "event-resolution",
            tarrowyn_protocol::RegionalEventStage::Resolution,
            1,
        )],
        cursor: 1,
    });

    client.queue_cycle("region-event");
    assert!(!client.event_command_pending());
    assert!(client.commands.is_empty());
}

#[test]
fn event_button_uses_the_server_listed_intervention() {
    let mut client = Phase5Client::new();
    let mut event = regional_event(
        "event-intervention",
        tarrowyn_protocol::RegionalEventStage::Escalation,
        1,
    );
    event.intervention_options = vec![
        "protect grain stores".to_owned(),
        "close the ford".to_owned(),
    ];
    client.events = Some(RegionalEventsResponse {
        events: vec![event],
        cursor: 1,
    });

    client.queue_cycle("region-event");
    assert!(client.event_command_pending());
    let Some(Phase5Command::Event(request)) = client.commands.pop_front() else {
        panic!("an active event should queue an intervention");
    };
    assert_eq!(request.action, RegionalEventAction::Intervene);
    assert_eq!(
        request.intervention.as_deref(),
        Some("protect grain stores")
    );
    assert!(!client.event_command_pending());
}

#[test]
fn event_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = Phase5Client::new();
    let request = tarrowyn_protocol::RegionalEventRequest {
        request_id: "event-queued".to_owned(),
        action: RegionalEventAction::Seed,
        event_id: None,
        intervention: None,
    };
    client
        .commands
        .push_back(Phase5Command::Event(request.clone()));

    assert!(client.event_command_pending());
    assert!(!client.queue_cycle("region-event"));
    assert!(!client.queue_event_intervention(
        "event-choice-duplicate".to_owned(),
        "protect grain stores".to_owned(),
    ));

    client.commands.clear();
    client.in_flight_command = Some(Phase5Command::Event(request));
    assert!(client.event_command_pending());
    assert!(!client.queue_cycle("region-event"));
}

#[test]
fn selected_event_choice_queues_the_exact_visible_intervention() {
    let mut client = Phase5Client::new();
    let mut event = regional_event(
        "event-selected",
        tarrowyn_protocol::RegionalEventStage::Escalation,
        1,
    );
    event.intervention_options = vec![
        "protect grain stores".to_owned(),
        "close the ford".to_owned(),
    ];
    client.events = Some(RegionalEventsResponse {
        events: vec![event],
        cursor: 1,
    });

    assert!(
        client.queue_event_intervention("selected-event-1".to_owned(), "close the ford".to_owned())
    );
    assert!(client.event_command_pending());
    let Some(Phase5Command::Event(request)) = client.commands.pop_front() else {
        panic!("a selected event choice should queue an intervention");
    };
    assert_eq!(request.intervention.as_deref(), Some("close the ford"));
    assert!(!client.event_command_pending());
}

#[test]
fn regional_event_cursor_merges_updates_without_dropping_known_events() {
    let mut current = Some(RegionalEventsResponse {
        events: vec![regional_event(
            "event-1",
            tarrowyn_protocol::RegionalEventStage::Signal,
            4,
        )],
        cursor: 4,
    });
    merge_regional_events(
        &mut current,
        RegionalEventsResponse {
            events: vec![
                regional_event(
                    "event-1",
                    tarrowyn_protocol::RegionalEventStage::Escalation,
                    7,
                ),
                regional_event("event-2", tarrowyn_protocol::RegionalEventStage::Signal, 8),
            ],
            cursor: 8,
        },
    );

    let current = current.expect("regional events should remain cached");
    assert_eq!(current.cursor, 8);
    assert_eq!(current.events.len(), 2);
    assert_eq!(current.events[0].event_id, "event-1");
    assert_eq!(
        current.events[0].stage,
        tarrowyn_protocol::RegionalEventStage::Escalation
    );
    assert_eq!(current.events[1].event_id, "event-2");
}

#[test]
fn regional_event_merge_ignores_stale_updates_and_cursor_regressions() {
    let mut current = Some(RegionalEventsResponse {
        events: vec![regional_event(
            "event-1",
            tarrowyn_protocol::RegionalEventStage::Escalation,
            7,
        )],
        cursor: 8,
    });

    merge_regional_events(
        &mut current,
        RegionalEventsResponse {
            events: vec![regional_event(
                "event-1",
                tarrowyn_protocol::RegionalEventStage::Signal,
                6,
            )],
            cursor: 7,
        },
    );

    let current = current.expect("regional events should remain cached");
    assert_eq!(current.cursor, 8);
    assert_eq!(current.events.len(), 1);
    assert_eq!(current.events[0].event_id, "event-1");
    assert_eq!(
        current.events[0].stage,
        tarrowyn_protocol::RegionalEventStage::Escalation
    );
}

#[test]
fn regional_event_cache_stays_bounded_after_incremental_updates() {
    let mut current = Some(RegionalEventsResponse {
        events: Vec::new(),
        cursor: 0,
    });
    merge_regional_events(
        &mut current,
        RegionalEventsResponse {
            events: (0..=MAX_CACHED_REGIONAL_EVENTS)
                .map(|index| {
                    regional_event(
                        &format!("event-{index}"),
                        tarrowyn_protocol::RegionalEventStage::Aftermath,
                        index as u64,
                    )
                })
                .collect(),
            cursor: MAX_CACHED_REGIONAL_EVENTS as u64,
        },
    );

    let current = current.expect("regional events should remain cached");
    assert_eq!(current.events.len(), MAX_CACHED_REGIONAL_EVENTS);
    assert_eq!(current.events.first().unwrap().event_id, "event-1");
    assert_eq!(current.events.last().unwrap().event_id, "event-2048");
}

#[test]
fn regional_event_initial_cache_stays_bounded() {
    let mut current = None;
    merge_regional_events(
        &mut current,
        RegionalEventsResponse {
            events: (0..=MAX_CACHED_REGIONAL_EVENTS)
                .map(|index| {
                    regional_event(
                        &format!("initial-event-{index}"),
                        tarrowyn_protocol::RegionalEventStage::Aftermath,
                        index as u64,
                    )
                })
                .collect(),
            cursor: MAX_CACHED_REGIONAL_EVENTS as u64,
        },
    );

    let current = current.expect("initial regional events should be cached");
    assert_eq!(current.events.len(), MAX_CACHED_REGIONAL_EVENTS);
    assert_eq!(current.events.first().unwrap().event_id, "initial-event-1");
    assert_eq!(
        current.events.last().unwrap().event_id,
        "initial-event-2048"
    );
}

#[test]
fn stale_regional_history_resumes_from_the_authoritative_world_cursor() {
    let mut client = Phase5Client::new();
    client.region = Some(tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: Vec::new(),
        visible_settlements: Vec::new(),
        player_location_id: "hearth".to_owned(),
        travel: None,
        interest_radius: 12,
        cursor: 18,
    });
    client.events = Some(RegionalEventsResponse {
        events: vec![regional_event(
            "stale-event",
            tarrowyn_protocol::RegionalEventStage::Aftermath,
            18,
        )],
        cursor: 18,
    });
    client.projection_cursor = 18;
    client.pending_region = Some(Pending::failed(
        "the current regional map is still in flight",
    ));
    client.pending_events = Some(Pending::failed(
        "HTTP API error in 'GET /v1/events/region?since=18' [cursor_stale]: history changed",
    ));
    let data = crate::data::GameData::load().expect("embedded game data should load");
    let mut projection = WorldProjection::new(&data.config);
    projection.cursor = 42;
    let mut notices = Vec::new();

    client.poll_events(0.0, &mut projection, &mut notices);

    assert_eq!(client.events.as_ref().map(|events| events.cursor), Some(42));
    assert!(client
        .events
        .as_ref()
        .is_some_and(|events| events.events.is_empty()));
    assert!(client.pending_region.is_some());
    assert!(client.region.is_some());
    assert_eq!(client.projection_cursor, 42);
    assert_eq!(client.refresh_timer, 0.0);
    assert_eq!(notices.len(), 1);
}

#[test]
fn regional_cursor_reset_discards_stale_events_and_restarts_refresh() {
    let mut client = Phase5Client::new();
    client.account = Some(account_response(false));
    client.region = Some(tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: Vec::new(),
        visible_settlements: Vec::new(),
        player_location_id: "hearth".to_owned(),
        travel: None,
        interest_radius: 12,
        cursor: 9,
    });
    client.market = Some(MarketSnapshot {
        orders: Vec::new(),
        stock_notes: Vec::new(),
        prices: Vec::new(),
        cursor: 9,
    });
    client.events = Some(RegionalEventsResponse {
        events: vec![regional_event(
            "event-1",
            tarrowyn_protocol::RegionalEventStage::Aftermath,
            9,
        )],
        cursor: 9,
    });
    client
        .commands
        .push_back(Phase5Command::Event(RegionalEventRequest {
            request_id: "stale-event-command".to_owned(),
            action: RegionalEventAction::Intervene,
            event_id: Some("event-1".to_owned()),
            intervention: Some("stale choice".to_owned()),
        }));
    client.refresh_timer = 3.0;

    client.reset_event_cursor();

    assert!(client.region.is_none());
    assert!(client.market.is_none());
    assert!(client.events.is_none());
    assert!(client.account.is_none());
    assert!(client.commands.is_empty());
    assert_eq!(client.refresh_timer, 0.0);
}
