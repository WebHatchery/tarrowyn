use super::*;
use tarrowyn_protocol::{RegionSnapshot, RouteStatus};

pub fn combat_side_control(
    combat: Option<&tarrowyn_protocol::LocalCombatState>,
    frontier_threat_active: bool,
) -> (&'static str, &'static str) {
    if combat.is_some_and(|combat| combat.status == tarrowyn_protocol::LocalCombatStatus::Engaged) {
        ("retreat", "Retreat")
    } else if frontier_threat_active {
        ("frontier-retreat", "Retreat")
    } else {
        ("contract", "Contract")
    }
}

pub fn draw_regional_inspection(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
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
            (
                "route-escort",
                "Escort road",
                has_open_local_route(ctx.regional_region),
                ButtonTone::Positive,
            ),
            (
                "route-improve",
                "Improve road",
                has_open_local_route(ctx.regional_region),
                ButtonTone::Primary,
            ),
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

pub(super) fn has_open_local_route(region: Option<&RegionSnapshot>) -> bool {
    region.is_some_and(|region| {
        region.routes.iter().any(|route| {
            (route.origin_location_id == region.player_location_id
                || route.destination_location_id == region.player_location_id)
                && route.status != RouteStatus::Closed
        })
    })
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

pub fn draw_skill_selection(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    if !ctx.skill_selection_open {
        return;
    }
    let panel = Rect::new(42.0, 126.0, 780.0, 438.0);
    draw_surface_with_title(
        panel,
        Some("Choose a discipline"),
        &SurfaceStyle::new(Color::new(0.055, 0.075, 0.09, 0.99))
            .with_border(1.0, Color::new(0.50, 0.72, 0.62, 0.9))
            .with_header(42.0, Color::new(0.09, 0.14, 0.15, 1.0))
            .with_header_divider(1.0, Color::new(0.32, 0.48, 0.50, 0.75)),
        TextStyle::new(17.0, CREAM),
    );
    draw_text_block(
        "Tap a depth-one practice to begin or continue it. Advanced arts emerge from play and are not direct choices here.",
        panel.x + 20.0,
        panel.y + 70.0,
        panel.w - 40.0,
        34.0,
        13.0,
        2.0,
        CREAM,
    );
    let choices: Vec<_> = ctx
        .skills
        .iter()
        .filter(|skill| skill_practice_choice(skill))
        .collect();
    if choices.is_empty() {
        draw_ui_text_ex(
            if ctx.skills.is_empty() {
                "The skill ledger is loading; tap Close and try again shortly."
            } else {
                "Every depth-one practice is mastered; advanced discoveries remain in the ledger."
            },
            panel.x + 20.0,
            panel.y + 138.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
    } else {
        draw_ui_text_ex(
            &format!("{} depth-one practices available", choices.len()),
            panel.x + 20.0,
            panel.y + 133.0,
            TextStyle::new(11.0, MINT).params(),
        );
        draw_skill_choices(panel, &choices, mouse, actions);
    }
    if virtual_button(
        Rect::new(panel.right() - 126.0, panel.bottom() - 42.0, 106.0, 28.0),
        "Close",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("skill-close".to_owned()));
    }
}

fn draw_skill_choices(
    panel: Rect,
    choices: &[&tarrowyn_protocol::SkillView],
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    let columns = 4;
    let gap = 4.0;
    let width = (panel.w - 40.0 - gap * (columns - 1) as f32) / columns as f32;
    let height = 25.0;
    for (index, skill) in choices.iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        if virtual_button(
            Rect::new(
                panel.x + 20.0 + column as f32 * (width + gap),
                panel.y + 148.0 + row as f32 * (height + gap),
                width,
                height,
            ),
            &skill.name,
            true,
            ButtonTone::Primary,
            mouse,
        ) {
            actions.push(UiAction::Practice(skill.skill_id.clone()));
        }
    }
}

pub fn skill_practice_choice(skill: &tarrowyn_protocol::SkillView) -> bool {
    skill.depth == 1
        && matches!(
            skill.status,
            tarrowyn_protocol::SkillStatus::Available | tarrowyn_protocol::SkillStatus::Practising
        )
}

pub fn draw_combat_status(ctx: &UiContext<'_>, content: Rect, top: f32) {
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

pub fn recovery_risk_label(carried_risk: &str) -> &'static str {
    if carried_risk.to_ascii_lowercase().contains("seed") {
        "1 carried seed"
    } else {
        "carried item"
    }
}

pub fn draw_button_row(
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
