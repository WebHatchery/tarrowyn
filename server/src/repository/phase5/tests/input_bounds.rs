use super::*;
use tarrowyn_protocol::{
    MarketOrderAction, MarketOrderRequest, RegionalEventAction, RegionalEventRequest, RouteAction,
    RouteRequest, TravelAction, TravelRequest,
};

#[test]
fn route_and_travel_selectors_reject_unbounded_or_controlled_ids() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase5-route-input");

    for route_id in ["x".repeat(161), "route\nwith-control".to_owned()] {
        let error = repository
            .route_action(
                &session.account_token,
                RouteRequest {
                    request_id: format!("route-input-{}", route_id.len()),
                    route_id,
                    action: RouteAction::Repair,
                },
            )
            .expect_err("invalid route selector should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, "invalid_route_id");
    }

    let error = repository
        .travel(
            &session.account_token,
            TravelRequest {
                request_id: "travel-input".to_owned(),
                action: TravelAction::Interrupt,
                route_id: None,
                travel_id: Some("travel\nwith-control".to_owned()),
            },
        )
        .expect_err("invalid travel selector should be rejected");
    assert_eq!(error.status, 400);
    assert_eq!(error.error.code, "invalid_travel_id");
}

#[test]
fn market_selectors_reject_unbounded_or_controlled_ids() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase5-market-input");

    let cases = [
        (Some("x".repeat(161)), None, "invalid_order_id"),
        (
            None,
            Some("location\nwith-control".to_owned()),
            "invalid_location_id",
        ),
    ];
    for (order_id, destination_location_id, expected_code) in cases {
        let error = repository
            .market_order(
                &session.account_token,
                MarketOrderRequest {
                    request_id: format!("market-input-{expected_code}"),
                    action: MarketOrderAction::Create,
                    order_id,
                    destination_location_id,
                    commodity: None,
                    quantity: None,
                },
            )
            .expect_err("invalid market selector should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, expected_code);
    }
}

#[test]
fn regional_event_selectors_reject_unbounded_or_controlled_values() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase5-event-input");

    let cases = [
        (Some("x".repeat(161)), None, "invalid_event_id"),
        (
            None,
            Some("intervention\nwith-control".to_owned()),
            "invalid_intervention",
        ),
    ];
    for (event_id, intervention, expected_code) in cases {
        let error = repository
            .event_action(
                &session.account_token,
                RegionalEventRequest {
                    request_id: format!("event-input-{expected_code}"),
                    action: RegionalEventAction::Intervene,
                    event_id,
                    intervention,
                },
            )
            .expect_err("invalid event input should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, expected_code);
    }
}
