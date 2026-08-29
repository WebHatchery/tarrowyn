use super::super::super::{ServerConfig, WorldRepository};
use super::super::logic::advance_travel;
use super::guest;
use tarrowyn_protocol::{RouteStatus, TravelAction, TravelRequest};

#[test]
fn travel_progress_preserves_large_route_percentages_without_overflow() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "travel-progress-boundary");
    {
        let mut state = repository.state.lock().unwrap();
        let route = state
            .phase5
            .routes
            .iter_mut()
            .find(|route| route.route_id == "north-pack-road")
            .unwrap();
        route.status = RouteStatus::Operational;
        route.travel_ticks = u64::MAX;
    }

    let started = repository
        .travel(
            &session.account_token,
            TravelRequest {
                request_id: "travel-progress-start".to_owned(),
                action: TravelAction::Start,
                route_id: Some("north-pack-road".to_owned()),
                travel_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(started.accepted);

    let mut state = repository.state.lock().unwrap();
    state.tick = u64::MAX / 2;
    advance_travel(&mut state);
    assert_eq!(
        state
            .phase5
            .travel
            .get(&session.client_key)
            .unwrap()
            .progress,
        49
    );
}
