use super::*;

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
