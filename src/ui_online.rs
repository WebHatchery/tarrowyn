use super::*;
use tarrowyn_protocol::{RouteStatus, TradeStatus};

#[path = "ui_online/panels.rs"]
mod panels;
pub(super) use panels::{
    combat_side_control, draw_account, draw_button_row, draw_chronicle, draw_combat_status,
    draw_regional_inspection, draw_skill_selection, frontier_threat_is_reachable,
};

#[cfg(test)]
#[path = "ui_online/tests.rs"]
mod tests;

pub(super) fn draw_sidebar(
    ctx: &UiContext<'_>,
    content: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    let top = content.y + 34.0;
    draw_surface(
        Rect::new(content.x, top, content.w, 40.0),
        &SurfaceStyle::new(Color::new(0.10, 0.14, 0.15, 1.0))
            .with_border(1.0, Color::new(0.32, 0.48, 0.50, 0.45)),
    );
    draw_text_block(
        &format!(
            "{}\n{}  •  {} visible companions",
            ctx.identity_name.unwrap_or("Guest identity"),
            ctx.connection.label(),
            ctx.remote_players
                .iter()
                .filter(|player| ctx.own_account_id != Some(player.account_id.as_str()))
                .count()
        ),
        content.x + 8.0,
        top + 13.0,
        content.w - 16.0,
        25.0,
        11.0,
        2.0,
        CREAM,
    );

    draw_ui_text_ex(
        "Tap a tile or use arrows to walk",
        content.x,
        top + 57.0,
        TextStyle::new(12.0, CREAM).params(),
    );
    draw_move_pad(
        content.x + 238.0,
        top + 47.0,
        mouse,
        actions,
        !ctx.knocked_out,
    );

    draw_combat_status(ctx, content, top);

    let has_pending_trade = ctx.trades.iter().any(|trade| {
        trade.status == TradeStatus::Pending
            && ctx.own_account_id.is_some_and(|account_id| {
                trade.creator_account_id == account_id || trade.recipient_account_id == account_id
            })
    });
    let has_incoming_trade = ctx.trades.iter().any(|trade| {
        trade.status == TradeStatus::Pending
            && ctx
                .own_account_id
                .is_some_and(|account_id| trade.recipient_account_id == account_id)
    });
    let (combat_side_id, combat_side_label) = combat_side_control(
        ctx.combat,
        frontier_threat_is_reachable(ctx.player_position, ctx.wilderness),
    );

    draw_button_row(
        content,
        top + 141.0,
        23.0,
        mouse,
        &[
            ("plant", "Plant", !ctx.knocked_out, ButtonTone::Positive),
            ("tend", "Tend", !ctx.knocked_out, ButtonTone::Positive),
            ("harvest", "Harvest", !ctx.knocked_out, ButtonTone::Positive),
            ("animal", "Care", !ctx.knocked_out, ButtonTone::Positive),
            (
                "trade",
                if has_pending_trade { "Review" } else { "Trade" },
                true,
                ButtonTone::Positive,
            ),
            (
                "accept-trade",
                "Accept",
                has_incoming_trade,
                ButtonTone::Positive,
            ),
            (
                "cancel-trade",
                "Cancel",
                has_pending_trade,
                ButtonTone::Secondary,
            ),
        ],
        ctx,
        actions,
    );

    draw_surface(
        Rect::new(content.x, top + 169.0, content.w, 34.0),
        &SurfaceStyle::new(Color::new(0.075, 0.105, 0.115, 1.0))
            .with_border(1.0, Color::new(0.32, 0.48, 0.50, 0.45)),
    );
    for (index, message) in ctx.chat.iter().rev().take(2).enumerate() {
        draw_ui_text_ex(
            &format!("{}: {}", message.display_name, message.text),
            content.x + 8.0,
            top + 182.0 + index as f32 * 13.0,
            TextStyle::new(10.0, if index == 0 { CREAM } else { dark::TEXT_DIM }).params(),
        );
    }
    if ctx.chat.is_empty() {
        draw_ui_text_ex(
            if ctx.wilderness.is_some() {
                "The frontier channel is quiet."
            } else {
                "The settlement channel is quiet."
            },
            content.x + 8.0,
            top + 191.0,
            TextStyle::new(10.0, dark::TEXT_DIM).params(),
        );
    }

    draw_surface(
        Rect::new(content.x, top + 207.0, content.w - 70.0, 27.0),
        &SurfaceStyle::new(Color::new(0.10, 0.14, 0.15, 1.0))
            .with_border(1.0, Color::new(0.32, 0.48, 0.50, 0.45)),
    );
    draw_ui_text_ex(
        if ctx.chat_draft.is_empty() {
            "Type a message or tap Meet to call the Hearth"
        } else {
            ctx.chat_draft
        },
        content.x + 8.0,
        top + 225.0,
        TextStyle::new(
            10.0,
            if ctx.chat_draft.is_empty() {
                dark::TEXT_DIM
            } else {
                CREAM
            },
        )
        .params(),
    );
    if virtual_button(
        Rect::new(content.right() - 70.0, top + 207.0, 70.0, 27.0),
        "Send",
        !ctx.chat_draft.trim().is_empty() && ctx.connection == ConnectionState::Online,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::SendChat);
    }

    if ctx.knocked_out {
        draw_button_row(
            content,
            top + 239.0,
            22.0,
            mouse,
            &[
                ("recover-self", "Self", true, ButtonTone::Secondary),
                ("recover", "Rescuer", true, ButtonTone::Secondary),
                ("recover-healer", "Healer", true, ButtonTone::Secondary),
            ],
            ctx,
            actions,
        );
    } else {
        draw_button_row(
            content,
            top + 239.0,
            22.0,
            mouse,
            &[
                (
                    combat_side_id,
                    combat_side_label,
                    true,
                    ButtonTone::Secondary,
                ),
                ("strike", "Strike", true, ButtonTone::Secondary),
                ("technique", "Technique", true, ButtonTone::Secondary),
                ("claim", "Claim", true, ButtonTone::Secondary),
                ("item", "Bandage", true, ButtonTone::Secondary),
            ],
            ctx,
            actions,
        );
    }
    draw_button_row(
        content,
        top + 264.0,
        22.0,
        mouse,
        &[
            (
                "spell",
                if ctx.storm_magic_unlocked {
                    "Storm"
                } else {
                    "Spell"
                },
                !ctx.knocked_out,
                ButtonTone::Secondary,
            ),
            (
                "expedition",
                "Pioneer",
                !ctx.knocked_out,
                ButtonTone::Primary,
            ),
            ("chronicle", "Chronicle", true, ButtonTone::Secondary),
            ("say-hello", "Meet", true, ButtonTone::Secondary),
            ("school", "School", !ctx.knocked_out, ButtonTone::Primary),
            (
                "practice",
                "Practice",
                !ctx.knocked_out,
                ButtonTone::Positive,
            ),
        ],
        ctx,
        actions,
    );
    draw_button_row(
        content,
        top + 289.0,
        22.0,
        mouse,
        &[
            ("town-hall", "Town hall", true, ButtonTone::Primary),
            ("registry", "Registry", true, ButtonTone::Secondary),
            (
                "abandon-claim",
                "Abandon",
                ctx.can_abandon_claim,
                ButtonTone::Secondary,
            ),
            (
                "transfer-claim",
                "Transfer",
                ctx.can_transfer_claim,
                ButtonTone::Secondary,
            ),
            ("order", "Order", true, ButtonTone::Secondary),
            ("tax-rate", "Tax", true, ButtonTone::Secondary),
        ],
        ctx,
        actions,
    );
    draw_button_row(
        content,
        top + 314.0,
        22.0,
        mouse,
        &[
            (
                "knowledge",
                ctx.knowledge_label,
                true,
                ButtonTone::Secondary,
            ),
            ("households", "Households", true, ButtonTone::Secondary),
            (
                "local-fight",
                "Local fight",
                !ctx.knocked_out,
                ButtonTone::Secondary,
            ),
            ("guard", "Guard", !ctx.knocked_out, ButtonTone::Secondary),
            (
                "reposition",
                "Reposition",
                !ctx.knocked_out,
                ButtonTone::Secondary,
            ),
        ],
        ctx,
        actions,
    );
    draw_button_row(
        content,
        top + 339.0,
        22.0,
        mouse,
        &[
            (
                "travel",
                ctx.travel_label,
                travel_control_enabled(ctx.can_travel, ctx.knocked_out),
                ButtonTone::Primary,
            ),
            (
                "recover-travel",
                "Recover",
                travel_control_enabled(ctx.can_recover_travel, ctx.knocked_out),
                ButtonTone::Primary,
            ),
            (
                "route-repair",
                "Repair",
                ctx.regional_region.is_some_and(|region| {
                    region.routes.iter().any(|route| {
                        (route.origin_location_id == region.player_location_id
                            || route.destination_location_id == region.player_location_id)
                            && route.status != RouteStatus::Operational
                    })
                }),
                ButtonTone::Positive,
            ),
            ("market-region", "Market", true, ButtonTone::Primary),
            ("region-event", "Event", true, ButtonTone::Primary),
            ("region-details", "Inspect", true, ButtonTone::Secondary),
            (
                "cancel-market",
                "Cancel",
                ctx.has_open_market_order,
                ButtonTone::Secondary,
            ),
        ],
        ctx,
        actions,
    );
    draw_button_row(
        content,
        top + 364.0,
        22.0,
        mouse,
        &[
            (
                if ctx.account_link_available {
                    "account"
                } else {
                    "account-details"
                },
                "Account",
                true,
                ButtonTone::Secondary,
            ),
            ("logout", "Logout", true, ButtonTone::Secondary),
            ("report", "Report", true, ButtonTone::Secondary),
            (
                "delete-account",
                if ctx.account_deletion_armed {
                    "Tap again"
                } else {
                    "Delete"
                },
                ctx.account_deletion_available,
                ButtonTone::Secondary,
            ),
        ],
        ctx,
        actions,
    );
    draw_button_row(
        content,
        top + 389.0,
        22.0,
        mouse,
        &[
            ("reconnect", "Reconnect", true, ButtonTone::Primary),
            ("offline", "Offline fixture", true, ButtonTone::Secondary),
        ],
        ctx,
        actions,
    );
    let chronicle_line = ctx
        .expedition
        .map(|expedition| pioneer_status_line(expedition, ctx.expedition_requirements))
        .or_else(|| {
            ctx.chronicle_summary.map(|summary| {
                format!(
                    "{} older chronicle entries remain searchable; {}",
                    summary.entry_count,
                    summary
                        .highlights
                        .last()
                        .map(String::as_str)
                        .unwrap_or("the archive is ready")
                )
            })
        })
        .or_else(|| {
            ctx.chronicle
                .last()
                .map(|entry| entry.title.clone())
                .or_else(|| {
                    ctx.opportunities
                        .first()
                        .map(|opportunity| opportunity.clue.clone())
                })
        })
        .unwrap_or_else(|| "The frontier registry is listening.".to_owned());
    let phase_line = format!(
        "{} • {}",
        ctx.phase4_summary
            .lines()
            .next()
            .unwrap_or("Town hall loading"),
        ctx.phase5_summary
            .lines()
            .next()
            .unwrap_or("Region loading")
    );
    draw_ui_text_ex(
        &phase_line,
        content.x,
        top + 416.0,
        TextStyle::new(9.0, dark::TEXT_DIM).params(),
    );
    let settlement_line = ctx
        .phase5_summary
        .lines()
        .nth(1)
        .unwrap_or("Settlement comparison loading");
    draw_ui_text_ex(
        settlement_line,
        content.x,
        top + 426.0,
        TextStyle::new(8.5, dark::TEXT_DIM).params(),
    );
    let regional_economy_line = ctx
        .phase5_summary
        .lines()
        .nth(2)
        .unwrap_or("Road and market telemetry loading");
    draw_ui_text_ex(
        regional_economy_line,
        content.x,
        top + 440.0,
        TextStyle::new(8.25, dark::TEXT_DIM).params(),
    );
    draw_ui_text_ex(
        &chronicle_line,
        content.x,
        top + 454.0,
        TextStyle::new(8.25, dark::TEXT_DIM).params(),
    );
}

fn travel_control_enabled(available: bool, knocked_out: bool) -> bool {
    available && !knocked_out
}

fn pioneer_status_line(
    expedition: &tarrowyn_protocol::Expedition,
    requirements: tarrowyn_protocol::ExpeditionRequirements,
) -> String {
    let status = match expedition.status {
        tarrowyn_protocol::ExpeditionStatus::Planning => "planning",
        tarrowyn_protocol::ExpeditionStatus::Launched => "on the road",
        tarrowyn_protocol::ExpeditionStatus::Succeeded => "founded",
        tarrowyn_protocol::ExpeditionStatus::Retreated => "retreated",
    };
    format!(
        "Pioneer {status} • {} companions • F{}/{} T{}/{} M{}/{} S{}/{} • {}",
        expedition.members.len(),
        expedition.food,
        requirements.food,
        expedition.tools,
        requirements.tools,
        expedition.materials,
        requirements.materials,
        expedition.safety,
        requirements.safety,
        expedition.outpost_name,
    )
}
