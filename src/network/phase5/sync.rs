//! Regional projection refresh and command dispatch.

use super::*;

impl Phase5Client {
    pub(super) fn dispatch(&mut self, api: &mut HttpClient, another_mutation_pending: bool) {
        if self.logged_out {
            return;
        }
        self.dispatch_refresh(api, another_mutation_pending);
        if self.dispatch_blocked() {
            return;
        }
        if self.refresh_timer <= 0.0 {
            if self.pending_region.is_none() {
                self.pending_region = Some(api.get("/v1/region"));
            }
            if self.pending_settlements.is_none() {
                self.pending_settlements = Some(api.get("/v1/settlements"));
            }
            if self.pending_households.is_none() {
                self.pending_households = Some(api.get("/v1/households/region"));
            }
            if self.pending_market.is_none() {
                self.pending_market = Some(api.get("/v1/market/orders"));
            }
            if self.pending_events.is_none() {
                let cursor = self
                    .events
                    .as_ref()
                    .map(|events| events.cursor)
                    .unwrap_or(0);
                self.pending_events = Some(api.get(&format!("/v1/events/region?since={cursor}")));
            }
            if self.pending_law.is_none() {
                self.pending_law = Some(api.get("/v1/law"));
            }
            if self.pending_account.is_none() {
                self.pending_account = Some(api.get("/v1/account"));
            }
            self.refresh_timer = 1.5;
        }
        if self.pending_command.is_none()
            && self.pending_refresh.is_none()
            && !another_mutation_pending
            && self.command_retry_timer <= 0.0
        {
            if let Some(command) = self.commands.pop_front() {
                self.pending_market_action = match &command {
                    Phase5Command::Market(request) => Some(request.action),
                    _ => None,
                };
                self.pending_command = Some(match &command {
                    Phase5Command::Travel(request) => api.post_json("/v1/travel", request),
                    Phase5Command::Route(request) => api.post_json("/v1/routes", request),
                    Phase5Command::Market(request) => api.post_json("/v1/market/orders", request),
                    Phase5Command::Event(request) => api.post_json("/v1/events/region", request),
                    Phase5Command::Link(request) => api.post_json("/v1/auth/link", request),
                    Phase5Command::Revoke(request) => api.post_json("/v1/auth/revoke", request),
                    Phase5Command::Report(request) => {
                        api.post_json("/v1/moderation/report", request)
                    }
                    Phase5Command::Delete(request) => api.post_json("/v1/account/delete", request),
                });
                self.in_flight_command = Some(command);
            }
        }
    }
}
