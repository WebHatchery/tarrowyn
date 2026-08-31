use super::*;

#[test]
fn finished_expedition_cycle_announces_a_new_party() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;
    client.projection.expedition = Some(tarrowyn_protocol::Expedition {
        expedition_id: "pioneer-1".to_owned(),
        outpost_name: "Lantern Rest".to_owned(),
        leader_account_id: "account-1".to_owned(),
        members: Vec::new(),
        food: 12,
        tools: 6,
        materials: 16,
        safety: 6,
        status: tarrowyn_protocol::ExpeditionStatus::Succeeded,
        outcome: Some("The outpost stands.".to_owned()),
        outpost_position: tarrowyn_protocol::Position { x: 14, y: 8 },
    });

    client.queue_expedition_cycle();
    let Some(FrontierCommand::Expedition(request)) = client.frontier.commands.pop_front() else {
        panic!("a finished expedition should queue a new announcement");
    };
    assert_eq!(request.action, ExpeditionAction::Announce);
    assert_eq!(request.role, Some(ExpeditionRole::Scout));
}

#[test]
fn expedition_cycle_chooses_the_missing_scout_role() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;
    client.account = Some(tarrowyn_protocol::GuestSessionResponse {
        client_key: "expedition-role-client".to_owned(),
        account_id: "account-4".to_owned(),
        character_id: "character-4".to_owned(),
        display_name: "The traveller".to_owned(),
        account_token: "token".to_owned(),
        expires_in_seconds: 900,
    });
    client.projection.expedition = Some(tarrowyn_protocol::Expedition {
        expedition_id: "pioneer-1".to_owned(),
        outpost_name: "Lantern Rest".to_owned(),
        leader_account_id: "account-1".to_owned(),
        members: vec![
            tarrowyn_protocol::ExpeditionMember {
                account_id: "account-1".to_owned(),
                display_name: "A farmer".to_owned(),
                role: ExpeditionRole::Farmer,
            },
            tarrowyn_protocol::ExpeditionMember {
                account_id: "account-2".to_owned(),
                display_name: "A builder".to_owned(),
                role: ExpeditionRole::Builder,
            },
        ],
        food: 0,
        tools: 0,
        materials: 0,
        safety: 0,
        status: tarrowyn_protocol::ExpeditionStatus::Planning,
        outcome: None,
        outpost_position: tarrowyn_protocol::Position { x: 14, y: 8 },
    });

    client.queue_expedition_cycle();
    let Some(FrontierCommand::Expedition(request)) = client.frontier.commands.pop_front() else {
        panic!("a missing expedition role should queue a join request");
    };
    assert_eq!(request.action, ExpeditionAction::Join);
    assert_eq!(request.role, Some(ExpeditionRole::Scout));

    client
        .projection
        .expedition
        .as_mut()
        .expect("expedition projection")
        .members
        .push(tarrowyn_protocol::ExpeditionMember {
            account_id: "account-4".to_owned(),
            display_name: "The traveller".to_owned(),
            role: ExpeditionRole::Scout,
        });
    client
        .projection
        .expedition
        .as_mut()
        .expect("expedition projection")
        .food = 6;
    client
        .projection
        .expedition
        .as_mut()
        .expect("expedition projection")
        .tools = 3;
    client
        .projection
        .expedition
        .as_mut()
        .expect("expedition projection")
        .materials = 8;
    client
        .projection
        .expedition
        .as_mut()
        .expect("expedition projection")
        .safety = 3;
    client.projection.expedition_requirements = tarrowyn_protocol::ExpeditionRequirements {
        food: 10,
        tools: 5,
        materials: 12,
        safety: 7,
    };

    client.frontier.commands.clear();
    client.queue_expedition_cycle();
    let Some(FrontierCommand::Expedition(request)) = client.frontier.commands.pop_front() else {
        panic!("custom expedition requirements should queue another supply request");
    };
    assert_eq!(request.action, ExpeditionAction::Supply);
}

#[test]
fn expedition_resolution_notice_explains_a_retreat() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    let expedition = tarrowyn_protocol::Expedition {
        expedition_id: "pioneer-1".to_owned(),
        outpost_name: "Lantern Rest".to_owned(),
        leader_account_id: "account-1".to_owned(),
        members: Vec::new(),
        food: 0,
        tools: 0,
        materials: 0,
        safety: 0,
        status: tarrowyn_protocol::ExpeditionStatus::Retreated,
        outcome: Some("Supplies failed the road.".to_owned()),
        outpost_position: tarrowyn_protocol::Position { x: 14, y: 8 },
    };
    let mut notices = Vec::new();

    client.frontier.apply_command(
        FrontierCommandResponse::Expedition(ExpeditionResponse {
            request_id: "retreat-notice".to_owned(),
            accepted: true,
            expedition: Some(expedition),
            reason: None,
        }),
        &mut client.projection,
        &mut notices,
        true,
    );

    assert!(matches!(
        notices.first(),
        Some(NetworkNotice::Info(message))
            if message.contains("retreated") && message.contains("Supplies failed the road.")
    ));
}
