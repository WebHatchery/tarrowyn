use super::*;

fn config() -> crate::data::GameConfig {
    crate::data::GameConfig {
        game_name: "years_of_tarrowyn".to_owned(),
        display_name: "The Years of Tarrowyn".to_owned(),
        save_slot: "phase_0".to_owned(),
        version: "0.1.0".to_owned(),
        world_width: 3,
        world_height: 2,
        day_length_seconds: 180.0,
        starting_gold: 12,
        starting_skill: 1,
    }
}

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
