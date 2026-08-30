use super::*;
use crate::config::ServerConfig;
use crate::repository::WorldRepository;
use tarrowyn_protocol::{
    Expedition, ExpeditionAction, ExpeditionMember, ExpeditionRequest, ExpeditionRole,
    ExpeditionStatus, FrontierEvent, GuestSessionRequest, Position, WorldEvent,
};

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
fn chronicle_keeps_archived_entries_searchable_and_summarised() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        for index in 0..(MAX_CHRONICLE + 3) {
            record(
                &mut state,
                "archived achievement",
                &format!("Achievement {index:02}"),
                "An achievement remains in the regional record.",
            );
        }
    }

    let recent = repository
        .chronicle(&session.account_token, 0)
        .expect("chronicle")
        .data;
    assert_eq!(recent.entries.len(), MAX_CHRONICLE);
    let summary = recent.summary.expect("archived summary");
    assert_eq!(summary.entry_count, 3);
    assert!(summary.from_cursor < recent.entries[0].cursor);
    assert!(summary.to_cursor > summary.from_cursor);

    let search = repository
        .chronicle_search(&session.account_token, "Achievement 00", 0)
        .expect("chronicle search")
        .data;
    assert!(search
        .entries
        .iter()
        .any(|entry| entry.title == "Achievement 00"));
    assert_eq!(search.summary.expect("search summary").entry_count, 1);
}

#[test]
fn chronicle_rejects_cursors_outside_the_retained_event_window() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository);
    let (oldest, current) = {
        let mut state = repository.state.lock().expect("repository lock");
        for index in 0..=super::super::MAX_EVENTS {
            record(
                &mut state,
                "cursor boundary",
                &format!("Boundary entry {index}"),
                "The chronicle boundary remains explicit.",
            );
        }
        (
            state.events.front().expect("retained event window").cursor,
            state.cursor,
        )
    };

    let stale = repository
        .chronicle(&session.account_token, oldest - 2)
        .expect_err("a stale chronicle cursor must fail closed");
    assert_eq!(stale.status, 409);
    assert_eq!(stale.error.code, "cursor_stale");

    let ahead = repository
        .chronicle(&session.account_token, current + 1)
        .expect_err("a chronicle cursor ahead of the world must fail closed");
    assert_eq!(ahead.status, 409);
    assert_eq!(ahead.error.code, "cursor_ahead");
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
fn pioneer_expedition_keeps_its_durable_member_list_bounded() {
    let repository = WorldRepository::new(ServerConfig::default());
    let leader = guest(&repository);
    repository
        .expedition(
            &leader.account_token,
            ExpeditionRequest {
                request_id: "member-cap-announce".to_owned(),
                action: ExpeditionAction::Announce,
                expedition_id: None,
                role: Some(tarrowyn_protocol::ExpeditionRole::Scout),
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("expedition announce");

    for index in 1..super::MAX_EXPEDITION_MEMBERS {
        let member = repository
            .guest_session(GuestSessionRequest {
                client_key: Some(format!("expedition-member-{index}")),
                reset: false,
            })
            .expect("guest member")
            .data;
        let joined = repository
            .expedition(
                &member.account_token,
                ExpeditionRequest {
                    request_id: format!("member-cap-join-{index}"),
                    action: ExpeditionAction::Join,
                    expedition_id: Some("pioneer-1".to_owned()),
                    role: Some(tarrowyn_protocol::ExpeditionRole::Builder),
                    food: 0,
                    tools: 0,
                    materials: 0,
                    safety: 0,
                    outpost_name: None,
                },
            )
            .expect("expedition join")
            .data;
        assert!(joined.accepted);
    }

    let extra = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("expedition-member-overflow".to_owned()),
            reset: false,
        })
        .expect("overflow guest")
        .data;
    let rejected = repository
        .expedition(
            &extra.account_token,
            ExpeditionRequest {
                request_id: "member-cap-overflow".to_owned(),
                action: ExpeditionAction::Join,
                expedition_id: Some("pioneer-1".to_owned()),
                role: Some(tarrowyn_protocol::ExpeditionRole::Farmer),
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("overflow join should return a readable rejection")
        .data;

    assert!(!rejected.accepted);
    assert!(rejected
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("20-member")));
    assert_eq!(
        rejected
            .expedition
            .expect("expedition projection")
            .members
            .len(),
        super::MAX_EXPEDITION_MEMBERS
    );
}

#[test]
fn loading_an_oversized_pioneer_record_keeps_a_valid_leader_window() {
    let mut phase = super::Phase3State {
        expedition: Some(Expedition {
            expedition_id: "legacy-pioneer".to_owned(),
            outpost_name: "Legacy Rest".to_owned(),
            leader_account_id: "account-outside-window".to_owned(),
            members: (0..=super::MAX_EXPEDITION_MEMBERS)
                .map(|index| ExpeditionMember {
                    account_id: format!("account-{index}"),
                    display_name: format!("Member {index}"),
                    role: ExpeditionRole::Builder,
                })
                .collect(),
            food: 6,
            tools: 3,
            materials: 8,
            safety: 3,
            status: ExpeditionStatus::Planning,
            outcome: None,
            outpost_position: Position { x: 14, y: 8 },
        }),
        ..Default::default()
    };

    super::trim_expedition_members(&mut phase);

    let expedition = phase.expedition.expect("legacy expedition");
    assert_eq!(expedition.members.len(), super::MAX_EXPEDITION_MEMBERS);
    assert_eq!(expedition.leader_account_id, "account-0");
    assert!(expedition
        .members
        .iter()
        .any(|member| member.account_id == expedition.leader_account_id));
}
