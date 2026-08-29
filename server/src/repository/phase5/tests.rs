use super::super::WorldRepository;
use crate::ServerConfig;
use std::time::Duration;
use tarrowyn_protocol::{
    AuthLinkRequest, AuthRefreshRequest, AuthRevokeRequest, ClaimAction, ClaimRequest,
    GuestSessionRequest, MarketOrderAction, MarketOrderRequest, ModerationReportRequest,
    MovementIntent, RegionalEventAction, RegionalEventRequest, RouteAction, RouteRequest,
    TravelAction, TravelRequest,
};

mod settlements;

fn guest(repository: &WorldRepository, key: &str) -> tarrowyn_protocol::GuestSessionResponse {
    repository
        .guest_session(GuestSessionRequest {
            client_key: Some(key.to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data
}

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
    let settled = repository
        .market_order(
            &traveller.account_token,
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

#[test]
fn travel_locks_movement_until_server_arrival() {
    let repository = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let traveller = guest(&repository, "phase5-travel-lock");
    let started = repository
        .travel(
            &traveller.account_token,
            TravelRequest {
                request_id: "travel-lock-start".to_owned(),
                action: TravelAction::Start,
                route_id: Some("saltmere-ferry".to_owned()),
                travel_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(started.accepted);

    let blocked = repository
        .movement(
            &traveller.account_token,
            MovementIntent {
                request_id: "travel-lock-move".to_owned(),
                dx: 1,
                dy: 0,
            },
        )
        .unwrap()
        .data;
    assert!(!blocked.accepted);
    assert!(blocked
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("regional ledger")));

    for _ in 0..7 {
        repository.tick();
    }
    let arrived = repository
        .movement(
            &traveller.account_token,
            MovementIntent {
                request_id: "travel-lock-arrived-move".to_owned(),
                dx: 1,
                dy: 0,
            },
        )
        .unwrap()
        .data;
    assert!(arrived.accepted);
}

#[test]
fn regional_event_cursor_and_household_history_survive_ticks() {
    let repository = WorldRepository::new(ServerConfig {
        tick_interval: Duration::from_millis(1),
        household_decision_interval_ticks: 1,
        ..ServerConfig::default()
    });
    let session = guest(&repository, "phase5-events");
    let seeded = repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "seed".to_owned(),
                action: RegionalEventAction::Seed,
                event_id: None,
                intervention: None,
            },
        )
        .unwrap()
        .data;
    assert!(seeded.accepted);
    let event_id = seeded.event.as_ref().unwrap().event_id.clone();
    for _ in 0..3 {
        repository.tick();
    }
    let signalled = repository
        .events_region(&session.account_token, 0)
        .unwrap()
        .data;
    assert!(signalled
        .events
        .iter()
        .any(|event| event.event_id == event_id
            && event.stage == tarrowyn_protocol::RegionalEventStage::Escalation));
    let premature_resolution = repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "resolve-too-soon".to_owned(),
                action: RegionalEventAction::Resolve,
                event_id: Some(event_id.clone()),
                intervention: None,
            },
        )
        .unwrap()
        .data;
    assert!(!premature_resolution.accepted);
    assert!(premature_resolution
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("intervention")));
    let arbitrary_intervention = repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "intervene-arbitrary".to_owned(),
                action: RegionalEventAction::Intervene,
                event_id: Some(event_id.clone()),
                intervention: Some("ferry sabotage".to_owned()),
            },
        )
        .unwrap()
        .data;
    assert!(!arbitrary_intervention.accepted);
    assert!(arbitrary_intervention
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("visible intervention options")));
    let intervention = repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "intervene".to_owned(),
                action: RegionalEventAction::Intervene,
                event_id: Some(event_id.clone()),
                intervention: Some("repair ferry markers".to_owned()),
            },
        )
        .unwrap()
        .data;
    assert!(intervention.accepted);
    let resolved = repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "resolve".to_owned(),
                action: RegionalEventAction::Resolve,
                event_id: Some(event_id.clone()),
                intervention: None,
            },
        )
        .unwrap()
        .data;
    assert!(resolved.accepted);
    let resolution_cursor = resolved.event.as_ref().unwrap().cursor;
    for _ in 0..4 {
        repository.tick();
    }
    let aftermath = repository
        .events_region(&session.account_token, resolution_cursor)
        .unwrap()
        .data;
    assert!(aftermath.events.iter().any(|event| {
        event.event_id == event_id
            && event.stage == tarrowyn_protocol::RegionalEventStage::Aftermath
            && event.cursor > resolution_cursor
    }));
    let households = repository
        .households_region(&session.account_token)
        .unwrap()
        .data;
    assert!(!households.households[0].history.is_empty());
    assert!(
        !repository
            .law_boundary(&session.account_token)
            .unwrap()
            .data
            .pvp_enabled
    );
}

