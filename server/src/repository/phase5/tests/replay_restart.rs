use super::*;

#[test]
fn regional_mutation_replays_survive_repository_restart() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-regional-replay-{}.json",
        std::process::id()
    ));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        tick_interval: Duration::from_millis(1),
        movement_cooldown_ticks: 0,
        ..ServerConfig::default()
    };
    let first = WorldRepository::new(config.clone());
    let first_session = guest(&first, "phase5-replay");
    let route_request = RouteRequest {
        request_id: "restart-route-repair".to_owned(),
        route_id: "north-pack-road".to_owned(),
        action: RouteAction::Repair,
    };
    let repaired = first
        .route_action(&first_session.account_token, route_request.clone())
        .unwrap()
        .data;
    assert!(repaired.accepted);
    let market_request = MarketOrderRequest {
        request_id: "restart-market-create".to_owned(),
        action: MarketOrderAction::Create,
        order_id: None,
        destination_location_id: Some("saltmere".to_owned()),
        commodity: Some(tarrowyn_protocol::CommodityKind::Seeds),
        quantity: Some(2),
    };
    let order = first
        .market_order(&first_session.account_token, market_request.clone())
        .unwrap()
        .data;
    assert!(order.accepted);
    let event_request = RegionalEventRequest {
        request_id: "restart-event-seed".to_owned(),
        action: RegionalEventAction::Seed,
        event_id: None,
        intervention: None,
    };
    let seeded = first
        .event_action(&first_session.account_token, event_request.clone())
        .unwrap()
        .data;
    assert!(seeded.accepted);
    drop(first);

    let second = WorldRepository::new(config);
    let second_session = guest(&second, "phase5-replay");
    assert_eq!(
        second
            .route_action(&second_session.account_token, route_request)
            .unwrap()
            .data,
        repaired
    );
    assert_eq!(
        second
            .market_order(&second_session.account_token, market_request)
            .unwrap()
            .data,
        order
    );
    assert_eq!(
        second
            .event_action(&second_session.account_token, event_request)
            .unwrap()
            .data,
        seeded
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejected_regional_mutations_replay_after_repository_restart() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-rejected-replay-{}.json",
        std::process::id()
    ));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        tick_interval: Duration::from_millis(1),
        ..ServerConfig::default()
    };
    let first = WorldRepository::new(config.clone());
    let first_session = guest(&first, "phase5-rejected-replay");
    let claim_request = ClaimRequest {
        request_id: "restart-rejected-claim".to_owned(),
        action: ClaimAction::Renew,
    };
    let rejected_claim = first
        .claim(&first_session.account_token, claim_request.clone())
        .unwrap()
        .data;
    assert!(!rejected_claim.accepted);
    let travel_request = TravelRequest {
        request_id: "restart-rejected-travel".to_owned(),
        action: TravelAction::Interrupt,
        route_id: None,
        travel_id: None,
    };
    let rejected_travel = first
        .travel(&first_session.account_token, travel_request.clone())
        .unwrap()
        .data;
    assert!(!rejected_travel.accepted);
    drop(first);

    let second = WorldRepository::new(config);
    let second_session = guest(&second, "phase5-rejected-replay");
    assert_eq!(
        second
            .claim(&second_session.account_token, claim_request)
            .unwrap()
            .data,
        rejected_claim
    );
    assert_eq!(
        second
            .travel(&second_session.account_token, travel_request)
            .unwrap()
            .data,
        rejected_travel
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn auth_replay_results_survive_repository_restart() {
    let path =
        std::env::temp_dir().join(format!("tarrowyn-auth-replay-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let path_string = path.to_string_lossy().into_owned();
    let config = ServerConfig {
        persistence_path: Some(path_string),
        backup_path: None,
        ..ServerConfig::default()
    };
    let first = WorldRepository::new(config.clone());
    let first_guest = guest(&first, "phase6-auth-replay");
    let link_request = AuthLinkRequest {
        request_id: "restart-link".to_owned(),
        provider: "webhatchery-identity-oidc".to_owned(),
        subject: "restart-subject".to_owned(),
        display_name: Some("Restart traveller".to_owned()),
    };
    let linked = first
        .auth_link(&first_guest.account_token, link_request.clone())
        .unwrap()
        .data;
    let refresh_request = AuthRefreshRequest {
        request_id: "restart-refresh".to_owned(),
        refresh_token: linked.session.refresh_token.clone(),
    };
    drop(first);

    let second = WorldRepository::new(config.clone());
    let linked_after_restart = second
        .auth_link(&first_guest.account_token, link_request)
        .unwrap()
        .data;
    assert_eq!(linked_after_restart, linked);
    let refreshed = second.auth_refresh(refresh_request.clone()).unwrap().data;
    drop(second);

    let third = WorldRepository::new(config.clone());
    let refreshed_after_restart = third.auth_refresh(refresh_request).unwrap().data;
    assert_eq!(refreshed_after_restart, refreshed);
    assert!(third
        .account(&refreshed_after_restart.session.account_token)
        .is_ok());
    let revoke_request = AuthRevokeRequest {
        request_id: "restart-revoke".to_owned(),
        revoke_all: true,
    };
    let revoked = third
        .auth_revoke(&refreshed.session.account_token, revoke_request.clone())
        .unwrap()
        .data;
    drop(third);

    let fourth = WorldRepository::new(config);
    let revoked_after_restart = fourth
        .auth_revoke(&refreshed.session.account_token, revoke_request)
        .unwrap()
        .data;
    assert_eq!(revoked_after_restart, revoked);
    let _ = std::fs::remove_file(path);
}
