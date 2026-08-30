use super::*;
use tarrowyn_protocol::Position;

#[test]
fn region_travel_recovery_and_market_settle_authoritatively() {
    let repository = WorldRepository::new(ServerConfig {
        tick_interval: Duration::from_millis(1),
        ..ServerConfig::default()
    });
    let traveller = guest(&repository, "phase5-traveller");
    let region = repository.region(&traveller.account_token).unwrap().data;
    assert_eq!(region.locations.len(), 3);
    assert_eq!(region.routes.len(), 3);

    let started = repository
        .travel(
            &traveller.account_token,
            TravelRequest {
                request_id: "start-road".to_owned(),
                action: TravelAction::Start,
                route_id: Some("north-pack-road".to_owned()),
                travel_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(started.accepted);
    let interrupted = repository
        .travel(
            &traveller.account_token,
            TravelRequest {
                request_id: "interrupt-road".to_owned(),
                action: TravelAction::Interrupt,
                route_id: None,
                travel_id: started
                    .travel
                    .as_ref()
                    .map(|travel| travel.travel_id.clone()),
            },
        )
        .unwrap()
        .data;
    assert!(interrupted.accepted);
    let recovered = repository
        .travel(
            &traveller.account_token,
            TravelRequest {
                request_id: "recover-road".to_owned(),
                action: TravelAction::Recover,
                route_id: None,
                travel_id: interrupted
                    .travel
                    .as_ref()
                    .map(|travel| travel.travel_id.clone()),
            },
        )
        .unwrap()
        .data;
    assert!(recovered.accepted);
    for _ in 0..4 {
        repository.tick();
    }
    let arrived = repository.region(&traveller.account_token).unwrap().data;
    assert_eq!(arrived.player_location_id, "whisperwood-outpost");

    let order = repository
        .market_order(
            &traveller.account_token,
            MarketOrderRequest {
                request_id: "order".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("saltmere".to_owned()),
                commodity: Some(tarrowyn_protocol::CommodityKind::Seeds),
                quantity: Some(2),
            },
        )
        .unwrap()
        .data;
    assert!(order.accepted);
    let moved = repository
        .travel(
            &traveller.account_token,
            TravelRequest {
                request_id: "trail".to_owned(),
                action: TravelAction::Start,
                route_id: Some("watch-trail".to_owned()),
                travel_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(moved.accepted);
    for _ in 0..10 {
        repository.tick();
    }
    assert_eq!(
        repository
            .region(&traveller.account_token)
            .unwrap()
            .data
            .player_location_id,
        "saltmere"
    );
    let carrier = guest(&repository, "phase5-market-carrier");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .identities
            .get_mut(&carrier.client_key)
            .expect("carrier identity")
            .position = Position { x: 3, y: 9 };
    }
    let settled = repository
        .market_order(
            &carrier.account_token,
            MarketOrderRequest {
                request_id: "fulfil".to_owned(),
                action: MarketOrderAction::Fulfil,
                order_id: order.order.as_ref().map(|order| order.order_id.clone()),
                destination_location_id: None,
                commodity: None,
                quantity: None,
            },
        )
        .unwrap()
        .data;
    assert!(settled.accepted);
    assert_eq!(
        settled.order.unwrap().status,
        tarrowyn_protocol::MarketOrderStatus::Fulfilled
    );

    let repaired_at_destination = repository
        .route_action(
            &traveller.account_token,
            RouteRequest {
                request_id: "repair-watch-trail".to_owned(),
                route_id: "watch-trail".to_owned(),
                action: RouteAction::Repair,
            },
        )
        .unwrap()
        .data;
    assert!(repaired_at_destination.accepted);

    let returned = repository
        .travel(
            &traveller.account_token,
            TravelRequest {
                request_id: "return-ferry".to_owned(),
                action: TravelAction::Start,
                route_id: Some("saltmere-ferry".to_owned()),
                travel_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(returned.accepted);
    for _ in 0..8 {
        repository.tick();
    }
    assert_eq!(
        repository
            .region(&traveller.account_token)
            .unwrap()
            .data
            .player_location_id,
        "hearth"
    );
}
