use super::*;
use tarrowyn_protocol::PlayerProjection;

#[test]
fn knocked_out_input_waits_for_a_visible_recovery_prompt() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client
        .projection
        .set_authoritative_player_position(TilePos::new(8, 6));
    client.projection.player = Some(PlayerProjection {
        account_id: "account-1".to_owned(),
        character_id: "character-1".to_owned(),
        display_name: "Traveller".to_owned(),
        position: Position { x: 8, y: 6 },
        gold: 12,
        field_tool_condition: 3,
        field_weather: tarrowyn_protocol::FieldWeather::Clear,
        field_pest_pressure: 0,
        animal_condition: 10,
        animal_max_condition: 10,
        skill: 1,
        reputation: 0,
        adventurer_rank: tarrowyn_protocol::AdventurerRank::Unproven,
        adventurer_credentials: Vec::new(),
        inventory: tarrowyn_protocol::Inventory::default(),
        weapon: tarrowyn_protocol::WeaponKind::IronSword,
        knocked_out: true,
        injuries: 1,
        recovery_cost: 2,
    });

    client.queue_movement(1, 0);

    assert!(client.movement_queue.is_empty());
    assert_eq!(
        client.status_message,
        "Choose a recovery prompt before walking."
    );
}
