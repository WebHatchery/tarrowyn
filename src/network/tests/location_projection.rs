use super::*;
use tarrowyn_protocol::{PlayerPresence, Position};

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
}
