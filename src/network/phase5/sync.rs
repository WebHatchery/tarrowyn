//! Regional projection refresh and command dispatch.

use super::*;

impl Phase5Client {
    #[cfg(test)]
    pub(super) fn dispatch(&mut self, api: &mut HttpClient, another_mutation_pending: bool) {
        self.dispatch_with_mode(api, another_mutation_pending, false);
    }

    pub(super) fn dispatch_with_mode(
        &mut self,
        api: &mut HttpClient,
        another_mutation_pending: bool,
        session_only: bool,
    ) {
        if self.logged_out {
            return;
        }
        self.dispatch_refresh(api, another_mutation_pending);
        if self.dispatch_blocked() {
            return;
        }
        if !session_only && self.refresh_timer <= 0.0 {
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
                    .unwrap_or(self.projection_cursor);
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
            let retry_command = (!session_only
                || self
                    .in_flight_command
                    .as_ref()
                    .is_some_and(is_session_command))
            .then(|| self.in_flight_command.take())
            .flatten();
            let command = retry_command.or_else(|| {
                let command_index = if session_only {
                    self.commands.iter().position(is_session_command)
                } else if self.commands.is_empty() {
                    None
                } else {
                    Some(0)
                };
                command_index.map(|command_index| {
                    self.commands
                        .remove(command_index)
                        .expect("queued command index exists")
                })
            });
            if let Some(command) = command {
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

pub(super) fn is_session_command(command: &Phase5Command) -> bool {
    matches!(
        command,
        Phase5Command::Link(_)
            | Phase5Command::Revoke(_)
            | Phase5Command::Report(_)
            | Phase5Command::Delete(_)
    )
}
