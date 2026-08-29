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
fn projection_exposes_the_server_clock_period_without_local_timekeeping() {
    let mut projection = WorldProjection::new(&config());
    projection.day_seconds = 45.0;

    assert_eq!(projection.clock_minutes(), 12 * 60);
    assert_eq!(
        projection.time_of_day(),
        tarrowyn_protocol::TimeOfDay::Afternoon
    );
    assert!(!projection.is_night());
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
fn session_reset_discards_world_and_frontier_projection_state() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.account = Some(tarrowyn_protocol::GuestSessionResponse {
        client_key: "client".to_owned(),
        account_id: "account".to_owned(),
        character_id: "character".to_owned(),
        display_name: "Traveller".to_owned(),
        account_token: "token".to_owned(),
        expires_in_seconds: 60,
    });
    client.projection.player_position = TilePos::new(2, 1);
    client.projection.day = 9;
    client.projection.server_tick = 24;
    client.projection.cursor = 18;
    client
        .frontier
        .contracts
        .push(tarrowyn_protocol::AdventurerContract {
            contract_id: "contract".to_owned(),
            title: "A contract".to_owned(),
            description: "A test contract".to_owned(),
            target: tarrowyn_protocol::MonsterKind::Brambleback,
            progress: 0,
            required_progress: 1,
            reward_gold: 1,
            status: tarrowyn_protocol::ContractStatus::Available,
            completion_count: 0,
            available_at_tick: 0,
        });
    client.trades.push(tarrowyn_protocol::TradeOffer {
        trade_id: "trade".to_owned(),
        creator_account_id: "from".to_owned(),
        creator_name: "From".to_owned(),
        recipient_account_id: "to".to_owned(),
        recipient_name: "To".to_owned(),
        offer: tarrowyn_protocol::TradeBundle {
            seeds: 1,
            ..Default::default()
        },
        request: tarrowyn_protocol::TradeBundle {
            wheat: 1,
            ..Default::default()
        },
        status: tarrowyn_protocol::TradeStatus::Pending,
        created_tick: 1,
        expires_tick: 2,
    });

    client.clear_session_state();

    assert!(client.account.is_none());
    assert_eq!(client.projection.player_position, TilePos::new(2, 1));
    assert_eq!(client.projection.day, 1);
    assert_eq!(client.projection.server_tick, 0);
    assert_eq!(client.projection.cursor, 0);
    assert!(client.frontier.contracts.is_empty());
    assert!(client.trades.is_empty());
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

#[test]
fn client_pending_queues_stop_at_the_backpressure_limit() {
    let mut queue = VecDeque::new();
    for value in 0..(super::queue::MAX_PENDING_COMMANDS + 4) {
        assert_eq!(
            super::queue::try_push(&mut queue, value),
            value < super::queue::MAX_PENDING_COMMANDS
        );
    }
    assert_eq!(queue.len(), super::queue::MAX_PENDING_COMMANDS);
}

#[test]
fn farming_backpressure_does_not_claim_pending_confirmation() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.projection.world.tiles = FlatGrid::new(3, 2, TileKind::Field);
    client.projection.player_position = TilePos::new(1, 1);
    for index in 0..super::queue::MAX_PENDING_COMMANDS {
        client.farming_queue.push_back(FarmingRequest {
            request_id: format!("queued-{index}"),
            action: FarmingAction::Plant,
            position: Position { x: 0, y: 0 },
        });
    }

    client.queue_farming(FarmingAction::Tend);

    assert!(!client.action_awaiting_confirmation);
    assert!(client.pending_request_id.is_none());
    assert!(client.status_message.contains("ledger is busy"));
    assert_eq!(
        client.farming_queue.len(),
        super::queue::MAX_PENDING_COMMANDS
    );
}

#[test]
fn trade_backpressure_does_not_claim_pending_confirmation() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    for index in 0..super::queue::MAX_PENDING_COMMANDS {
        client.trade_queue.push_back(TradeRequest {
            request_id: format!("queued-{index}"),
            action: tarrowyn_protocol::TradeAction::Review,
            trade_id: None,
            recipient_account_id: None,
            offer: None,
            request: None,
        });
    }

    client.queue_trade(TradeRequest {
        request_id: "dropped".to_owned(),
        action: tarrowyn_protocol::TradeAction::Review,
        trade_id: None,
        recipient_account_id: None,
        offer: None,
        request: None,
    });

    assert!(client.pending_request_id.is_none());
    assert!(client.status_message.contains("trade ledger is busy"));
    assert_eq!(client.trade_queue.len(), super::queue::MAX_PENDING_COMMANDS);
}
