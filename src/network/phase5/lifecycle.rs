//! Clear regional projections when identity or retained history changes.

use super::*;

impl Phase5Client {
    pub(crate) fn clear(&mut self) {
        self.pending_region = None;
        self.pending_settlements = None;
        self.pending_households = None;
        self.pending_market = None;
        self.pending_events = None;
        self.pending_law = None;
        self.pending_account = None;
        self.pending_refresh = None;
        self.in_flight_refresh = None;
        self.pending_command = None;
        self.pending_market_action = None;
        self.in_flight_command = None;
        self.commands.clear();
        self.clear_cached_projections();
        self.account = None;
        self.linked_account = None;
        self.refreshed_session = None;
        self.refresh_token = None;
        self.logged_out = false;
        self.deletion_armed = false;
        self.refresh_timer = 0.0;
        self.auth_refresh_timer = f32::MAX;
        self.refresh_retry_timer = 0.0;
        self.refresh_retry_count = 0;
        self.command_retry_timer = 0.0;
        self.command_retry_count = 0;
        self.projection_cursor = 0;
    }

    pub(crate) fn clear_for_reconnect(&mut self) {
        let refresh_token = self.refresh_token.clone();
        self.clear();
        self.refresh_token = refresh_token;
        self.refresh_timer = f32::MAX;
        self.auth_refresh_timer = 0.0;
    }

    pub(super) fn clear_cached_projections(&mut self) {
        self.region = None;
        self.settlements = None;
        self.households = None;
        self.market = None;
        self.events = None;
        self.law = None;
    }

    pub(crate) fn reset_identity_projections(&mut self) {
        self.pending_region = None;
        self.pending_settlements = None;
        self.pending_households = None;
        self.pending_market = None;
        self.pending_events = None;
        self.pending_law = None;
        self.pending_account = None;
        self.pending_market_action = None;
        self.commands.clear();
        self.command_retry_timer = 0.0;
        self.command_retry_count = 0;
        self.clear_cached_projections();
        self.refresh_timer = 0.0;
    }

    pub(crate) fn reset_event_cursor(&mut self) {
        self.pending_region = None;
        self.pending_settlements = None;
        self.pending_households = None;
        self.pending_market = None;
        self.pending_events = None;
        self.pending_law = None;
        self.pending_account = None;
        self.pending_command = None;
        self.pending_market_action = None;
        self.in_flight_command = None;
        self.commands.clear();
        self.clear_cached_projections();
        self.account = None;
        self.refresh_timer = 0.0;
        self.command_retry_timer = 0.0;
        self.command_retry_count = 0;
        self.projection_cursor = 0;
    }

    pub(super) fn reset_regional_event_cursor(&mut self, current_cursor: u64) {
        self.events = Some(tarrowyn_protocol::RegionalEventsResponse {
            events: Vec::new(),
            cursor: current_cursor,
        });
        self.projection_cursor = self.projection_cursor.max(current_cursor);
        self.refresh_timer = 0.0;
    }
}
