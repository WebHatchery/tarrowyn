use super::*;
use crate::data::GameConfig;
use tarrowyn_protocol::{Position, TileKind as ProtocolTileKind, WorldTile};

fn config() -> GameConfig {
    GameConfig {
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
fn projection_adopts_server_tiles_without_local_movement_validation() {
    let mut projection = WorldProjection::new(&config());
    projection.world.tiles = FlatGrid::new(3, 2, TileKind::Meadow);
    let tile = WorldTile {
        position: Position { x: 1, y: 0 },
        kind: ProtocolTileKind::Water,
    };
    projection.world.tiles.set(
        TilePos::new(tile.position.x, tile.position.y),
        from_protocol_tile(tile.kind),
    );
    assert_eq!(
        projection.world.tiles.get(TilePos::new(1, 0)),
        Some(&TileKind::Water)
    );
}

#[test]
fn stale_presence_is_visible_after_server_ticks_age() {
    let player = RemotePlayer {
        account_id: "a".to_owned(),
        character_id: "c".to_owned(),
        display_name: "Guest".to_owned(),
        position: TilePos::new(0, 0),
        last_seen_tick: 1,
        online: true,
    };
    assert!(player.stale(super::types::STALE_TICKS + 2));
    assert!(!player.stale(super::types::STALE_TICKS));
}

#[test]
fn connection_failure_exposes_recovery_and_reconnect_cooldown() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    let mut notices = Vec::new();

    client.connection_failed("server is unavailable".to_owned(), &mut notices);
    assert_eq!(client.state, ConnectionState::Offline);
    assert!(client.status_message.contains("unavailable"));
    assert!(notices.iter().any(|notice| matches!(
        notice,
        NetworkNotice::Danger(message) if message.contains("Reconnect is available")
    )));
    assert!(!client.reconnect());

    client.update(2.0);
    assert!(client.reconnect());
    assert_eq!(client.state, ConnectionState::Connecting);

    client.had_world = true;
    client.connection_failed("server stopped".to_owned(), &mut notices);
    assert_eq!(client.state, ConnectionState::Degraded);
    assert!(client.status_message.contains("last shared road"));
}

#[test]
fn online_commands_are_not_queued_while_disconnected() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.queue_movement(1, 0);
    client.queue_chat("This must wait for the road");
    assert!(client.movement_queue.is_empty());
    assert!(client.chat_queue.is_empty());

    client.state = ConnectionState::Online;
    client.queue_movement(1, 0);
    client.queue_chat("The road is open");
    assert_eq!(client.movement_queue.len(), 1);
    assert_eq!(client.chat_queue.len(), 1);
}
