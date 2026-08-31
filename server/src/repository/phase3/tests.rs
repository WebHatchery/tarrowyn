use super::*;
use crate::config::ServerConfig;
use crate::repository::WorldRepository;
use tarrowyn_protocol::{
    Expedition, ExpeditionAction, ExpeditionMember, ExpeditionRequest, ExpeditionRole,
    ExpeditionStatus, FrontierEvent, GuestSessionRequest, Position, WorldEvent,
};

mod chronicle;
mod expedition_limits;
mod input_bounds;

fn guest(repository: &WorldRepository) -> tarrowyn_protocol::GuestSessionResponse {
    repository
        .guest_session(GuestSessionRequest {
            client_key: Some("chronicle-archive-test".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data
}

#[test]
fn expedition_rejects_unbounded_or_controlled_outpost_names() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository);
    for (request_id, outpost_name) in [
        ("long-outpost-name", "x".repeat(81)),
        ("controlled-outpost-name", "Lantern\nRest".to_owned()),
    ] {
        let error = repository
            .expedition(
                &session.account_token,
                ExpeditionRequest {
                    request_id: request_id.to_owned(),
                    action: ExpeditionAction::Announce,
                    expedition_id: None,
                    role: None,
                    food: 0,
                    tools: 0,
                    materials: 0,
                    safety: 0,
                    outpost_name: Some(outpost_name),
                },
            )
            .expect_err("malformed outpost name should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, "invalid_outpost_name");
    }
}

#[test]
fn expedition_announcement_emits_one_frontier_event() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository);

    repository
        .expedition(
            &session.account_token,
            ExpeditionRequest {
                request_id: "single-announcement-event".to_owned(),
                action: ExpeditionAction::Announce,
                expedition_id: None,
                role: Some(ExpeditionRole::Scout),
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("expedition announce");

    let events = repository
        .events(&session.account_token, 0)
        .expect("world events")
        .data
        .events;
    let expedition_events = events
        .iter()
        .filter(|event| {
            matches!(
                &event.event,
                WorldEvent::Frontier(FrontierEvent::Expedition(expedition))
                    if expedition.expedition_id == "pioneer-1"
            )
        })
        .count();
    assert_eq!(expedition_events, 1);
}

#[test]
fn expedition_actions_reject_stale_selectors_without_mutating_the_registry() {
    let repository = WorldRepository::new(ServerConfig::default());
    let leader = guest(&repository);
    let member = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("stale-expedition-selector-member".to_owned()),
            reset: false,
        })
        .expect("guest member")
        .data;
    repository
        .expedition(
            &leader.account_token,
            ExpeditionRequest {
                request_id: "stale-selector-announce".to_owned(),
                action: ExpeditionAction::Announce,
                expedition_id: None,
                role: Some(ExpeditionRole::Scout),
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("expedition announce");

    for (request_id, action) in [
        ("stale-selector-join", ExpeditionAction::Join),
        ("stale-selector-supply", ExpeditionAction::Supply),
        ("stale-selector-launch", ExpeditionAction::Launch),
        ("stale-selector-resolve", ExpeditionAction::Resolve),
    ] {
        let response = repository
            .expedition(
                &member.account_token,
                ExpeditionRequest {
                    request_id: request_id.to_owned(),
                    action,
                    expedition_id: Some("pioneer-old".to_owned()),
                    role: Some(ExpeditionRole::Builder),
                    food: 6,
                    tools: 3,
                    materials: 8,
                    safety: 3,
                    outpost_name: None,
                },
            )
            .expect("stale expedition selector should return a response")
            .data;
        assert!(!response.accepted);
        assert_eq!(
            response.reason.as_deref(),
            Some("That expedition is no longer current.")
        );
    }

    let expedition = repository
        .world(&leader.account_token)
        .expect("world projection")
        .data
        .expedition
        .expect("current expedition");
    assert_eq!(expedition.expedition_id, "pioneer-1");
    assert_eq!(expedition.members.len(), 1);
    assert_eq!(expedition.food, 0);
    assert_eq!(expedition.status, ExpeditionStatus::Planning);
}

#[test]
fn expedition_launch_and_resolve_require_party_membership() {
    let repository = WorldRepository::new(ServerConfig::default());
    let leader = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("expedition-authority-leader".to_owned()),
            reset: false,
        })
        .expect("leader guest")
        .data;
    let farmer = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("expedition-authority-farmer".to_owned()),
            reset: false,
        })
        .expect("farmer guest")
        .data;
    let builder = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("expedition-authority-builder".to_owned()),
            reset: false,
        })
        .expect("builder guest")
        .data;
    let outsider = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("expedition-authority-outsider".to_owned()),
            reset: false,
        })
        .expect("outsider guest")
        .data;

    repository
        .expedition(
            &leader.account_token,
            ExpeditionRequest {
                request_id: "authority-announce".to_owned(),
                action: ExpeditionAction::Announce,
                expedition_id: None,
                role: Some(ExpeditionRole::Scout),
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("expedition announce");
    for (session, role, request_id) in [
        (&farmer, ExpeditionRole::Farmer, "authority-join-farmer"),
        (&builder, ExpeditionRole::Builder, "authority-join-builder"),
    ] {
        assert!(
            repository
                .expedition(
                    &session.account_token,
                    ExpeditionRequest {
                        request_id: request_id.to_owned(),
                        action: ExpeditionAction::Join,
                        expedition_id: Some("pioneer-1".to_owned()),
                        role: Some(role),
                        food: 0,
                        tools: 0,
                        materials: 0,
                        safety: 0,
                        outpost_name: None,
                    },
                )
                .expect("expedition join")
                .data
                .accepted
        );
    }

    let outsider_launch = repository
        .expedition(
            &outsider.account_token,
            ExpeditionRequest {
                request_id: "authority-outsider-launch".to_owned(),
                action: ExpeditionAction::Launch,
                expedition_id: Some("pioneer-1".to_owned()),
                role: None,
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("outsider launch response")
        .data;
    assert!(!outsider_launch.accepted);
    assert_eq!(
        outsider_launch.reason.as_deref(),
        Some("Join the pioneer party before launching it.")
    );

    let leader_launch = repository
        .expedition(
            &leader.account_token,
            ExpeditionRequest {
                request_id: "authority-leader-launch".to_owned(),
                action: ExpeditionAction::Launch,
                expedition_id: Some("pioneer-1".to_owned()),
                role: None,
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("leader launch response")
        .data;
    assert!(leader_launch.accepted);

    let outsider_resolve = repository
        .expedition(
            &outsider.account_token,
            ExpeditionRequest {
                request_id: "authority-outsider-resolve".to_owned(),
                action: ExpeditionAction::Resolve,
                expedition_id: Some("pioneer-1".to_owned()),
                role: None,
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("outsider resolve response")
        .data;
    assert!(!outsider_resolve.accepted);
    assert_eq!(
        outsider_resolve.reason.as_deref(),
        Some("Join the pioneer party before resolving it.")
    );
    assert_eq!(
        outsider_resolve
            .expedition
            .expect("current expedition")
            .status,
        ExpeditionStatus::Launched
    );
}

#[test]
fn poorly_prepared_expedition_can_retreat_without_founding_an_outpost() {
    let repository = WorldRepository::new(ServerConfig::default());
    let leader = guest(&repository);
    let farmer = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("retreat-expedition-farmer".to_owned()),
            reset: false,
        })
        .expect("farmer guest")
        .data;
    let builder = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("retreat-expedition-builder".to_owned()),
            reset: false,
        })
        .expect("builder guest")
        .data;
    repository
        .expedition(
            &leader.account_token,
            ExpeditionRequest {
                request_id: "retreat-announce".to_owned(),
                action: ExpeditionAction::Announce,
                expedition_id: None,
                role: Some(ExpeditionRole::Scout),
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("expedition announce");
    for (session, role, request_id) in [
        (&farmer, ExpeditionRole::Farmer, "retreat-join-farmer"),
        (&builder, ExpeditionRole::Builder, "retreat-join-builder"),
    ] {
        assert!(
            repository
                .expedition(
                    &session.account_token,
                    ExpeditionRequest {
                        request_id: request_id.to_owned(),
                        action: ExpeditionAction::Join,
                        expedition_id: Some("pioneer-1".to_owned()),
                        role: Some(role),
                        food: 0,
                        tools: 0,
                        materials: 0,
                        safety: 0,
                        outpost_name: None,
                    },
                )
                .expect("expedition join")
                .data
                .accepted
        );
    }

    let launch = repository
        .expedition(
            &leader.account_token,
            ExpeditionRequest {
                request_id: "retreat-launch".to_owned(),
                action: ExpeditionAction::Launch,
                expedition_id: Some("pioneer-1".to_owned()),
                role: None,
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("under-supplied launch should be accepted as an attempt")
        .data;
    assert!(launch.accepted);

    let resolved = repository
        .expedition(
            &leader.account_token,
            ExpeditionRequest {
                request_id: "retreat-resolve".to_owned(),
                action: ExpeditionAction::Resolve,
                expedition_id: Some("pioneer-1".to_owned()),
                role: None,
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("retreated expedition should resolve")
        .data;
    let expedition = resolved.expedition.expect("retreat projection");
    assert!(resolved.accepted);
    assert_eq!(expedition.status, ExpeditionStatus::Retreated);
    assert!(expedition
        .outcome
        .as_deref()
        .is_some_and(|outcome| outcome.contains("retreat")));
    assert!(repository
        .world(&leader.account_token)
        .expect("world projection")
        .data
        .outpost
        .is_none());
    let chronicle = repository
        .chronicle(&leader.account_token, 0)
        .expect("retreat chronicle")
        .data;
    assert_eq!(
        chronicle
            .entries
            .last()
            .expect("retreat chronicle entry")
            .title,
        "The pioneer party returns before the outpost is founded"
    );
}

#[test]
fn state_advertises_configured_pioneer_requirements() {
    let repository = WorldRepository::new(ServerConfig {
        expedition_min_food: 10,
        expedition_min_tools: 5,
        expedition_min_materials: 12,
        expedition_min_safety: 7,
        ..ServerConfig::default()
    });
    let session = guest(&repository);
    let snapshot = repository
        .state(&session.account_token)
        .expect("state")
        .data;

    assert_eq!(snapshot.world.expedition_requirements.food, 10);
    assert_eq!(snapshot.world.expedition_requirements.tools, 5);
    assert_eq!(snapshot.world.expedition_requirements.materials, 12);
    assert_eq!(snapshot.world.expedition_requirements.safety, 7);
}

#[test]
fn threat_pressure_keeps_household_opportunity_score_in_its_valid_range() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase3
            .households
            .first_mut()
            .expect("household")
            .opportunity_score = 0;
    }

    repository.tick();

    let state = repository.state.lock().expect("repository lock");
    assert_eq!(
        state
            .phase3
            .households
            .first()
            .expect("household")
            .opportunity_score,
        0
    );
    drop(state);
    assert!(repository.ops_health().data.ready);
}
