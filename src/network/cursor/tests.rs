use super::*;
use crate::data::GameConfig;
use crate::state::{CropKind, CropState};
use macroquad_toolkit::grid::TilePos;

fn config() -> GameConfig {
    GameConfig {
        game_name: "years_of_tarrowyn".to_owned(),
        display_name: "The Years of Tarrowyn".to_owned(),
        save_slot: "phase_6".to_owned(),
        version: "0.1.0".to_owned(),
        world_width: 3,
        world_height: 2,
        day_length_seconds: 180.0,
        starting_gold: 12,
        starting_seeds: 6,
        starting_skill: 1,
    }
}

#[test]
fn cursor_boundary_detection_accepts_shared_api_and_native_status_shapes() {
    assert!(is_cursor_recovery_error(
        "HTTP API error in 'GET /v1/events?since=9' [cursor_ahead]: The requested cursor is ahead."
    ));
    assert!(is_cursor_recovery_error(
        "HTTP API error in 'GET /v1/events?since=9' [cursor_stale]: The requested history is gone."
    ));
    assert!(is_cursor_recovery_error(
        "HTTP request 'GET /v1/events?since=9' returned status code 409"
    ));
    assert!(is_cursor_recovery_error(
        "HTTP request 'GET /v1/events/region?since=9' returned status code 409"
    ));
    assert!(is_cursor_recovery_error(
        "HTTP API error in 'GET /v1/settlement/chronicle?since=9' [cursor_stale]: The requested history is gone."
    ));
    assert!(!is_cursor_recovery_error(
        "HTTP API error in 'GET /v1/chat' [rate_limited]: Try again later."
    ));
    assert!(!is_cursor_recovery_error(
        "HTTP request 'GET /v1/chat' returned status code 409"
    ));
}

#[test]
fn restore_recovery_discards_stale_history_and_schedules_state_reload() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.had_world = true;
    client.projection.cursor = 99;
    client.projection.server_tick = 44;
    client.projection.world.crops.set(
        TilePos::new(0, 0),
        Some(CropState {
            kind: CropKind::Wheat,
            stage: CropState::MATURE_STAGE,
        }),
    );
    client.projection.world.reachable.insert(TilePos::new(0, 0));
    client.projection.feed.cursor = 99;
    client.projection.chat.push(tarrowyn_protocol::ChatMessage {
        message_id: 1,
        account_id: "account".to_owned(),
        display_name: "Guest".to_owned(),
        channel: "tavern".to_owned(),
        text: "stale".to_owned(),
        cursor: 99,
    });
    client.state_refresh = 4.0;
    let mut notices = Vec::new();

    recover_from_cursor_boundary(&mut client, &mut notices);

    assert_eq!(client.projection.cursor, 0);
    assert_eq!(client.projection.server_tick, 0);
    assert!(client.projection.chat.is_empty());
    assert_eq!(client.projection.feed.cursor, 0);
    assert_eq!(
        client.projection.world.crops.get(TilePos::new(0, 0)),
        Some(&None)
    );
    assert!(client.projection.world.reachable.is_empty());
    assert_eq!(client.state_refresh, 0.0);
    assert!(!client.had_world);
    assert!(client.pending_state.is_none());
    assert!(client.pending_events.is_none());
    assert!(client.status_message.contains("reloading"));
    assert!(notices.iter().any(|notice| matches!(
        notice,
        NetworkNotice::Warning(message) if message.contains("shared history window changed")
    )));
}
