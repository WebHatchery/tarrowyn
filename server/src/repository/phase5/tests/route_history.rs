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
