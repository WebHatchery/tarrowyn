use super::*;

impl WorldRepository {
    pub fn trades(&self, token: &str) -> Result<ApiResponse<TradesResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let identity_key = authenticate(&mut state, token, &self.config)?;
        let account_id = state
            .identities
            .get(&identity_key)
            .expect("identity exists")
            .account_id
            .clone();
        let mut trades: Vec<TradeOffer> = state
            .trades
            .values()
            .filter(|trade| {
                trade.creator_account_id == account_id || trade.recipient_account_id == account_id
            })
            .cloned()
            .collect();
        trades.sort_by_key(|trade| std::cmp::Reverse(trade.created_tick));
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: TradesResponse {
                trades,
                cursor: state.cursor,
            },
        })
    }

    pub fn trade(
        &self,
        token: &str,
        request: TradeRequest,
    ) -> Result<ApiResponse<TradeResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let identity_key = authenticate(&mut state, token, &self.config)?;
        if request.request_id.trim().is_empty() || request.request_id.len() > 64 {
            return Err(RepositoryError::new(
                400,
                "invalid_request_id",
                "Trade request IDs must contain 1 to 64 characters.",
            ));
        }
        if let Some(previous) = state
            .identities
            .get(&identity_key)
            .and_then(|identity| identity.trade_results.get(&request.request_id))
            .cloned()
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous,
            });
        }
        let response = match request.action {
            TradeAction::Create => create_trade(&mut state, &self.config, &identity_key, &request),
            TradeAction::Review => review_trade(&state, &identity_key, &request),
            TradeAction::Accept => accept_trade(&mut state, &identity_key, &request),
            TradeAction::Cancel => cancel_trade(&mut state, &identity_key, &request),
        };
        if response.accepted {
            if let Some(trade) = &response.trade {
                push_event(&mut state, WorldEvent::Trade(trade.clone()));
            }
        }
        state
            .identities
            .get_mut(&identity_key)
            .expect("identity exists")
            .trade_results
            .insert(request.request_id.clone(), response.clone());
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }
}

