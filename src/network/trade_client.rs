use super::*;

impl OnlineClient {
    pub fn queue_trade(&mut self, mut request: TradeRequest) {
        if self.state != ConnectionState::Online {
            return;
        }
        if request.request_id.trim().is_empty() {
            request.request_id = self.next_request_id("trade");
        }
        self.pending_request_type = Some(format!("trade::{:?}", request.action));
        self.pending_request_id = Some(request.request_id.clone());
        self.status_message = "Trade command sent; waiting for the ledger…".to_owned();
        self.trade_queue.push_back(request);
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
