use super::*;
use crate::data::GameConfig;
use crate::network::ConnectionState;
use crate::state::{CropKind, CropState};
use macroquad_toolkit::grid::TilePos;
use macroquad_toolkit::net::Pending;
use tarrowyn_protocol::{ChatRequest, FarmingAction, FarmingRequest, TradeAction, TradeRequest};

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
    client.state = ConnectionState::Online;
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
    assert!(client.state_reload_pending);
    assert!(!client.had_world);
    assert!(client.pending_state.is_none());
    client.queue_chat("This must wait for the restored road snapshot");
    assert!(client.chat_queue.is_empty());
    client.dispatch_requests();
    assert!(client.pending_state.is_some());
    assert!(client.pending_events.is_none());
    assert!(client.status_message.contains("reloading"));
    assert!(notices.iter().any(|notice| matches!(
        notice,
        NetworkNotice::Warning(message) if message.contains("shared history window changed")
    )));
}

#[test]
fn restore_recovery_cancels_stale_low_level_mutations() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.pending_movement = Some(crate::network::PendingMovement {
        pending: Some(Pending::failed("movement before restore")),
        request: tarrowyn_protocol::MovementIntent {
            request_id: "movement-before-restore".to_owned(),
            dx: 1,
            dy: 0,
        },
        retries: 0,
        retry_timer: 0.0,
    });
    client.pending_chat = Some(crate::network::PendingChat {
        pending: Some(Pending::failed("chat before restore")),
        request: ChatRequest {
            request_id: "chat-before-restore".to_owned(),
            channel: "settlement".to_owned(),
            text: "Before restore".to_owned(),
        },
        retries: 0,
        retry_timer: 0.0,
    });
    client.pending_farming = Some(crate::network::PendingFarming {
        pending: Some(Pending::failed("farming before restore")),
        request: FarmingRequest {
            request_id: "farming-before-restore".to_owned(),
            action: FarmingAction::Tend,
            position: tarrowyn_protocol::Position { x: 1, y: 1 },
        },
        retries: 0,
        retry_timer: 0.0,
    });
    client.pending_trade = Some(crate::network::PendingTrade {
        pending: Some(Pending::failed("trade before restore")),
        request: TradeRequest {
            request_id: "trade-before-restore".to_owned(),
            action: TradeAction::Review,
            trade_id: None,
            recipient_account_id: None,
            offer: None,
            request: None,
        },
        retries: 0,
        retry_timer: 0.0,
    });
    client
        .movement_queue
        .push_back(tarrowyn_protocol::MovementIntent {
            request_id: "queued-movement-before-restore".to_owned(),
            dx: -1,
            dy: 0,
        });
    client.chat_queue.push_back(ChatRequest {
        request_id: "queued-chat-before-restore".to_owned(),
        channel: "settlement".to_owned(),
        text: "Queued before restore".to_owned(),
    });
    client.farming_queue.push_back(FarmingRequest {
        request_id: "queued-farming-before-restore".to_owned(),
        action: FarmingAction::Plant,
        position: tarrowyn_protocol::Position { x: 0, y: 1 },
    });
    client.trade_queue.push_back(TradeRequest {
        request_id: "queued-trade-before-restore".to_owned(),
        action: TradeAction::Cancel,
        trade_id: Some("trade".to_owned()),
        recipient_account_id: None,
        offer: None,
        request: None,
    });
    client.pending_request_type = Some("trade::Review".to_owned());
    client.pending_request_id = Some("trade-before-restore".to_owned());
    client.pending_trade_action = Some(TradeAction::Review);
    client.action_awaiting_confirmation = true;
    let mut notices = Vec::new();

    recover_from_cursor_boundary(&mut client, &mut notices);

    assert!(client.pending_movement.is_none());
    assert!(client.pending_chat.is_none());
    assert!(client.pending_farming.is_none());
    assert!(client.pending_trade.is_none());
    assert!(client.movement_queue.is_empty());
    assert!(client.chat_queue.is_empty());
    assert!(client.farming_queue.is_empty());
    assert!(client.trade_queue.is_empty());
    assert!(client.pending_request_type.is_none());
    assert!(client.pending_request_id.is_none());
    assert!(client.pending_trade_action.is_none());
    assert!(!client.action_awaiting_confirmation);
}