pub(super) fn expire_trades(state: &mut RepositoryState) {
    let expired: Vec<String> = state
        .trades
        .iter()
        .filter(|(_, trade)| {
            trade.status == TradeStatus::Pending && trade.expires_tick <= state.tick
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in expired {
        if let Some(trade) = state.trades.get_mut(&id) {
            trade.status = TradeStatus::Expired;
            let event = trade.clone();
            push_event(state, WorldEvent::Trade(event));
        }
    }
}

fn create_trade(
    state: &mut RepositoryState,
    config: &ServerConfig,
    identity_key: &str,
    request: &TradeRequest,
) -> TradeResponse {
    let Some(recipient_account_id) = request.recipient_account_id.clone() else {
        return rejected_trade(request, "Choose a settlement player for the trade.");
    };
    let Some(offer) = request.offer else {
        return rejected_trade(request, "The offer is missing.");
    };
    let Some(wanted) = request.request else {
        return rejected_trade(request, "The requested goods are missing.");
    };
    if offer.item_count() > MAX_TRADE_ITEMS
        || wanted.item_count() > MAX_TRADE_ITEMS
        || offer.gold > 10_000
        || wanted.gold > 10_000
    {
        return rejected_trade(request, "A trade is too large for the Hearth ledger.");
    }
    let creator = state.identities.get(identity_key).expect("identity exists");
    if creator.account_id == recipient_account_id || !has_bundle(creator, offer) {
        return rejected_trade(request, "You do not have the offered goods or gold.");
    }
    let Some(recipient) = state
        .identities
        .values()
        .find(|identity| identity.account_id == recipient_account_id)
    else {
        return rejected_trade(request, "That player is not known to the settlement.");
    };
    let trade = TradeOffer {
        trade_id: format!("trade-{}", state.next_trade),
        creator_account_id: creator.account_id.clone(),
        creator_name: creator.display_name.clone(),
        recipient_account_id,
        recipient_name: recipient.display_name.clone(),
        offer,
        request: wanted,
        status: TradeStatus::Pending,
        created_tick: state.tick,
        expires_tick: state.tick + config.trade_expiry_ticks.max(1),
    };
    state.next_trade += 1;
    state.trades.insert(trade.trade_id.clone(), trade.clone());
    trim_map(&mut state.trades, MAX_TRADES);
    accepted_trade(request, trade)
}

fn review_trade(
    state: &RepositoryState,
    identity_key: &str,
    request: &TradeRequest,
) -> TradeResponse {
    let Some(trade_id) = request.trade_id.as_deref() else {
        return rejected_trade(request, "Choose a trade to review.");
    };
    let Some(trade) = state.trades.get(trade_id) else {
        return rejected_trade(request, "That trade is no longer in the ledger.");
    };
    let account_id = state
        .identities
        .get(identity_key)
        .expect("identity exists")
        .account_id
        .as_str();
    if trade.creator_account_id != account_id && trade.recipient_account_id != account_id {
        return rejected_trade(request, "That trade is not addressed to you.");
    }
    accepted_trade(request, trade.clone())
}

fn accept_trade(
    state: &mut RepositoryState,
    identity_key: &str,
    request: &TradeRequest,
) -> TradeResponse {
    let Some(trade_id) = request.trade_id.as_deref() else {
        return rejected_trade(request, "Choose a trade to accept.");
    };
    let Some(trade) = state.trades.get(trade_id).cloned() else {
        return rejected_trade(request, "That trade is no longer in the ledger.");
    };
    let recipient_account = state
        .identities
        .get(identity_key)
        .expect("identity exists")
        .account_id
        .clone();
    if trade.recipient_account_id != recipient_account {
        return rejected_trade(request, "Only the named recipient can accept this trade.");
    }
    if trade.status != TradeStatus::Pending {
        return rejected_trade(request, "That trade is no longer pending.");
    }
    let Some(creator_key) = identity_key_for_account(state, &trade.creator_account_id) else {
        return rejected_trade(request, "The offering player is no longer known.");
    };
    if !has_bundle(
        state.identities.get(&creator_key).expect("creator exists"),
        trade.offer,
    ) || !has_bundle(
        state
            .identities
            .get(identity_key)
            .expect("recipient exists"),
        trade.request,
    ) {
        return rejected_trade(request, "One side no longer has the promised goods.");
    }
    apply_bundle(
        state
            .identities
            .get_mut(&creator_key)
            .expect("creator exists"),
        trade.offer,
        -1,
    );
    apply_bundle(
        state
            .identities
            .get_mut(identity_key)
            .expect("recipient exists"),
        trade.request,
        -1,
    );
    apply_bundle(
        state
            .identities
            .get_mut(&creator_key)
            .expect("creator exists"),
        trade.request,
        1,
    );
    apply_bundle(
        state
            .identities
            .get_mut(identity_key)
            .expect("recipient exists"),
        trade.offer,
        1,
    );
    let mut completed = trade;
    completed.status = TradeStatus::Accepted;
    state
        .trades
        .insert(completed.trade_id.clone(), completed.clone());
    accepted_trade(request, completed)
}

fn cancel_trade(
    state: &mut RepositoryState,
    identity_key: &str,
    request: &TradeRequest,
) -> TradeResponse {
    let Some(trade_id) = request.trade_id.as_deref() else {
        return rejected_trade(request, "Choose a trade to cancel.");
    };
    let Some(trade) = state.trades.get(trade_id).cloned() else {
        return rejected_trade(request, "That trade is no longer in the ledger.");
    };
    let account_id = state
        .identities
        .get(identity_key)
        .expect("identity exists")
        .account_id
        .as_str();
    if trade.creator_account_id != account_id && trade.recipient_account_id != account_id {
        return rejected_trade(request, "That trade is not addressed to you.");
    }
    if trade.status != TradeStatus::Pending {
        return rejected_trade(request, "That trade is no longer pending.");
    }
    let mut cancelled = trade;
    cancelled.status = TradeStatus::Cancelled;
    state
        .trades
        .insert(cancelled.trade_id.clone(), cancelled.clone());
    accepted_trade(request, cancelled)
}

fn rejected_trade(request: &TradeRequest, reason: &str) -> TradeResponse {
    TradeResponse {
        request_id: request.request_id.clone(),
        accepted: false,
        trade: None,
        reason: Some(reason.to_owned()),
    }
}

fn accepted_trade(request: &TradeRequest, trade: TradeOffer) -> TradeResponse {
    TradeResponse {
        request_id: request.request_id.clone(),
        accepted: true,
        trade: Some(trade),
        reason: None,
    }
}

fn has_bundle(identity: &Identity, bundle: TradeBundle) -> bool {
    identity.inventory.wheat >= bundle.wheat
        && identity.inventory.turnips >= bundle.turnips
        && identity.inventory.moonberries >= bundle.moonberries
        && identity.inventory.seeds >= bundle.seeds
        && identity.gold >= bundle.gold
}

fn apply_bundle(identity: &mut Identity, bundle: TradeBundle, direction: i32) {
    if direction < 0 {
        identity.inventory.wheat -= bundle.wheat;
        identity.inventory.turnips -= bundle.turnips;
        identity.inventory.moonberries -= bundle.moonberries;
        identity.inventory.seeds -= bundle.seeds;
        identity.gold -= bundle.gold;
    } else {
        identity.inventory.wheat += bundle.wheat;
        identity.inventory.turnips += bundle.turnips;
        identity.inventory.moonberries += bundle.moonberries;
        identity.inventory.seeds += bundle.seeds;
        identity.gold += bundle.gold;
    }
}

fn identity_key_for_account(state: &RepositoryState, account_id: &str) -> Option<String> {
    state
        .identities
        .iter()
        .find(|(_, identity)| identity.account_id == account_id)
        .map(|(key, _)| key.clone())
}

fn trim_map<K: Clone + std::hash::Hash + Eq, V>(map: &mut HashMap<K, V>, max: usize) {
    while map.len() > max {
        if let Some(key) = map.keys().next().cloned() {
            map.remove(&key);
        } else {
            break;
        }
    }
}
