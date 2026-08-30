use super::*;
use tarrowyn_protocol::{TradeAction, TradeBundle, TradeOffer, TradeStatus};

pub(super) fn trade_success_message(
    action: Option<TradeAction>,
    trade: Option<&TradeOffer>,
) -> String {
    let Some(trade) = trade else {
        return trade_action_success_message(action).to_owned();
    };
    match action {
        Some(TradeAction::Create) => format!(
            "Offer sent to {}; give {}, receive {}.",
            trade.recipient_name,
            trade_bundle_summary(trade.offer),
            trade_bundle_summary(trade.request)
        ),
        Some(TradeAction::Review) => format!(
            "Trade with {} remains pending: give {}, receive {}.",
            trade.recipient_name,
            trade_bundle_summary(trade.offer),
            trade_bundle_summary(trade.request)
        ),
        Some(TradeAction::Accept) => format!(
            "Trade completed with {}; exchanged {} for {}.",
            trade.creator_name,
            trade_bundle_summary(trade.request),
            trade_bundle_summary(trade.offer)
        ),
        Some(TradeAction::Cancel) => {
            format!("Trade with {} withdrawn.", trade.recipient_name)
        }
        None => format!(
            "Trade recorded with {}; give {}, receive {}.",
            trade.recipient_name,
            trade_bundle_summary(trade.offer),
            trade_bundle_summary(trade.request)
        ),
    }
}

fn trade_action_success_message(action: Option<TradeAction>) -> &'static str {
    match action {
        Some(TradeAction::Create) => "The trade offer is on the ledger; awaiting the other player.",
        Some(TradeAction::Review) => "The trade details are current.",
        Some(TradeAction::Accept) => "The trade ledger completed the exchange.",
        Some(TradeAction::Cancel) => "The trade offer was withdrawn.",
        None => "The trade ledger accepted the exchange.",
    }
}

fn trade_bundle_summary(bundle: TradeBundle) -> String {
    let mut parts = Vec::new();
    if bundle.wheat > 0 {
        parts.push(format!("{} wheat", bundle.wheat));
    }
    if bundle.turnips > 0 {
        let unit = if bundle.turnips == 1 {
            "turnip"
        } else {
            "turnips"
        };
        parts.push(format!("{} {unit}", bundle.turnips));
    }
    if bundle.moonberries > 0 {
        let unit = if bundle.moonberries == 1 {
            "moonberry"
        } else {
            "moonberries"
        };
        parts.push(format!("{} {unit}", bundle.moonberries));
    }
    if bundle.seeds > 0 {
        let unit = if bundle.seeds == 1 { "seed" } else { "seeds" };
        parts.push(format!("{} {unit}", bundle.seeds));
    }
    if bundle.gold > 0 {
        parts.push(format!("{} gold", bundle.gold));
    }
    if parts.is_empty() {
        "nothing".to_owned()
    } else {
        parts.join(", ")
    }
}

impl OnlineClient {
    pub(crate) fn pending_trade_for(&self, account_id: &str) -> Option<&TradeOffer> {
        self.projection.trades.iter().find(|trade| {
            trade.status == TradeStatus::Pending
                && (trade.creator_account_id == account_id
                    || trade.recipient_account_id == account_id)
        })
    }

    pub(crate) fn incoming_trade_for(&self, account_id: &str) -> Option<&TradeOffer> {
        self.projection.trades.iter().find(|trade| {
            trade.status == TradeStatus::Pending && trade.recipient_account_id == account_id
        })
    }

    pub fn queue_trade(&mut self, mut request: TradeRequest) {
        if self.state != ConnectionState::Online {
            return;
        }
        if request.request_id.trim().is_empty() {
            request.request_id = self.next_request_id("trade");
        }
        if super::queue::try_push(&mut self.trade_queue, request) {
            self.status_message = "Trade command sent; waiting for the ledger…".to_owned();
        } else {
            self.status_message =
                "The trade ledger is busy; wait for current actions before trying again."
                    .to_owned();
        }
    }

    pub(super) fn poll_trade_requests(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let list_result = self
            .pending_trades
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        if let Some(result) = list_result {
            self.pending_trades = None;
            match result {
                Ok(response) => {
                    let cursor = response.meta.cursor.unwrap_or(response.data.cursor);
                    if self
                        .projection
                        .response_is_current(response.meta.server_tick, cursor)
                    {
                        self.projection.record_response_version(
                            response.meta.server_tick,
                            response.meta.cursor,
                        );
                        self.trades = response.data.trades;
                        self.projection.trades = self.trades.clone();
                    }
                }
                Err(error) => self.connection_failed(error, notices),
            }
        }

        let Some(mut pending) = self.pending_trade.take() else {
            return;
        };
        pending.retry_timer = (pending.retry_timer - dt.max(0.0)).max(0.0);
        if pending.retry_timer > 0.0 {
            self.pending_trade = Some(pending);
            return;
        }
        if pending.pending.is_none() {
            let request = pending.request.clone();
            pending.pending = Some(self.api.post_json("/v1/trades", &request));
        }
        let Some(result) = pending
            .pending
            .as_mut()
            .and_then(|request| request.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        else {
            self.pending_trade = Some(pending);
            return;
        };
        pending.pending = None;
        let trade_action = self.pending_trade_action;
        match result {
            Ok(response) => {
                self.projection
                    .record_response_version(response.meta.server_tick, response.meta.cursor);
                self.pending_trade_action = None;
                self.pending_request_id = None;
                self.pending_request_type = None;
                if response.data.accepted {
                    notices.push(NetworkNotice::Success(trade_success_message(
                        trade_action,
                        response.data.trade.as_ref(),
                    )));
                } else {
                    notices.push(NetworkNotice::Warning(
                        response
                            .data
                            .reason
                            .unwrap_or_else(|| "The trade was rejected.".to_owned()),
                    ));
                }
                self.pending_trades = Some(self.api.get("/v1/trades"));
            }
            Err(error)
                if is_transient_transport_error(&error)
                    && pending.retries < super::commands::MAX_COMMAND_RETRIES =>
            {
                let retries = pending.retries + 1;
                pending.retries = retries;
                pending.retry_timer = super::commands::COMMAND_RETRY_DELAY_SECONDS;
                self.pending_trade = Some(pending);
                notices.push(NetworkNotice::Warning(format!(
                    "The trade command could not be confirmed; retrying the same request ({retries}/{}).",
                    super::commands::MAX_COMMAND_RETRIES
                )));
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }

    pub(super) fn dispatch_trade_requests(&mut self) {
        if self.state != ConnectionState::Online {
            return;
        }
        if self.phase4.auth_refresh_pending() {
            return;
        }
        if self.pending_trades.is_none() {
            self.pending_trades = Some(self.api.get("/v1/trades"));
        }
        if self.pending_trade.is_none() {
            if let Some(request) = self.trade_queue.pop_front() {
                self.pending_trade_action = Some(request.action);
                self.pending_request_type = Some(format!("trade::{:?}", request.action));
                self.pending_request_id = Some(request.request_id.clone());
                self.pending_trade = Some(PendingTrade {
                    pending: Some(self.api.post_json("/v1/trades", &request)),
                    request,
                    retries: 0,
                    retry_timer: 0.0,
                });
            }
        }
    }
}
