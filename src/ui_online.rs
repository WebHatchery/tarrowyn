use super::*;
#[path = "ui_online/controls.rs"]
mod controls;
#[path = "ui_online/panels.rs"]
mod panels;
pub(super) use controls::{
    cancel_market_control_enabled, claim_control_enabled, combat_control_enabled,
    contract_control_enabled, event_control_enabled, expedition_control_enabled,
    farming_control_enabled, frontier_combat_control_enabled, governance_control_enabled,
    identity_control_enabled, knowledge_control_enabled, market_control_enabled, movement_enabled,
    movement_tooltip, order_control_enabled, recovery_control_enabled, report_control_enabled,
    route_control_enabled, skill_control_enabled, trade_control_enabled, travel_control_enabled,
    visible_companion_count, visible_player_count,
};
#[cfg(test)]
pub(crate) use controls::{
    movement_tooltip_for, pioneer_status_line, walking_connection_enabled,
    walking_projection_enabled,
};
#[cfg(test)]
pub(crate) use panels::sidebar_modal_control_enabled;
pub(super) use panels::{
    combat_side_control, draw_account, draw_button_row, draw_chronicle, draw_regional_inspection,
    draw_school_selection, draw_skill_selection, frontier_threat_is_reachable,
    local_combat_action_enabled,
};

pub(super) fn gameplay_modal_open(ctx: &UiContext<'_>) -> bool {
    panels::sidebar_modal_open(ctx)
}

pub(super) fn movement_tooltip_for_overlay(
    modal_open: bool,
    movement_tooltip: &'static str,
) -> &'static str {
    if modal_open {
        "Close the open panel to use road controls"
    } else {
        movement_tooltip
    }
}

#[cfg(test)]
#[path = "ui_online/tests.rs"]
mod tests;

#[cfg(test)]
pub(super) fn tavern_feed_line(
    notices: &[tarrowyn_protocol::TavernNotice],
    rumours: &[String],
) -> Option<String> {
    notices
        .iter()
        .rev()
        .find(|notice| notice.kind != "settlement" && !notice.text.trim().is_empty())
        .map(|notice| format!("Tavern notice: {}", compact_feed_text(&notice.text)))
        .or_else(|| {
            rumours
                .iter()
                .find(|rumour| !rumour.trim().is_empty())
                .map(|rumour| format!("Tavern rumour: {}", compact_feed_text(rumour)))
        })
        .or_else(|| {
            notices
                .iter()
                .rev()
                .find(|notice| !notice.text.trim().is_empty())
                .map(|notice| format!("Tavern notice: {}", compact_feed_text(&notice.text)))
        })
}

#[cfg(test)]
fn compact_feed_text(text: &str) -> String {
    let mut chars = text.chars();
    let compact: String = chars.by_ref().take(60).collect();
    if chars.next().is_some() {
        format!("{compact}…")
    } else {
        compact
    }
}

#[cfg(test)]
pub(super) fn quiet_chat_label() -> &'static str {
    "The settlement channel is quiet."
}
