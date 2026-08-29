//! Production-session refresh and transport recovery for the regional client.

use super::*;

impl Phase5Client {
    pub(super) fn dispatch_refresh(&mut self, api: &mut HttpClient) {
        if self.pending_refresh.is_some()
            || self.auth_refresh_timer > 0.0
            || self.refresh_retry_timer > 0.0
            || self.pending_command.is_some()
        {
            return;
        }
        let request = self.in_flight_refresh.clone().or_else(|| {
            self.refresh_token
                .clone()
                .map(|refresh_token| tarrowyn_protocol::AuthRefreshRequest {
                    request_id: self.next_id(),
                    refresh_token,
                })
        });
        if let Some(request) = request {
            self.pending_refresh = Some(api.post_json("/v1/auth/refresh", &request));
            self.in_flight_refresh = Some(request);
            self.auth_refresh_timer = f32::MAX;
        }
    }

    pub(super) fn poll_refresh(
        &mut self,
        dt: f32,
        api: &mut HttpClient,
        notices: &mut Vec<NetworkNotice>,
    ) {
        let result = self
            .pending_refresh
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = result else { return };
        self.pending_refresh = None;
        let in_flight_refresh = self.in_flight_refresh.take();
        match result {
            Ok(response) => self.apply_refresh(response.data.session, api, notices),
            Err(error)
                if is_transient_transport_error(&error)
                    && self.refresh_retry_count < MAX_REFRESH_RETRIES
                    && in_flight_refresh.is_some() =>
            {
                self.in_flight_refresh = in_flight_refresh;
                self.refresh_retry_count += 1;
                self.auth_refresh_timer = 0.0;
                self.refresh_retry_timer = REFRESH_RETRY_DELAY_SECONDS;
                notices.push(NetworkNotice::Warning(format!(
                    "The production session refresh could not be confirmed; retrying the same request ({}/{}). {}",
                    self.refresh_retry_count,
                    MAX_REFRESH_RETRIES,
                    short_error(&error)
                )));
            }
            Err(error) => self.fail_refresh(error, api, notices),
        }
    }

    fn apply_refresh(
        &mut self,
        session: tarrowyn_protocol::AuthSession,
        api: &mut HttpClient,
        notices: &mut Vec<NetworkNotice>,
    ) {
        api.set_bearer_token(Some(&session.account_token));
        self.refresh_token = Some(session.refresh_token.clone());
        self.auth_refresh_timer = refresh_delay(session.expires_in_seconds);
        self.refresh_retry_timer = 0.0;
        self.refresh_retry_count = 0;
        self.refreshed_session = Some(session);
        notices.push(NetworkNotice::Success(
            "The production session was refreshed safely.".to_owned(),
        ));
    }

    fn fail_refresh(
        &mut self,
        error: String,
        api: &mut HttpClient,
        notices: &mut Vec<NetworkNotice>,
    ) {
        self.in_flight_refresh = None;
        self.refresh_retry_timer = 0.0;
        self.refresh_retry_count = 0;
        self.pending_region = None;
        self.pending_settlements = None;
        self.pending_households = None;
        self.pending_market = None;
        self.pending_events = None;
        self.pending_law = None;
        self.pending_account = None;
        self.pending_command = None;
        self.in_flight_command = None;
        self.pending_market_action = None;
        self.command_retry_timer = 0.0;
        self.command_retry_count = 0;
        self.commands.clear();
        self.account = None;
        self.refresh_token = None;
        self.auth_refresh_timer = f32::MAX;
        self.refresh_timer = f32::MAX;
        self.clear_cached_projections();
        api.set_bearer_token(None);
        self.deletion_armed = false;
        self.logged_out = true;
        notices.push(NetworkNotice::Warning(format!(
            "The production session could not be refreshed: {}; provider sign-in is required.",
            short_error(&error)
        )));
    }
}
