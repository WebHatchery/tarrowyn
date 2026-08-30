use super::*;

#[test]
fn trade_backpressure_does_not_claim_pending_confirmation() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    for index in 0..super::super::queue::MAX_PENDING_COMMANDS {
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
    assert_eq!(
        client.trade_queue.len(),
        super::super::queue::MAX_PENDING_COMMANDS
    );
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
fn trade_controls_reject_duplicate_targets_but_keep_distinct_actions_queued() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.queue_trade(TradeRequest {
        request_id: "first-trade".to_owned(),
        action: TradeAction::Create,
        trade_id: None,
        recipient_account_id: Some("friend".to_owned()),
        offer: Some(Default::default()),
        request: Some(Default::default()),
    });
    assert!(client.trade_pending());

    client.queue_trade(TradeRequest {
        request_id: "duplicate-create".to_owned(),
        action: TradeAction::Create,
        trade_id: None,
        recipient_account_id: Some("friend".to_owned()),
        offer: Some(Default::default()),
        request: Some(Default::default()),
    });
    assert_eq!(client.trade_queue.len(), 1);
    assert!(client.status_message.contains("already waiting"));

    client.queue_trade(TradeRequest {
        request_id: "distinct-trade".to_owned(),
        action: TradeAction::Accept,
        trade_id: Some("other-trade".to_owned()),
        recipient_account_id: None,
        offer: None,
        request: None,
    });
    assert_eq!(client.trade_queue.len(), 2);

    client.dispatch_trade_requests();
    assert!(client.trade_pending());
    client.queue_trade(TradeRequest {
        request_id: "duplicate-in-flight".to_owned(),
        action: TradeAction::Create,
        trade_id: None,
        recipient_account_id: Some("friend".to_owned()),
        offer: Some(Default::default()),
        request: Some(Default::default()),
    });
    assert_eq!(client.trade_queue.len(), 1);
    assert!(client.status_message.contains("already waiting"));
}

#[test]
fn trade_success_notice_describes_the_requested_action() {
    assert_eq!(
        super::super::trade_client::trade_success_message(
            Some(tarrowyn_protocol::TradeAction::Create),
            None,
        ),
        "The trade offer is on the ledger; awaiting the other player."
    );
    assert_eq!(
        super::super::trade_client::trade_success_message(
            Some(tarrowyn_protocol::TradeAction::Review),
            None,
        ),
        "The trade details are current."
    );
    assert_eq!(
        super::super::trade_client::trade_success_message(
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
        super::super::trade_client::trade_success_message(
            Some(tarrowyn_protocol::TradeAction::Accept),
            Some(&trade),
        ),
        "Trade completed with Mara; exchanged 1 moonberry, 3 gold for 2 wheat, 1 seed."
    );
}
