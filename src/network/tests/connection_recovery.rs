use super::*;

#[test]
fn connection_failure_discards_in_flight_low_level_requests_and_indicators() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.had_world = true;
    client.pending_state = Some(Pending::failed("state unavailable"));
    client.pending_events = Some(Pending::failed("events unavailable"));
    client.pending_trades = Some(Pending::failed("trades unavailable"));
    client.pending_foundation_journey = Some(Pending::failed("journey unavailable"));
    client.pending_movement = Some(PendingMovement {
        pending: Some(Pending::failed("movement unavailable")),
        request: MovementIntent {
            request_id: "movement-in-flight".to_owned(),
            dx: 1,
            dy: 0,
        },
        retries: 0,
        retry_timer: 0.0,
    });
    client.pending_chat = Some(PendingChat {
        pending: Some(Pending::failed("chat unavailable")),
        request: ChatRequest {
            request_id: "chat-in-flight".to_owned(),
            channel: "settlement".to_owned(),
            text: "Hello".to_owned(),
        },
        retries: 0,
        retry_timer: 0.0,
    });
    client.pending_farming = Some(PendingFarming {
        pending: Some(Pending::failed("farming unavailable")),
        request: FarmingRequest {
            request_id: "farming-in-flight".to_owned(),
            action: FarmingAction::Tend,
            position: Position { x: 1, y: 1 },
        },
        retries: 0,
        retry_timer: 0.0,
    });
    client.pending_trade = Some(PendingTrade {
        pending: Some(Pending::failed("trade unavailable")),
        request: TradeRequest {
            request_id: "trade-in-flight".to_owned(),
            action: TradeAction::Review,
            trade_id: None,
            recipient_account_id: None,
            offer: None,
            request: None,
        },
        retries: 0,
        retry_timer: 0.0,
    });
    client.movement_queue.push_back(MovementIntent {
        request_id: "movement-queued".to_owned(),
        dx: -1,
        dy: 0,
    });
    client.chat_queue.push_back(ChatRequest {
        request_id: "chat-queued".to_owned(),
        channel: "settlement".to_owned(),
        text: "Queued".to_owned(),
    });
    client.farming_queue.push_back(FarmingRequest {
        request_id: "farming-queued".to_owned(),
        action: FarmingAction::Plant,
        position: Position { x: 0, y: 1 },
    });
    client.trade_queue.push_back(TradeRequest {
        request_id: "trade-queued".to_owned(),
        action: TradeAction::Cancel,
        trade_id: Some("trade".to_owned()),
        recipient_account_id: None,
        offer: None,
        request: None,
    });
    client.pending_request_type = Some("farming::Tend".to_owned());
    client.pending_request_id = Some("farming-in-flight".to_owned());
    client.pending_trade_action = Some(TradeAction::Review);
    client.action_awaiting_confirmation = true;
    let mut notices = Vec::new();

    client.connection_failed("the shared road stopped answering".to_owned(), &mut notices);

    assert_eq!(client.state, ConnectionState::Degraded);
    assert!(client.pending_state.is_none());
    assert!(client.pending_events.is_none());
    assert!(client.pending_trades.is_none());
    assert!(client.pending_foundation_journey.is_none());
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
