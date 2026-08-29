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
