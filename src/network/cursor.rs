//! Recovery for event cursors invalidated by a restore or history rebuild.

use super::{NetworkNotice, OnlineClient, WorldProjection};
use tarrowyn_protocol::TavernFeedResponse;

pub(super) fn is_cursor_ahead_error(error: &str) -> bool {
    error.contains("cursor_ahead")
        || (error.contains("GET /v1/events?") && error.contains("status code 409"))
}

pub(super) fn recover_from_restore(client: &mut OnlineClient, notices: &mut Vec<NetworkNotice>) {
    reset_projection_history(&mut client.projection);
    client.pending_state = None;
    client.pending_events = None;
    client.pending_trades = None;
    client.trades.clear();
    client.state_refresh = 0.0;
    client.phase4.recover_regional_cursor();
    client.frontier.clear();
    client.status_message =
        "The shared road was restored; reloading the latest history…".to_owned();
    notices.push(NetworkNotice::Warning(
        "The shared history was restored; the latest settlement state is reloading.".to_owned(),
    ));
}

fn reset_projection_history(projection: &mut WorldProjection) {
    projection.players.clear();
    projection.chat.clear();
    projection.day = 1;
    projection.day_seconds = 0.0;
    projection.server_tick = 0;
    projection.cursor = 0;
    projection.player = None;
    projection.animals.clear();
    projection.feed = TavernFeedResponse {
        notices: Vec::new(),
        rumours: Vec::new(),
        chat: Vec::new(),
        cursor: 0,
    };
    projection.trades.clear();
    projection.wilderness = None;
    projection.chronicle.clear();
    projection.chronicle_summary = None;
    projection.opportunities.clear();
    projection.claim = None;
    projection.outpost = None;
    projection.expedition = None;
}

#[cfg(test)]
mod tests;
