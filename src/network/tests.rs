use super::*;
use crate::data::GameConfig;
use macroquad_toolkit::net::Pending;
use tarrowyn_protocol::{
    ChatRequest, ChronicleEntry, CropKind, CropState, EventRecord, EventsResponse, FarmAnimal,
    FarmAnimalKind, FarmPlot, FarmingAction, FarmingRequest, MovementIntent, Position,
    TileKind as ProtocolTileKind, TradeAction, TradeRequest, WorldClock, WorldEvent, WorldTile,
};

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
        starting_seeds: 6,
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
fn chronicle_cache_keeps_the_newest_entries_after_incremental_events() {
    let mut projection = WorldProjection::new(&config());
    let events = (0..(super::chronicle::MAX_CACHED_CHRONICLE + 1))
        .map(|index| {
            let cursor = (index + 1) as u64;
            EventRecord {
                cursor,
                event: WorldEvent::Chronicle(ChronicleEntry {
                    event_id: format!("chronicle-{index}"),
                    kind: "settlement".to_owned(),
                    title: format!("Settlement record {index}"),
                    text: "The latest work remains visible.".to_owned(),
                    created_tick: cursor,
                    cursor,
                }),
            }
        })
        .collect();
    projection.apply_events(
        EventsResponse {
            cursor: super::chronicle::MAX_CACHED_CHRONICLE as u64 + 1,
            clock: WorldClock {
                day: 1,
                seconds: 0.0,
                day_length_seconds: 180.0,
            },
            events,
        },
        "account",
        1,
    );

    assert_eq!(
        projection.chronicle.len(),
        super::chronicle::MAX_CACHED_CHRONICLE
    );
    assert_eq!(projection.chronicle[0].event_id, "chronicle-1");
    assert_eq!(
        projection.chronicle.last().unwrap().event_id,
        "chronicle-12"
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
fn projection_versions_accept_equal_state_but_reject_older_responses() {
    let mut projection = WorldProjection::new(&config());
    projection.record_response_version(12, Some(9));

    assert!(!projection.response_is_current(11, 8));
    assert!(projection.response_is_current(12, 9));
    assert!(projection.response_is_newer(12, 10));
    assert!(!projection.response_is_newer(12, 9));
    assert!(!projection.accept_response_version(11, Some(9)));
    assert_eq!(projection.server_tick, 12);
    assert_eq!(projection.cursor, 9);
    assert!(projection.accept_response_version(12, Some(9)));
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
fn explicit_logout_forgets_a_linked_key_before_guest_reconnect() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.client_key = Some("linked-client".to_owned());
    client.account = Some(tarrowyn_protocol::GuestSessionResponse {
        client_key: "linked-client".to_owned(),
        account_id: "account-1".to_owned(),
        character_id: "character-1".to_owned(),
        display_name: "Linked traveller".to_owned(),
        account_token: "access".to_owned(),
        expires_in_seconds: 900,
    });

    client.clear_logged_out_session();

    assert!(client.client_key.is_none());
    assert!(client.account.is_none());
    assert_eq!(client.state, ConnectionState::Degraded);
    assert!(client.status_message.contains("fresh guest fixture"));
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
fn authenticated_reads_wait_for_a_same_frame_refresh_boundary() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client
        .phase4
        .prime_refresh_for_test(&mut client.api, &mut client.projection);
    client.trade_queue.push_back(TradeRequest {
        request_id: "refresh-read-boundary".to_owned(),
        action: TradeAction::Review,
        trade_id: None,
        recipient_account_id: None,
        offer: None,
        request: None,
    });

    client.dispatch_requests();
    client.dispatch_trade_requests();
    client.frontier.dispatch(&mut client.api, true, 0, true);

    assert!(client.pending_state.is_none());
    assert!(client.pending_events.is_none());
    assert!(client.pending_trades.is_none());
    assert!(client.frontier.pending_contracts.is_none());
    assert!(client.frontier.pending_chronicle.is_none());
    assert!(client.frontier.pending_opportunities.is_none());
    assert!(client.frontier.pending_command.is_none());
    assert_eq!(client.trade_queue.len(), 1);
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
fn movement_and_chat_backpressure_explain_the_retry_path() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    for index in 0..super::queue::MAX_PENDING_COMMANDS {
        client.movement_queue.push_back(super::MovementIntent {
            request_id: format!("move-{index}"),
            dx: 1,
            dy: 0,
        });
    }

    client.queue_movement(1, 0);

    assert_eq!(
        client.movement_queue.len(),
        super::queue::MAX_PENDING_COMMANDS
    );
    assert!(client.status_message.contains("movement queue is full"));

    for index in 0..super::queue::MAX_PENDING_COMMANDS {
        client.chat_queue.push_back(ChatRequest {
            request_id: format!("chat-{index}"),
            channel: "settlement".to_owned(),
            text: "queued".to_owned(),
        });
    }

    client.queue_chat("try again");

    assert_eq!(client.chat_queue.len(), super::queue::MAX_PENDING_COMMANDS);
    assert!(client.status_message.contains("chat channel is busy"));
}

#[test]
fn farming_without_a_nearby_target_explains_where_to_go() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.projection.world.tiles = FlatGrid::new(3, 2, TileKind::Meadow);
    client.projection.player_position = TilePos::new(1, 1);

    client.queue_farming(FarmingAction::Plant);

    assert!(client.farming_queue.is_empty());
    assert!(client.status_message.contains("shared field plot"));
}

#[test]
fn farming_actions_choose_a_nearby_plot_matching_the_action() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.projection.world.tiles = FlatGrid::new(3, 2, TileKind::Field);
    client.projection.player_position = TilePos::new(1, 1);
    client
        .projection
        .world
        .tiles
        .set(TilePos::new(1, 1), TileKind::Meadow);
    client.projection.world.crops.set(
        TilePos::new(0, 1),
        Some(crate::state::CropState {
            kind: crate::state::CropKind::Wheat,
            stage: crate::state::CropState::MATURE_STAGE,
        }),
    );
    client.projection.world.crops.set(
        TilePos::new(1, 0),
        Some(crate::state::CropState {
            kind: crate::state::CropKind::Turnip,
            stage: 1,
        }),
    );

    client.queue_farming(FarmingAction::Harvest);
    assert_eq!(
        client.farming_queue.back().map(|request| request.position),
        Some(Position { x: 0, y: 1 })
    );
    client.farming_queue.clear();

    client.queue_farming(FarmingAction::Tend);
    assert_eq!(
        client.farming_queue.back().map(|request| request.position),
        Some(Position { x: 1, y: 0 })
    );
    client.farming_queue.clear();

    client.queue_farming(FarmingAction::Plant);
    assert_eq!(
        client.farming_queue.back().map(|request| request.position),
        Some(Position { x: 2, y: 1 })
    );
}

