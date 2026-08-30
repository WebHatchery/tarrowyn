use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{GuestSessionRequest, RouteAction, RouteRequest};

#[test]
fn accepted_route_logistics_history_reaches_both_endpoints() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase5-route-history".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    let route = repository
        .route_action(
            &session.account_token,
            RouteRequest {
                request_id: "route-history-repair".to_owned(),
                route_id: "north-pack-road".to_owned(),
                action: RouteAction::Repair,
            },
        )
        .expect("route action")
        .data;
    assert!(route.accepted);

    let state = repository.state.lock().unwrap();
    for location_id in ["hearth", "whisperwood-outpost"] {
        let settlement = state
            .phase5
            .settlements
            .iter()
            .find(|settlement| settlement.location_id == location_id)
            .expect("route endpoint settlement");
        assert!(settlement
            .chronicle
            .iter()
            .any(|entry| entry.kind == "route logistics"));
    }
}

#[test]
fn rejected_route_repair_does_not_mark_route_as_active() {
    let repository = WorldRepository::new(ServerConfig {
        starting_gold: 0,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase5-route-rejection".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    repository.tick();
    let response = repository
        .route_action(
            &session.account_token,
            RouteRequest {
                request_id: "route-rejection-repair".to_owned(),
                route_id: "north-pack-road".to_owned(),
                action: RouteAction::Repair,
            },
        )
        .expect("route action")
        .data;

    assert!(!response.accepted);
    assert_eq!(response.route.last_action_tick, 0);
    assert_eq!(
        response.route.status,
        tarrowyn_protocol::RouteStatus::Threatened
    );
    let state = repository.state.lock().unwrap();
    let route = state
        .phase5
        .routes
        .iter()
        .find(|route| route.route_id == "north-pack-road")
        .expect("route remains recorded");
    assert_eq!(route.last_action_tick, 0);
}

#[test]
fn route_logistics_accept_only_one_step_per_decision_interval() {
    let repository = WorldRepository::new(ServerConfig {
        household_decision_interval_ticks: 4,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase5-route-cooldown".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    let first = repository
        .route_action(
            &session.account_token,
            RouteRequest {
                request_id: "route-cooldown-first".to_owned(),
                route_id: "north-pack-road".to_owned(),
                action: RouteAction::Improve,
            },
        )
        .expect("first route improvement")
        .data;
    assert!(first.accepted);

    let blocked = repository
        .route_action(
            &session.account_token,
            RouteRequest {
                request_id: "route-cooldown-blocked".to_owned(),
                route_id: "north-pack-road".to_owned(),
                action: RouteAction::Improve,
            },
        )
        .expect("cooldown response")
        .data;
    assert!(!blocked.accepted);
    assert!(blocked
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("next logistics step")));

    for _ in 0..4 {
        repository.tick();
    }
    let state = repository.state.lock().unwrap();
    assert!(!state
        .phase5
        .route_action_available_at_tick
        .contains_key("north-pack-road"));
    drop(state);
    let after_interval = repository
        .route_action(
            &session.account_token,
            RouteRequest {
                request_id: "route-cooldown-after".to_owned(),
                route_id: "north-pack-road".to_owned(),
                action: RouteAction::Improve,
            },
        )
        .expect("route improvement after cooldown")
        .data;
    assert!(after_interval.accepted);
}