#[test]
fn regional_event_timeout_keeps_resolution_authoritative() {
    let repository = WorldRepository::new(ServerConfig {
        tick_interval: Duration::from_millis(1),
        household_decision_interval_ticks: 1,
        ..ServerConfig::default()
    });
    let session = guest(&repository, "phase5-event-timeout");
    let seeded = repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "timeout-seed".to_owned(),
                action: RegionalEventAction::Seed,
                event_id: None,
                intervention: None,
            },
        )
        .unwrap()
        .data;
    let event_id = seeded.event.as_ref().unwrap().event_id.clone();
    let intervention = repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "timeout-intervene".to_owned(),
                action: RegionalEventAction::Intervene,
                event_id: Some(event_id.clone()),
                intervention: Some("repair ferry markers".to_owned()),
            },
        )
        .unwrap()
        .data;
    assert!(intervention.accepted);
    for _ in 0..5 {
        repository.tick();
    }
    let events = repository
        .events_region(&session.account_token, 0)
        .unwrap()
        .data;
    let resolution = events
        .events
        .iter()
        .find(|event| event.event_id == event_id)
        .expect("the timed event should remain in the regional stream");
    assert_eq!(
        resolution.stage,
        tarrowyn_protocol::RegionalEventStage::Resolution
    );
    assert!(resolution.outcome.is_some());
}

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
fn oidc_link_refresh_and_revoke_keep_character_boundary() {
    let repository = WorldRepository::new(ServerConfig {
        tick_interval: Duration::from_millis(1),
        ..ServerConfig::default()
    });
    let linked_guest = guest(&repository, "phase5-link");
    let link_request = AuthLinkRequest {
        request_id: "link".to_owned(),
        provider: "webhatchery-identity-oidc".to_owned(),
        subject: "subject-42".to_owned(),
        display_name: Some("Linked traveller".to_owned()),
    };
    let linked = repository
        .auth_link(&linked_guest.account_token, link_request.clone())
        .unwrap()
        .data;
    let linked_retry = repository
        .auth_link(&linked.session.account_token, link_request)
        .unwrap()
        .data;
    assert_eq!(linked_retry, linked);
    let account = repository
        .account(&linked.session.account_token)
        .unwrap()
        .data;
    assert!(!account.guest_fixture);
    assert_eq!(account.character_id, linked.character_id);
    assert!(repository.account(&linked_guest.account_token).is_err());
    assert!(repository.account(&linked.session.account_token).is_ok());
    let refresh_request = AuthRefreshRequest {
        request_id: "refresh".to_owned(),
        refresh_token: linked.session.refresh_token.clone(),
    };
    let refreshed = repository
        .auth_refresh(refresh_request.clone())
        .unwrap()
        .data;
    let refreshed_retry = repository.auth_refresh(refresh_request).unwrap().data;
    assert_eq!(refreshed_retry, refreshed);
    assert!(repository.account(&refreshed.session.account_token).is_ok());
    let revoke_request = AuthRevokeRequest {
        request_id: "revoke".to_owned(),
        revoke_all: true,
    };
    let revoked = repository
        .auth_revoke(&refreshed.session.account_token, revoke_request.clone())
        .unwrap()
        .data;
    let revoked_retry = repository
        .auth_revoke(&refreshed.session.account_token, revoke_request)
        .unwrap()
        .data;
    assert_eq!(revoked_retry, revoked);
    assert!(revoked.revoked_sessions >= 1);
    assert!(repository
        .account(&refreshed.session.account_token)
        .is_err());
}

