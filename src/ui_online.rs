use super::*;
use tarrowyn_protocol::RouteStatus;

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
            ("trade", "Trade", true, ButtonTone::Positive),
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

    draw_button_row(
        content,
        top + 239.0,
        22.0,
        mouse,
        &[
            (
                "contract",
                "Contract",
                !ctx.knocked_out,
                ButtonTone::Secondary,
            ),
            ("strike", "Strike", !ctx.knocked_out, ButtonTone::Secondary),
            (
                "technique",
                "Technique",
                !ctx.knocked_out,
                ButtonTone::Secondary,
            ),
            ("recover", "Recover", ctx.knocked_out, ButtonTone::Secondary),
            ("claim", "Claim", !ctx.knocked_out, ButtonTone::Secondary),
            ("item", "Bandage", !ctx.knocked_out, ButtonTone::Secondary),
        ],
        ctx,
        actions,
    );
    draw_button_row(
        content,
        top + 264.0,
        22.0,
        mouse,
        &[
            ("spell", "Spell", !ctx.knocked_out, ButtonTone::Secondary),
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
            ("knowledge", "Knowledge", true, ButtonTone::Secondary),
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
            ("travel", "Travel", true, ButtonTone::Primary),
            ("recover-travel", "Recover", true, ButtonTone::Primary),
            (
                "route-repair",
                "Repair",
                ctx.regional_region.is_some_and(|region| {
                    region.routes.iter().any(|route| {
                        route.origin_location_id == region.player_location_id
                            && route.status != RouteStatus::Operational
                    })
                }),
                ButtonTone::Positive,
            ),
            ("market-region", "Market", true, ButtonTone::Primary),
            ("region-event", "Event", true, ButtonTone::Primary),
            ("region-details", "Inspect", true, ButtonTone::Secondary),
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
        Rect::new(content.x, top + 101.0, content.w, 34.0),
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
