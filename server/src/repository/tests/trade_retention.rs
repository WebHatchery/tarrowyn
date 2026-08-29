use super::*;
use tarrowyn_protocol::{TradeOffer, TradeStatus};

fn trade(
    creator: &GuestSessionResponse,
    recipient: &GuestSessionResponse,
    index: u64,
    status: TradeStatus,
) -> TradeOffer {
    TradeOffer {
        trade_id: format!("trade-{index}"),
        creator_account_id: creator.account_id.clone(),
        creator_name: "Creator".to_owned(),
        recipient_account_id: recipient.account_id.clone(),
        recipient_name: "Recipient".to_owned(),
        offer: TradeBundle::default(),
        request: TradeBundle::default(),
        status,
        created_tick: index,
        expires_tick: index + 100,
    }
}

#[test]
fn trade_history_evicts_terminal_records_and_preserves_pending_work() {
    let repo = repo();
    let creator = guest(&repo, "trade-retention-creator");
    let recipient = guest(&repo, "trade-retention-recipient");
    {
        let mut state = repo.state.lock().expect("world repository lock poisoned");
        for index in 0..128 {
            state.trades.insert(
                format!("trade-{index}"),
                trade(&creator, &recipient, index, TradeStatus::Accepted),
            );
        }
        state.next_trade = 128;
    }

    let created = repo
        .trade(
            &creator.account_token,
            TradeRequest {
                request_id: "trade-retention-create".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(recipient.account_id.clone()),
                offer: Some(TradeBundle::default()),
                request: Some(TradeBundle::default()),
            },
        )
        .expect("trade creation")
        .data;
    assert!(created.accepted);
    assert!(created.trade.is_some());
    let state = repo.state.lock().expect("world repository lock poisoned");
    assert_eq!(state.trades.len(), 128);
    assert!(!state.trades.contains_key("trade-0"));
    assert!(state.trades.contains_key("trade-128"));
    drop(state);

    {
        let mut state = repo.state.lock().expect("world repository lock poisoned");
        state.trades.clear();
        for index in 0..128 {
            state.trades.insert(
                format!("trade-{index}"),
                trade(&creator, &recipient, index, TradeStatus::Pending),
            );
        }
        state.next_trade = 128;
    }
    let rejected = repo
        .trade(
            &creator.account_token,
            TradeRequest {
                request_id: "trade-retention-full".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(recipient.account_id),
                offer: Some(TradeBundle::default()),
                request: Some(TradeBundle::default()),
            },
        )
        .expect("trade capacity response")
        .data;
    assert!(!rejected.accepted);
    assert!(rejected.reason.unwrap().contains("ledger is full"));
    let state = repo.state.lock().expect("world repository lock poisoned");
    assert_eq!(state.trades.len(), 128);
    assert!(!state.trades.contains_key("trade-128"));
}
