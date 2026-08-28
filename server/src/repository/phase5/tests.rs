use super::super::WorldRepository;
use crate::ServerConfig;
use std::time::Duration;
use tarrowyn_protocol::{
    AuthLinkRequest, AuthRefreshRequest, AuthRevokeRequest, GuestSessionRequest, MarketOrderAction,
    MarketOrderRequest, RegionalEventAction, RegionalEventRequest, TravelAction, TravelRequest,
};

fn guest(repository: &WorldRepository, key: &str) -> tarrowyn_protocol::GuestSessionResponse {
    repository
        .guest_session(GuestSessionRequest {
            client_key: Some(key.to_owned()),
            reset: false,
        })
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
    let event_id = seeded.event.unwrap().event_id;
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
                event_id: Some(event_id),
                intervention: None,
            },
        )
        .unwrap()
        .data;
    assert!(resolved.accepted);
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
        .auth_link(&linked_guest.account_token, link_request)
        .unwrap()
        .data;
    assert_eq!(linked_retry, linked);
    let account = repository
        .account(&linked.session.account_token)
        .unwrap()
        .data;
    assert!(!account.guest_fixture);
    assert_eq!(account.character_id, linked.character_id);
    let _resumed_guest = guest(&repository, "phase5-link");
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
    let revoked = repository
        .auth_revoke(
            &refreshed.session.account_token,
            AuthRevokeRequest {
                request_id: "revoke".to_owned(),
                revoke_all: true,
            },
        )
        .unwrap()
        .data;
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
            &first_guest.account_token,
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

    let second = WorldRepository::new(config);
    let resumed_guest = guest(&second, "phase6-auth-replay");
    let linked_after_restart = second
        .auth_link(&resumed_guest.account_token, link_request)
        .unwrap()
        .data;
    let refreshed_after_restart = second.auth_refresh(refresh_request).unwrap().data;
    assert_eq!(linked_after_restart, linked);
    assert_eq!(refreshed_after_restart, refreshed);
    assert!(second
        .account(&refreshed_after_restart.session.account_token)
        .is_ok());
    let _ = std::fs::remove_file(path);
}
