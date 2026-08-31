use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    GuestSessionRequest, RegionalEventAction, RegionalEventRequest, TravelAction, TravelRequest,
};

#[test]
fn malformed_regional_location_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.locations[0].access_note.clear();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_regional_route_action_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.routes[0].last_action_tick = state.tick.saturating_add(1);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn retained_regional_event_cannot_precede_the_history_floor() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("regional-history-floor-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let event = repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "regional-history-floor-seed".to_owned(),
                action: RegionalEventAction::Seed,
                event_id: None,
                intervention: None,
            },
        )
        .expect("regional event")
        .data
        .event
        .expect("seeded event");

    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.event_history_floor = event.cursor;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn retained_regional_events_cannot_share_a_cursor() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("regional-duplicate-cursor-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "regional-duplicate-cursor-seed".to_owned(),
                action: RegionalEventAction::Seed,
                event_id: None,
                intervention: None,
            },
        )
        .expect("regional event");

    {
        let mut state = repository.state.lock().expect("repository lock");
        let mut duplicate = state.phase5.events[0].clone();
        duplicate.event_id = "regional-duplicate-cursor-copy".to_owned();
        state.phase5.events.push(duplicate);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn travelling_record_cannot_be_past_its_eta() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("regional-travel-timeline-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let started = repository
        .travel(
            &session.account_token,
            TravelRequest {
                request_id: "regional-travel-timeline-start".to_owned(),
                action: TravelAction::Start,
                route_id: Some("north-pack-road".to_owned()),
                travel_id: None,
            },
        )
        .expect("travel should start")
        .data;
    assert!(started.accepted);

    {
        let mut state = repository.state.lock().expect("repository lock");
        let tick = state.tick;
        let travel = state
            .phase5
            .travel
            .get_mut(&session.client_key)
            .expect("travel record");
        travel.eta_tick = tick;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