#[test]
fn farming_success_notice_names_crop_and_plot() {
    let plot = FarmPlot {
        position: Position { x: 2, y: 3 },
        crop: Some(CropState {
            kind: CropKind::Turnip,
            stage: 2,
            quality: 2,
            planted_tick: 4,
            last_tended_tick: Some(5),
        }),
    };

    assert_eq!(
        super::commands::farming_success_notice(FarmingAction::Tend, Some(plot), None),
        "Tended Turnip at plot (2, 3); growth stage 2/3."
    );
    assert_eq!(
        super::commands::farming_success_notice(FarmingAction::Harvest, Some(plot), None),
        "Harvested Turnip from plot (2, 3)."
    );
}

#[test]
fn farming_success_notice_names_animal_condition() {
    let animal = FarmAnimal {
        animal_id: "bellweather-goat".to_owned(),
        name: "Bellweather".to_owned(),
        kind: FarmAnimalKind::Goat,
        position: Position { x: 1, y: 1 },
        condition: 3,
        max_condition: 3,
        last_cared_tick: 8,
        last_cared_day: 2,
    };

    assert_eq!(
        super::commands::farming_success_notice(FarmingAction::TendAnimal, None, Some(&animal)),
        "Cared for Bellweather • condition 3/3."
    );
}

#[test]
fn animal_care_targets_the_nearby_animal() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.projection.player_position = TilePos::new(1, 1);
    client.projection.animals = vec![FarmAnimal {
        animal_id: "bellweather-goat".to_owned(),
        name: "Bellweather".to_owned(),
        kind: tarrowyn_protocol::FarmAnimalKind::Goat,
        position: Position { x: 1, y: 1 },
        condition: 2,
        max_condition: 3,
        last_cared_tick: 0,
        last_cared_day: 1,
    }];

    client.queue_farming(FarmingAction::TendAnimal);

    assert!(matches!(
        client.farming_queue.front(),
        Some(request) if request.action == FarmingAction::TendAnimal
            && request.position == Position { x: 1, y: 1 }
    ));
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

