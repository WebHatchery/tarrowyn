use super::super::WorldRepository;
use crate::ServerConfig;
use std::time::Duration;
use tarrowyn_protocol::{
    AuthLinkRequest, AuthRefreshRequest, AuthRevokeRequest, ClaimAction, ClaimRequest,
    GuestSessionRequest, MarketOrderAction, MarketOrderRequest, RegionalEventAction,
    RegionalEventRequest, RouteAction, RouteRequest, TravelAction, TravelRequest,
};

mod event_choices;
mod event_retention;
mod fallback;
mod household_history;
mod input_bounds;
mod market_content;
mod market_history;
mod market_retention;
mod moderation;
mod price_boundaries;
mod regional_flow;
mod replay_restart;
mod route_history;
mod session_retention;
mod settlement_chronicle_retention;
mod settlements;
mod travel_boundary;

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