#[test]
fn identity_linking_rejects_guest_and_subject_collisions() {
    let repository = WorldRepository::new(ServerConfig::default());
    let first_guest = guest(&repository, "phase6-link-owner");
    let request = AuthLinkRequest {
        request_id: "owner-link".to_owned(),
        provider: "webhatchery-identity-oidc".to_owned(),
        subject: "shared-subject".to_owned(),
        display_name: None,
    };
    let linked = repository
        .auth_link(&first_guest.account_token, request.clone())
        .unwrap()
        .data;

    let second_guest = guest(&repository, "phase6-link-collision");
    let subject_conflict = repository
        .auth_link(
            &second_guest.account_token,
            AuthLinkRequest {
                request_id: "subject-collision".to_owned(),
                ..request.clone()
            },
        )
        .unwrap_err();
    assert_eq!(subject_conflict.status, 409);
    assert_eq!(subject_conflict.error.code, "identity_already_linked");

    let guest_conflict = repository
        .auth_link(
            &linked.session.account_token,
            AuthLinkRequest {
                request_id: "guest-collision".to_owned(),
                subject: "different-subject".to_owned(),
                ..request
            },
        )
        .unwrap_err();
    assert_eq!(guest_conflict.status, 409);
    assert_eq!(guest_conflict.error.code, "guest_already_linked");
    assert_eq!(
        repository
            .account(&linked.session.account_token)
            .unwrap()
            .data
            .account_id,
        linked.account_id
    );
}

#[test]
fn auth_replay_results_survive_repository_restart() {
    let path =
        std::env::temp_dir().join(format!("tarrowyn-auth-replay-{}.json", std::process::id()));
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
    let refreshed = first.auth_refresh(refresh_request.clone()).unwrap().data;
    drop(first);

    let second = WorldRepository::new(config.clone());
    let linked_after_restart = second
        .auth_link(&refreshed.session.account_token, link_request)
        .unwrap()
        .data;
    let refreshed_after_restart = second.auth_refresh(refresh_request).unwrap().data;
    assert_eq!(linked_after_restart, linked);
    assert_eq!(refreshed_after_restart, refreshed);
    assert!(second
        .account(&refreshed_after_restart.session.account_token)
        .is_ok());
    let revoke_request = AuthRevokeRequest {
        request_id: "restart-revoke".to_owned(),
        revoke_all: true,
    };
    let revoked = second
        .auth_revoke(&refreshed.session.account_token, revoke_request.clone())
        .unwrap()
        .data;
    drop(second);

    let third = WorldRepository::new(config);
    let revoked_after_restart = third
        .auth_revoke(&refreshed.session.account_token, revoke_request)
        .unwrap()
        .data;
    assert_eq!(revoked_after_restart, revoked);
    let _ = std::fs::remove_file(path);
}

#[test]
fn production_characters_cannot_reenter_through_guest_login() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest_session = guest(&repository, "phase6-guest-boundary");
    let linked = repository
        .auth_link(
            &guest_session.account_token,
            AuthLinkRequest {
                request_id: "boundary-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "boundary-subject".to_owned(),
                display_name: None,
            },
        )
        .unwrap()
        .data;
    repository
        .auth_revoke(
            &linked.session.account_token,
            AuthRevokeRequest {
                request_id: "boundary-revoke".to_owned(),
                revoke_all: true,
            },
        )
        .unwrap();

    let rejected = repository
        .guest_session(GuestSessionRequest {
            client_key: Some(guest_session.client_key),
            reset: false,
        })
        .unwrap_err();
    assert_eq!(rejected.status, 409);
    assert_eq!(rejected.error.code, "production_identity_required");
}

#[test]
fn moderation_report_retries_return_the_original_queued_report() {
    let repository = WorldRepository::new(ServerConfig {
        moderation_cooldown_ticks: 2,
        ..ServerConfig::default()
    });
    let session = guest(&repository, "phase6-moderation-replay");
    let request = ModerationReportRequest {
        request_id: "moderation-replay".to_owned(),
        target_account_id: None,
        message_id: None,
        category: "player_report".to_owned(),
        note: "The same report should not be queued twice.".to_owned(),
    };
    let first = repository
        .moderation_report(&session.account_token, request.clone())
        .unwrap()
        .data;
    let retry = repository
        .moderation_report(&session.account_token, request.clone())
        .unwrap()
        .data;
    assert_eq!(retry, first);
    assert_eq!(first.status, "queued");

    let limited = repository
        .moderation_report(
            &session.account_token,
            ModerationReportRequest {
                request_id: "moderation-too-soon".to_owned(),
                ..request
            },
        )
        .unwrap_err();
    assert_eq!(limited.status, 429);
    assert_eq!(limited.error.code, "moderation_rate_limited");
}