#[test]
fn trade_projection_selects_pending_and_incoming_offers_for_visible_actions() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.projection.trades = vec![
        tarrowyn_protocol::TradeOffer {
            trade_id: "outgoing".to_owned(),
            creator_account_id: "me".to_owned(),
            creator_name: "Me".to_owned(),
            recipient_account_id: "friend".to_owned(),
            recipient_name: "Friend".to_owned(),
            offer: Default::default(),
            request: Default::default(),
            status: tarrowyn_protocol::TradeStatus::Pending,
            created_tick: 1,
            expires_tick: 2,
        },
        tarrowyn_protocol::TradeOffer {
            trade_id: "incoming".to_owned(),
            creator_account_id: "friend".to_owned(),
            creator_name: "Friend".to_owned(),
            recipient_account_id: "me".to_owned(),
            recipient_name: "Me".to_owned(),
            offer: Default::default(),
            request: Default::default(),
            status: tarrowyn_protocol::TradeStatus::Pending,
            created_tick: 3,
            expires_tick: 4,
        },
    ];

    assert_eq!(
        client
            .pending_trade_for("me")
            .map(|trade| trade.trade_id.as_str()),
        Some("outgoing")
    );
    assert_eq!(
        client
            .incoming_trade_for("me")
            .map(|trade| trade.trade_id.as_str()),
        Some("incoming")
    );
    assert!(client.pending_trade_for("stranger").is_none());
}

#[test]
fn trade_metadata_waits_for_dispatch_and_keeps_queue_order() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.queue_trade(TradeRequest {
        request_id: "first-trade".to_owned(),
        action: tarrowyn_protocol::TradeAction::Create,
        trade_id: None,
        recipient_account_id: Some("friend".to_owned()),
        offer: Some(Default::default()),
        request: Some(Default::default()),
    });
    client.queue_trade(TradeRequest {
        request_id: "second-trade".to_owned(),
        action: tarrowyn_protocol::TradeAction::Accept,
        trade_id: Some("trade-2".to_owned()),
        recipient_account_id: None,
        offer: None,
        request: None,
    });

    assert!(client.pending_trade_action.is_none());
    assert!(client.pending_request_id.is_none());

    client.dispatch_trade_requests();

    assert_eq!(
        client.pending_trade_action,
        Some(tarrowyn_protocol::TradeAction::Create)
    );
    assert_eq!(client.pending_request_id.as_deref(), Some("first-trade"));
    assert_eq!(
        client.pending_request_type.as_deref(),
        Some("trade::Create")
    );
    assert_eq!(client.trade_queue.len(), 1);
}

#[test]
fn trade_success_notice_describes_the_requested_action() {
    assert_eq!(
        super::trade_client::trade_success_message(
            Some(tarrowyn_protocol::TradeAction::Create),
            None,
        ),
        "The trade offer is on the ledger; awaiting the other player."
    );
    assert_eq!(
        super::trade_client::trade_success_message(
            Some(tarrowyn_protocol::TradeAction::Review),
            None,
        ),
        "The trade details are current."
    );
    assert_eq!(
        super::trade_client::trade_success_message(
            Some(tarrowyn_protocol::TradeAction::Accept),
            None,
        ),
        "The trade ledger completed the exchange."
    );
}

#[test]
fn trade_success_notice_explains_the_accepted_exchange() {
    let trade = tarrowyn_protocol::TradeOffer {
        trade_id: "trade-1".to_owned(),
        creator_account_id: "account-1".to_owned(),
        creator_name: "Mara".to_owned(),
        recipient_account_id: "account-2".to_owned(),
        recipient_name: "The traveller".to_owned(),
        offer: tarrowyn_protocol::TradeBundle {
            wheat: 2,
            turnips: 0,
            moonberries: 0,
            seeds: 1,
            gold: 0,
        },
        request: tarrowyn_protocol::TradeBundle {
            wheat: 0,
            turnips: 0,
            moonberries: 1,
            seeds: 0,
            gold: 3,
        },
        status: tarrowyn_protocol::TradeStatus::Accepted,
        created_tick: 2,
        expires_tick: 8,
    };

    assert_eq!(
        super::trade_client::trade_success_message(
            Some(tarrowyn_protocol::TradeAction::Accept),
            Some(&trade),
        ),
        "Trade completed with Mara; exchanged 1 moonberry, 3 gold for 2 wheat, 1 seed."
    );
}

