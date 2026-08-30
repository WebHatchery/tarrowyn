use super::*;
use tarrowyn_protocol::{PlayerPresence, PlayerProjection, Position};

#[test]
fn player_position_is_unresolved_until_a_server_update_arrives() {
    let mut projection = WorldProjection::new(&config());

    assert_eq!(projection.authoritative_player_position(), None);

    projection.apply_presence(
        PlayerPresence {
            account_id: "account-1".to_owned(),
            character_id: "character-1".to_owned(),
            display_name: "Traveller".to_owned(),
            position: Position { x: 12, y: 4 },
            last_seen_tick: 1,
            online: true,
        },
        "account-1",
    );
    assert_eq!(
        projection.authoritative_player_position(),
        Some(TilePos::new(12, 4))
    );

    projection.forget_authoritative_player_position();
    assert_eq!(projection.authoritative_player_position(), None);
}

#[test]
fn authoritative_presence_keeps_player_projection_location_in_sync() {
    let mut projection = WorldProjection::new(&config());
    projection.player = Some(PlayerProjection {
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
        knocked_out: false,
        injuries: 0,
        recovery_cost: 0,
    });

    projection.apply_presence(
        PlayerPresence {
            account_id: "account-1".to_owned(),
            character_id: "character-1".to_owned(),
            display_name: "Traveller".to_owned(),
            position: Position { x: 12, y: 4 },
            last_seen_tick: 1,
            online: true,
        },
        "account-1",
    );

    assert_eq!(
        projection.player.as_ref().map(|player| player.position),
        Some(Position { x: 12, y: 4 })
    );
}

#[test]
fn offline_presence_does_not_authorize_player_movement() {
    let mut projection = WorldProjection::new(&config());

    projection.apply_presence(
        PlayerPresence {
            account_id: "account-1".to_owned(),
            character_id: "character-1".to_owned(),
            display_name: "Traveller".to_owned(),
            position: Position { x: 12, y: 4 },
            last_seen_tick: 2,
            online: false,
        },
        "account-1",
    );

    assert_eq!(projection.authoritative_player_position(), None);

    projection.apply_presence(
        PlayerPresence {
            account_id: "account-1".to_owned(),
            character_id: "character-1".to_owned(),
            display_name: "Traveller".to_owned(),
            position: Position { x: 10, y: 5 },
            last_seen_tick: 3,
            online: true,
        },
        "account-1",
    );
    assert_eq!(
        projection.authoritative_player_position(),
        Some(TilePos::new(10, 5))
    );

    projection.apply_presence(
        PlayerPresence {
            account_id: "account-1".to_owned(),
            character_id: "character-1".to_owned(),
            display_name: "Traveller".to_owned(),
            position: Position { x: 10, y: 5 },
            last_seen_tick: 4,
            online: false,
        },
        "account-1",
    );

    assert_eq!(projection.authoritative_player_position(), None);
}
