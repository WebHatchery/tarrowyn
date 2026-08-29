use super::*;
use tarrowyn_protocol::{RouteStatus, TradeStatus};

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
    draw_move_pad(content.x + 238.0, top + 47.0, mouse, actions);

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
    let (combat_side_id, combat_side_label) = local_combat_side_control(ctx.combat);

    draw_button_row(
        content,
        top + 141.0,
        23.0,
        mouse,
        &[
            ("plant", "Plant", true, ButtonTone::Positive),
            ("tend", "Tend", true, ButtonTone::Positive),
            ("harvest", "Harvest", true, ButtonTone::Positive),
            ("animal", "Care", true, ButtonTone::Positive),
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
            "Type a message or tap a quick phrase"
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
            ("say-hello", "Hello", true, ButtonTone::Secondary),
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
                ctx.can_travel,
                ButtonTone::Primary,
            ),
            (
                "recover-travel",
                "Recover",
                ctx.can_recover_travel,
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
            ("account", "Account", true, ButtonTone::Secondary),
            ("logout", "Logout", true, ButtonTone::Secondary),
            ("report", "Report", true, ButtonTone::Secondary),
            (
                "delete-account",
                if ctx.account_deletion_armed {
                    "Tap again"
                } else {
                    "Delete"
                },
                true,
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
        .chronicle_summary
        .map(|summary| {
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

fn local_combat_side_control(
    combat: Option<&tarrowyn_protocol::LocalCombatState>,
) -> (&'static str, &'static str) {
    if combat.is_some_and(|combat| combat.status == tarrowyn_protocol::LocalCombatStatus::Engaged) {
        ("retreat", "Retreat")
    } else {
        ("contract", "Contract")
    }
}

pub(super) fn draw_regional_inspection(
    ctx: &UiContext<'_>,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    let Some(details) = ctx.regional_inspection else {
        return;
    };
    let panel = Rect::new(42.0, 126.0, 780.0, 438.0);
    draw_surface_with_title(
        panel,
        Some("Regional ledger inspection"),
        &SurfaceStyle::new(Color::new(0.055, 0.075, 0.09, 0.99))
            .with_border(1.0, Color::new(0.50, 0.72, 0.62, 0.9))
            .with_header(42.0, Color::new(0.09, 0.14, 0.15, 1.0))
            .with_header_divider(1.0, Color::new(0.32, 0.48, 0.50, 0.75)),
        TextStyle::new(17.0, CREAM),
    );
    let detail_height = if ctx.regional_event_choices.is_empty() {
        318.0
    } else {
        238.0
    };
    draw_text_block(
        details,
        panel.x + 20.0,
        panel.y + 70.0,
        panel.w - 40.0,
        detail_height,
        14.0,
        3.0,
        CREAM,
    );
    if !ctx.regional_event_choices.is_empty() {
        draw_event_choices(panel, ctx.regional_event_choices, mouse, actions);
    }
    draw_button_row(
        Rect::new(panel.x + 20.0, panel.bottom() - 82.0, panel.w - 40.0, 28.0),
        panel.bottom() - 82.0,
        28.0,
        mouse,
        &[
            ("route-escort", "Escort road", true, ButtonTone::Positive),
            ("route-improve", "Improve road", true, ButtonTone::Primary),
        ],
        ctx,
        actions,
    );
    if virtual_button(
        Rect::new(panel.right() - 126.0, panel.bottom() - 42.0, 106.0, 28.0),
        "Close",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("region-details".to_owned()));
    }
}

fn draw_event_choices(panel: Rect, choices: &[String], mouse: Vec2, actions: &mut Vec<UiAction>) {
    let gap = 4.0;
    let visible = choices.iter().take(3).collect::<Vec<_>>();
    let width = (panel.w - 40.0 - gap * (visible.len().saturating_sub(1) as f32))
        / visible.len().max(1) as f32;
    for (index, choice) in visible.iter().enumerate() {
        if virtual_button(
            Rect::new(
                panel.x + 20.0 + index as f32 * (width + gap),
                panel.bottom() - 118.0,
                width,
                28.0,
            ),
            choice,
            true,
            ButtonTone::Primary,
            mouse,
        ) {
            actions.push(UiAction::RegionalEvent((*choice).clone()));
        }
    }
}

fn draw_combat_status(ctx: &UiContext<'_>, content: Rect, top: f32) {
    let Some(combat) = ctx.combat else {
        return;
    };
    let available_in = combat
        .action_available_at_tick
        .saturating_sub(ctx.server_tick);
    let timing = if available_in == 0 {
        "Action ready".to_owned()
    } else {
        format!(
            "Action opens in {available_in} beat{}",
            if available_in == 1 { "" } else { "s" }
        )
    };
    let status = match combat.status {
        tarrowyn_protocol::LocalCombatStatus::Ready => "ready",
        tarrowyn_protocol::LocalCombatStatus::Engaged => "engaged",
        tarrowyn_protocol::LocalCombatStatus::Victorious => "victorious",
        tarrowyn_protocol::LocalCombatStatus::KnockedOut => "knocked out",
        tarrowyn_protocol::LocalCombatStatus::Retreated => "retreated",
    };
    draw_surface(
        Rect::new(
            content.x,
            top + 101.0,
            content.w,
            if combat.status == tarrowyn_protocol::LocalCombatStatus::KnockedOut {
                38.0
            } else {
                34.0
            },
        ),
        &SurfaceStyle::new(Color::new(0.075, 0.105, 0.115, 1.0))
            .with_border(1.0, Color::new(0.62, 0.42, 0.22, 0.7)),
    );
    draw_ui_text_ex(
        &format!(
            "Encounter {status}  •  enemy {}  •  you {}  •  {timing}",
            combat.enemy_health, combat.player_health
        ),
        content.x + 8.0,
        top + 115.0,
        TextStyle::new(10.0, GOLD).params(),
    );
    if combat.status == tarrowyn_protocol::LocalCombatStatus::KnockedOut {
        draw_ui_text_ex(
            &format!(
                "Risk: {}  •  Healer: {} gold  •  stored property safe",
                recovery_risk_label(&combat.carried_risk),
                combat.recovery_cost,
            ),
            content.x + 8.0,
            top + 129.0,
            TextStyle::new(8.0, CREAM).params(),
        );
    }
}

fn recovery_risk_label(carried_risk: &str) -> &'static str {
    if carried_risk.to_ascii_lowercase().contains("seed") {
        "1 carried seed"
    } else {
        "carried item"
    }
}

fn draw_button_row(
    content: Rect,
    y: f32,
    height: f32,
    mouse: Vec2,
    entries: &[(&str, &str, bool, ButtonTone)],
    ctx: &UiContext<'_>,
    actions: &mut Vec<UiAction>,
) {
    let gap = 4.0;
    let width =
        (content.w - gap * (entries.len().saturating_sub(1) as f32)) / entries.len().max(1) as f32;
    for (index, (id, label, active, tone)) in entries.iter().enumerate() {
        let enabled = match *id {
            "reconnect" => ctx.connection != ConnectionState::Online,
            "offline" => true,
            _ => *active && ctx.connection == ConnectionState::Online,
        };
        if virtual_button(
            Rect::new(content.x + index as f32 * (width + gap), y, width, height),
            label,
            enabled,
            *tone,
            mouse,
        ) {
            if *id == "reconnect" {
                actions.push(UiAction::Reconnect);
            } else if *id == "offline" {
                actions.push(UiAction::UseOffline);
            } else if *id == "say-hello" {
                actions.push(UiAction::QuickChat("Meet at the Hearth".to_owned()));
            } else {
                actions.push(UiAction::Interact((*id).to_owned()));
            }
        }
    }
}