#[test]
fn client_request_id_stays_at_the_numeric_ceiling() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.next_request_id = u64::MAX;

    assert_eq!(
        client.next_request_id("boundary"),
        format!("boundary-{}", u64::MAX)
    );
    assert_eq!(
        client.next_request_id("boundary"),
        format!("boundary-{}", u64::MAX)
    );
    assert_eq!(client.next_request_id, u64::MAX);
}

#[test]
fn transient_low_level_commands_replay_the_same_request_ids() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.pending_movement = Some(super::PendingMovement {
        pending: Some(Pending::failed(
            "HTTP request 'POST /v1/movement' failed: connection reset",
        )),
        request: MovementIntent {
            request_id: "move-1".to_owned(),
            dx: 1,
            dy: 0,
        },
        retries: 0,
        retry_timer: 0.0,
    });
    client.pending_chat = Some(super::PendingChat {
        pending: Some(Pending::failed(
            "HTTP request 'POST /v1/chat' failed: connection reset",
        )),
        request: ChatRequest {
            request_id: "chat-1".to_owned(),
            channel: "settlement".to_owned(),
            text: "Still here?".to_owned(),
        },
        retries: 0,
        retry_timer: 0.0,
    });
    client.pending_farming = Some(super::PendingFarming {
        pending: Some(Pending::failed(
            "HTTP request 'POST /v1/farming/actions' failed: connection reset",
        )),
        request: FarmingRequest {
            request_id: "farm-1".to_owned(),
            action: FarmingAction::Tend,
            position: Position { x: 1, y: 1 },
        },
        retries: 0,
        retry_timer: 0.0,
    });
    client.action_awaiting_confirmation = true;
    client.pending_request_id = Some("farm-1".to_owned());
    client.pending_request_type = Some("farming::Tend".to_owned());
    client.pending_trade = Some(super::PendingTrade {
        pending: Some(Pending::failed(
            "HTTP request 'POST /v1/trades' failed: connection reset",
        )),
        request: TradeRequest {
            request_id: "trade-1".to_owned(),
            action: TradeAction::Review,
            trade_id: Some("trade".to_owned()),
            recipient_account_id: None,
            offer: None,
            request: None,
        },
        retries: 0,
        retry_timer: 0.0,
    });
    client.pending_trade_action = Some(TradeAction::Review);

    let mut notices = Vec::new();
    client.poll_movement(0.0, &mut notices);
    client.poll_chat(0.0, &mut notices);
    client.poll_farming(0.0, &mut notices);
    client.poll_trade_requests(0.0, &mut notices);

    assert_eq!(client.pending_movement.as_ref().unwrap().retries, 1);
    assert_eq!(
        client.pending_movement.as_ref().unwrap().retry_timer,
        super::commands::COMMAND_RETRY_DELAY_SECONDS
    );
    assert_eq!(
        client.pending_movement.as_ref().unwrap().request.request_id,
        "move-1"
    );
    assert_eq!(client.pending_chat.as_ref().unwrap().retries, 1);
    assert_eq!(
        client.pending_chat.as_ref().unwrap().retry_timer,
        super::commands::COMMAND_RETRY_DELAY_SECONDS
    );
    assert_eq!(
        client.pending_chat.as_ref().unwrap().request.request_id,
        "chat-1"
    );
    assert_eq!(client.pending_farming.as_ref().unwrap().retries, 1);
    assert_eq!(
        client.pending_farming.as_ref().unwrap().retry_timer,
        super::commands::COMMAND_RETRY_DELAY_SECONDS
    );
    assert_eq!(
        client.pending_farming.as_ref().unwrap().request.request_id,
        "farm-1"
    );
    assert!(client.action_awaiting_confirmation);
    assert_eq!(client.pending_request_id.as_deref(), Some("farm-1"));
    assert_eq!(client.pending_trade.as_ref().unwrap().retries, 1);
    assert_eq!(
        client.pending_trade.as_ref().unwrap().retry_timer,
        super::commands::COMMAND_RETRY_DELAY_SECONDS
    );
    assert_eq!(
        client.pending_trade.as_ref().unwrap().request.request_id,
        "trade-1"
    );
    assert_eq!(client.pending_trade_action, Some(TradeAction::Review));
    assert_eq!(notices.len(), 4);
}
