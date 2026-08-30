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
        movement_enabled(ctx),
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
    let frontier_threat_reachable =
        frontier_threat_is_reachable(ctx.player_position, ctx.wilderness);
    let (combat_side_id, combat_side_label) =
        combat_side_control(ctx.combat, frontier_threat_reachable);
    let local_combat_action_ready = combat_control_enabled(
        panels::local_combat_action_enabled(ctx.combat, ctx.server_tick),
        ctx.combat_pending,
    );
    let combat_side_enabled = match combat_side_id {
        "contract" => contract_control_enabled(true, ctx.contract_pending),
        "frontier-retreat" => {
            frontier_combat_control_enabled(frontier_threat_reachable, ctx.frontier_combat_pending)
        }
        "retreat" => local_combat_action_ready,
        _ => false,
    };

    draw_button_row(
        content,
        top + 141.0,
        23.0,
        mouse,
        &[
            (
                "plant",
                "Plant",
                farming_control_enabled(!ctx.knocked_out, ctx.farming_pending),
                ButtonTone::Positive,
            ),
            (
                "tend",
                "Tend",
                farming_control_enabled(!ctx.knocked_out, ctx.farming_pending),
                ButtonTone::Positive,
            ),
            (
                "harvest",
                "Harvest",
                farming_control_enabled(!ctx.knocked_out, ctx.farming_pending),
                ButtonTone::Positive,
            ),
            (
                "animal",
                "Care",
                farming_control_enabled(!ctx.knocked_out, ctx.farming_pending),
                ButtonTone::Positive,
            ),
            (
                "trade",
                if has_pending_trade { "Review" } else { "Trade" },
                trade_control_enabled(true, ctx.trade_pending),
                ButtonTone::Positive,
            ),
            (
                "accept-trade",
                "Accept",
                trade_control_enabled(has_incoming_trade, ctx.trade_pending),
                ButtonTone::Positive,
            ),
            (
                "cancel-trade",
                "Cancel",
                trade_control_enabled(has_pending_trade, ctx.trade_pending),
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
                (
                    "recover-self",
                    "Self",
                    recovery_control_enabled(ctx.knocked_out, ctx.recovery_pending),
                    ButtonTone::Secondary,
                ),
                (
                    "recover",
                    "Rescuer",
                    recovery_control_enabled(ctx.knocked_out, ctx.recovery_pending),
                    ButtonTone::Secondary,
                ),
                (
                    "recover-healer",
                    "Healer",
                    recovery_control_enabled(ctx.knocked_out, ctx.recovery_pending),
                    ButtonTone::Secondary,
                ),
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
                    combat_side_enabled,
                    ButtonTone::Secondary,
                ),
                (
                    "strike",
                    "Strike",
                    local_combat_action_ready,
                    ButtonTone::Secondary,
                ),
                (
                    "technique",
                    "Technique",
                    local_combat_action_ready,
                    ButtonTone::Secondary,
                ),
                (
                    "claim",
                    "Claim",
                    claim_control_enabled(true, ctx.frontier_claim_pending),
                    ButtonTone::Secondary,
                ),
                (
                    "item",
                    "Bandage",
                    local_combat_action_ready,
                    ButtonTone::Secondary,
                ),
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
                local_combat_action_ready,
                ButtonTone::Secondary,
            ),
            (
                "expedition",
                "Pioneer",
                expedition_control_enabled(!ctx.knocked_out, ctx.expedition_pending),
                ButtonTone::Primary,
            ),
            ("chronicle", "Chronicle", true, ButtonTone::Secondary),
            ("say-hello", "Meet", true, ButtonTone::Secondary),
            (
                "school",
                "School",
                skill_control_enabled(!ctx.knocked_out, ctx.skill_pending),
                ButtonTone::Primary,
            ),
            (
                "practice",
                "Practice",
                skill_control_enabled(!ctx.knocked_out, ctx.skill_pending),
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
            (
                "town-hall",
                "Town hall",
                governance_control_enabled(ctx.governance_pending),
                ButtonTone::Primary,
            ),
            (
                "registry",
                "Registry",
                claim_control_enabled(true, ctx.claim_pending),
                ButtonTone::Secondary,
            ),
            (
                "abandon-claim",
                "Abandon",
                claim_control_enabled(ctx.can_abandon_claim, ctx.claim_pending),
                ButtonTone::Secondary,
            ),
            (
                "transfer-claim",
                "Transfer",
                claim_control_enabled(ctx.can_transfer_claim, ctx.claim_pending),
                ButtonTone::Secondary,
            ),
            (
                "order",
                "Order",
                order_control_enabled(ctx.crafting.is_none(), ctx.order_pending),
                ButtonTone::Secondary,
            ),
            (
                "tax-rate",
                "Tax",
                governance_control_enabled(ctx.governance_pending),
                ButtonTone::Secondary,
            ),
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
                knowledge_control_enabled(ctx.knowledge_pending),
                ButtonTone::Secondary,
            ),
            ("households", "Households", true, ButtonTone::Secondary),
            (
                "local-fight",
                "Local fight",
                !ctx.knocked_out
                    && !ctx.combat.is_some_and(|combat| {
                        combat.status == tarrowyn_protocol::LocalCombatStatus::Engaged
                            && combat.action_available_at_tick > ctx.server_tick
                    })
                    && !ctx.combat_pending,
                ButtonTone::Secondary,
            ),
            (
                "guard",
                "Guard",
                local_combat_action_ready,
                ButtonTone::Secondary,
            ),
            (
                "reposition",
                "Reposition",
                local_combat_action_ready,
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
                travel_control_enabled(ctx.can_travel, ctx.knocked_out, ctx.travel_pending),
                ButtonTone::Primary,
            ),
            (
                "recover-travel",
                "Recover",
                travel_control_enabled(ctx.can_recover_travel, ctx.knocked_out, ctx.travel_pending),
                ButtonTone::Primary,
            ),
            (
                "route-repair",
                "Repair",
                route_control_enabled(
                    ctx.regional_region.is_some_and(|region| {
                        region.routes.iter().any(|route| {
                            (route.origin_location_id == region.player_location_id
                                || route.destination_location_id == region.player_location_id)
                                && route.status != RouteStatus::Operational
                        })
                    }),
                    ctx.route_pending,
                ),
                ButtonTone::Positive,
            ),
            (
                "market-region",
                "Market",
                market_control_enabled(ctx.market_pending),
                ButtonTone::Primary,
            ),
            (
                "region-event",
                "Event",
                event_control_enabled(ctx.event_pending),
                ButtonTone::Primary,
            ),
            ("region-details", "Inspect", true, ButtonTone::Secondary),
            (
                "cancel-market",
                "Cancel",
                cancel_market_control_enabled(ctx.has_open_market_order, ctx.market_pending),
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
            (
                "logout",
                "Logout",
                identity_control_enabled(ctx.identity_pending),
                ButtonTone::Secondary,
            ),
            (
                "report",
                "Report",
                report_control_enabled(ctx.report_pending),
                ButtonTone::Secondary,
            ),
            (
                "delete-account",
                if ctx.identity_pending {
                    "Pending"
                } else if ctx.account_deletion_armed {
                    "Tap again"
                } else {
                    "Delete"
                },
                ctx.account_deletion_available && identity_control_enabled(ctx.identity_pending),
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

fn travel_control_enabled(available: bool, knocked_out: bool, travel_pending: bool) -> bool {
    available && !knocked_out && !travel_pending
}

fn recovery_control_enabled(knocked_out: bool, recovery_pending: bool) -> bool {
    knocked_out && !recovery_pending
}

fn market_control_enabled(market_pending: bool) -> bool {
    !market_pending
}

fn trade_control_enabled(available: bool, trade_pending: bool) -> bool {
    available && !trade_pending
}

fn farming_control_enabled(available: bool, farming_pending: bool) -> bool {
    available && !farming_pending
}

fn cancel_market_control_enabled(has_open_market_order: bool, market_pending: bool) -> bool {
    has_open_market_order && !market_pending
}

fn event_control_enabled(event_pending: bool) -> bool {
    !event_pending
}

fn identity_control_enabled(identity_pending: bool) -> bool {
    !identity_pending
}

fn report_control_enabled(report_pending: bool) -> bool {
    !report_pending
}

fn claim_control_enabled(claim_available: bool, claim_pending: bool) -> bool {
    claim_available && !claim_pending
}

fn route_control_enabled(route_available: bool, route_pending: bool) -> bool {
    route_available && !route_pending
}

fn governance_control_enabled(governance_pending: bool) -> bool {
    !governance_pending
}

fn skill_control_enabled(available: bool, skill_pending: bool) -> bool {
    available && !skill_pending
}

fn knowledge_control_enabled(knowledge_pending: bool) -> bool {
    !knowledge_pending
}

fn order_control_enabled(available: bool, order_pending: bool) -> bool {
    available && !order_pending
}

fn combat_control_enabled(available: bool, combat_pending: bool) -> bool {
    available && !combat_pending
}

fn contract_control_enabled(available: bool, contract_pending: bool) -> bool {
    available && !contract_pending
}

fn expedition_control_enabled(available: bool, expedition_pending: bool) -> bool {
    available && !expedition_pending
}

fn frontier_combat_control_enabled(reachable: bool, combat_pending: bool) -> bool {
    reachable && !combat_pending
}

pub(super) fn movement_enabled(ctx: &UiContext<'_>) -> bool {
    !ctx.knocked_out
        && (ctx.offline || !panels::regional_travel_blocks_movement(ctx.regional_region))
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
