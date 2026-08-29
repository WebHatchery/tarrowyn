use super::*;
use crate::config::ServerConfig;
use crate::repository::WorldRepository;
use tarrowyn_protocol::{ExpeditionAction, ExpeditionRequest, GuestSessionRequest};

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
        .expect("expedition announce")
        .data;

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
