use super::*;
use tarrowyn_protocol::TradeStatus;

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
        let action = request.action;
        let request_id = request.request_id.clone();
        if super::queue::try_push(&mut self.trade_queue, request) {
            self.pending_request_type = Some(format!("trade::{action:?}"));
            self.pending_request_id = Some(request_id);
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
                    self.trades = response.data.trades;
                    self.projection.trades = self.trades.clone();
                }
                Err(error) => self.connection_failed(error, notices),
            }
        }

        let command_result = self
            .pending_trade
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = command_result else { return };
        self.pending_trade = None;
        self.pending_request_id = None;
        self.pending_request_type = None;
        match result {
            Ok(response) => {
                if response.data.accepted {
                    notices.push(NetworkNotice::Success(
                        "The trade ledger accepted the exchange.".to_owned(),
                    ));
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
            Err(error) => self.connection_failed(error, notices),
        }
    }

    pub(super) fn dispatch_trade_requests(&mut self) {
        if self.state != ConnectionState::Online {
            return;
        }
        if self.pending_trades.is_none() {
            self.pending_trades = Some(self.api.get("/v1/trades"));
        }
        if self.pending_trade.is_none() {
            if let Some(request) = self.trade_queue.pop_front() {
                self.pending_trade = Some(self.api.post_json("/v1/trades", &request));
            }
        }
    }
}
