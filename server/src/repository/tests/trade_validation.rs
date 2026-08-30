use super::{guest, repo};
use tarrowyn_protocol::{TradeAction, TradeBundle, TradeRequest};

#[test]
fn trade_rejects_overflowing_item_totals_before_mutation() {
    let repository = repo();
    let creator = guest(&repository, "trade-overflow-creator");
    let recipient = guest(&repository, "trade-overflow-recipient");
    let response = repository
        .trade(
            &creator.account_token,
            TradeRequest {
                request_id: "trade-overflow".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(recipient.account_id),
                offer: Some(TradeBundle {
                    wheat: u32::MAX,
                    turnips: u32::MAX,
                    moonberries: u32::MAX,
                    seeds: u32::MAX,
                    ..TradeBundle::default()
                }),
                request: Some(TradeBundle::default()),
            },
        )
        .expect("oversized trade should return a response")
        .data;
    assert!(!response.accepted);
    assert!(response
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("too large")));
}

#[test]
fn trade_rejects_an_empty_exchange_before_using_ledger_space() {
    let repository = repo();
    let creator = guest(&repository, "trade-empty-creator");
    let recipient = guest(&repository, "trade-empty-recipient");

    let response = repository
        .trade(
            &creator.account_token,
            TradeRequest {
                request_id: "trade-empty".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(recipient.account_id),
                offer: Some(TradeBundle::default()),
                request: Some(TradeBundle::default()),
            },
        )
        .expect("empty trade should return a response")
        .data;

    assert!(!response.accepted);
    assert!(response
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("at least one good or gold")));
    assert!(repository.state.lock().unwrap().trades.is_empty());
}

#[test]
fn accepted_trade_saturates_recipient_counters_at_their_numeric_ceiling() {
    let repository = repo();
    let creator = guest(&repository, "trade-saturating-creator");
    let recipient = guest(&repository, "trade-saturating-recipient");
    let mut state = repository.state.lock().unwrap();
    state
        .identities
        .get_mut("trade-saturating-creator")
        .unwrap()
        .inventory
        .wheat = 1;
    state
        .identities
        .get_mut("trade-saturating-recipient")
        .unwrap()
        .inventory
        .wheat = u32::MAX;
    drop(state);

    let trade = repository
        .trade(
            &creator.account_token,
            TradeRequest {
                request_id: "trade-saturating-create".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(recipient.account_id),
                offer: Some(TradeBundle {
                    wheat: 1,
                    ..TradeBundle::default()
                }),
                request: Some(TradeBundle::default()),
            },
        )
        .unwrap()
        .data
        .trade
        .unwrap();

    let accepted = repository
        .trade(
            &recipient.account_token,
            TradeRequest {
                request_id: "trade-saturating-accept".to_owned(),
                action: TradeAction::Accept,
                trade_id: Some(trade.trade_id),
                recipient_account_id: None,
                offer: None,
                request: None,
            },
        )
        .unwrap()
        .data;
    assert!(accepted.accepted);
    assert_eq!(
        repository
            .state
            .lock()
            .unwrap()
            .identities
            .get("trade-saturating-recipient")
            .unwrap()
            .inventory
            .wheat,
        u32::MAX
    );
}

#[test]
fn trade_expiry_saturates_at_the_world_tick_ceiling() {
    let repository = repo();
    let creator = guest(&repository, "trade-expiry-ceiling-creator");
    let recipient = guest(&repository, "trade-expiry-ceiling-recipient");
    let mut state = repository.state.lock().unwrap();
    state.tick = u64::MAX - 1;
    let tick = state.tick;
    state
        .sessions
        .get_mut(&creator.account_token)
        .unwrap()
        .last_seen_tick = tick;
    state
        .identities
        .get_mut("trade-expiry-ceiling-creator")
        .unwrap()
        .inventory
        .wheat = 1;
    drop(state);

    let trade = repository
        .trade(
            &creator.account_token,
            TradeRequest {
                request_id: "trade-expiry-ceiling-create".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(recipient.account_id),
                offer: Some(TradeBundle {
                    wheat: 1,
                    ..TradeBundle::default()
                }),
                request: Some(TradeBundle::default()),
            },
        )
        .unwrap()
        .data
        .trade
        .expect("trade should be created");

    assert_eq!(trade.created_tick, u64::MAX - 1);
    assert_eq!(trade.expires_tick, u64::MAX);
}
