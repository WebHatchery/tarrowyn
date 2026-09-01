//! Floating status, quick actions, and the expandable tools deck.

use super::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

#[path = "ui_hud/tools.rs"]
mod tools;

pub(crate) const HUD_HEIGHT: f32 = 64.0;

const HUD_FILL: Color = Color::new(0.025, 0.040, 0.043, 0.78);
const HUD_CARD: Color = Color::new(0.035, 0.058, 0.060, 0.76);

pub(super) fn blocks_map_click(point: Vec2, menu_open: bool) -> bool {
    menu_open
        || point.y >= LOGICAL_HEIGHT - HUD_HEIGHT
        || point.y <= 52.0
        || Rect::new(16.0, 12.0, 238.0, 28.0).contains_point(point)
        || Rect::new(270.0, 12.0, 420.0, 28.0).contains_point(point)
        || Rect::new(980.0, 12.0, 284.0, 28.0).contains_point(point)
}

pub(super) fn draw(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    draw_status_hud(ctx);
    if ctx.menu_open {
        draw_tools_overlay(ctx, mouse, actions);
    } else {
        draw_command_deck(ctx, mouse, actions);
    }
}

fn draw_status_hud(ctx: &UiContext<'_>) {
    draw_hud_card(
        Rect::new(16.0, 12.0, 238.0, 28.0),
        Color::new(0.88, 0.64, 0.25, 0.78),
    );
    draw_ui_text_ex(
        &ellipsize(ctx.identity_name.unwrap_or("Guest traveller"), 26),
        28.0,
        30.0,
        TextStyle::new(11.0, CREAM).params(),
    );

    draw_hud_card(Rect::new(270.0, 12.0, 420.0, 28.0), MINT);
    let overview = ctx
        .stats
        .lines()
        .next()
        .unwrap_or("The ledger is arriving…");
    let inventory = ctx
        .stats
        .lines()
        .find(|line| line.starts_with("Wheat") || line.starts_with("Turnips"))
        .unwrap_or("Inventory is arriving…");
    draw_ui_text_ex(
        &ellipsize(&format!("{overview}  •  {inventory}"), 65),
        284.0,
        30.0,
        TextStyle::new(9.0, CREAM).params(),
    );

    let day_label = ctx
        .calendar_season
        .map(|season| format!("DAY {}  /  {}", ctx.day, season.to_ascii_uppercase()))
        .unwrap_or_else(|| format!("DAY {}", ctx.day));
    draw_hud_badge(
        Rect::new(980.0, 12.0, 120.0, 28.0),
        &day_label,
        Color::new(0.12, 0.20, 0.20, 0.94),
        CREAM,
    );
    draw_hud_badge(
        Rect::new(1108.0, 12.0, 70.0, 28.0),
        &format_clock(ctx.clock_minutes),
        if ctx.night {
            Color::new(0.13, 0.15, 0.28, 0.96)
        } else {
            Color::new(0.27, 0.20, 0.10, 0.96)
        },
        CREAM,
    );
    draw_hud_badge(
        Rect::new(1186.0, 12.0, 78.0, 28.0),
        ctx.connection.label(),
        connection_fill(ctx),
        if ctx.offline { GOLD } else { MINT },
    );
}

fn draw_command_deck(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let dock = Rect::new(0.0, LOGICAL_HEIGHT - HUD_HEIGHT, LOGICAL_WIDTH, HUD_HEIGHT);
    draw_rectangle(dock.x, dock.y, dock.w, dock.h, HUD_FILL);
    draw_rectangle(
        dock.x,
        dock.y,
        dock.w,
        2.0,
        Color::new(0.50, 0.82, 0.68, 0.70),
    );
    draw_ui_text_ex(
        if ctx.offline { "FIELD" } else { "ROAD" },
        18.0,
        dock.y + 14.0,
        TextStyle::new(8.0, MINT).params(),
    );

    let content = Rect::new(18.0, dock.y + 24.0, 1122.0, 28.0);
    if ctx.offline {
        draw_offline_quick_row(content, mouse, actions);
    } else {
        super::ui_online::draw_button_row(
            content,
            content.y,
            content.h,
            mouse,
            &[
                ("plant", "Plant", true, ButtonTone::Positive),
                ("tend", "Tend", true, ButtonTone::Positive),
                ("harvest", "Harvest", true, ButtonTone::Positive),
                ("animal", "Care", true, ButtonTone::Positive),
                ("trade", "Trade", true, ButtonTone::Primary),
                ("say-hello", "Meet", true, ButtonTone::Secondary),
                ("practice", "Practice", true, ButtonTone::Primary),
                ("menu-toggle", "All tools", true, ButtonTone::Secondary),
            ],
            ctx,
            actions,
        );
    }
    draw_compact_move_strip(
        1152.0,
        dock.y + 24.0,
        mouse,
        actions,
        super::ui_online::movement_enabled(ctx),
    );
}

fn draw_compact_move_strip(
    x: f32,
    y: f32,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
    enabled: bool,
) {
    for (index, (label, dx, dy)) in [("<", -1, 0), ("^", 0, -1), ("v", 0, 1), (">", 1, 0)]
        .iter()
        .enumerate()
    {
        let rect = Rect::new(x + index as f32 * 27.0, y, 24.0, 28.0);
        if super::virtual_button(rect, label, enabled, ButtonTone::Secondary, mouse) {
            actions.push(UiAction::Move(*dx, *dy));
        }
    }
}

fn draw_tools_overlay(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    draw_rectangle(
        0.0,
        0.0,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        Color::new(0.005, 0.012, 0.014, 0.66),
    );
    let panel = Rect::new(58.0, 82.0, 1164.0, 556.0);
    draw_surface(
        panel,
        &SurfaceStyle::new(Color::new(0.025, 0.045, 0.046, 0.98))
            .with_shadow(vec2(0.0, 8.0), Color::new(0.0, 0.0, 0.0, 0.42))
            .with_left_accent(3.0, GOLD)
            .with_top_highlight(2.0, Color::new(0.50, 0.82, 0.68, 0.65)),
    );
    draw_ui_text_ex(
        "TOOLS & LEDGERS",
        panel.x + 28.0,
        panel.y + 34.0,
        TextStyle::new(22.0, CREAM).params(),
    );
    draw_ui_text_ex(
        "The road stays visible while you choose a service.",
        panel.x + 30.0,
        panel.y + 56.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );
    draw_ui_text_ex(
        &ellipsize(ctx.status_message, 96),
        panel.right() - 400.0,
        panel.y + 42.0,
        TextStyle::new(10.0, MINT).params(),
    );

    if ctx.offline {
        draw_offline_tools(panel, mouse, actions, ctx);
    } else {
        tools::draw_online_tools(panel, mouse, actions, ctx);
    }
    if super::virtual_button(
        Rect::new(panel.right() - 138.0, panel.bottom() - 44.0, 110.0, 28.0),
        "Back",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("menu-close".to_owned()));
    }
}

fn draw_offline_quick_row(content: Rect, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let entries = [
        ("plant", "Plant", ButtonTone::Positive),
        ("tend", "Tend", ButtonTone::Positive),
        ("harvest", "Harvest", ButtonTone::Positive),
        ("listen", "Listen", ButtonTone::Primary),
        ("save", "Save", ButtonTone::Secondary),
        ("load", "Load", ButtonTone::Secondary),
        ("new", "New evening", ButtonTone::Secondary),
        ("menu-toggle", "All tools", ButtonTone::Secondary),
    ];
    let gap = 5.0;
    let width = (content.w - gap * (entries.len() - 1) as f32) / entries.len() as f32;
    for (index, (id, label, tone)) in entries.iter().enumerate() {
        if super::virtual_button(
            Rect::new(
                content.x + index as f32 * (width + gap),
                content.y,
                width,
                content.h,
            ),
            label,
            true,
            *tone,
            mouse,
        ) {
            actions.push(match *id {
                "save" => UiAction::Save,
                "load" => UiAction::Load,
                "new" => UiAction::NewEvening,
                "menu-toggle" => UiAction::Interact("menu-toggle".to_owned()),
                _ => UiAction::Interact((*id).to_owned()),
            });
        }
    }
}

fn draw_offline_tools(panel: Rect, mouse: Vec2, actions: &mut Vec<UiAction>, ctx: &UiContext<'_>) {
    let content = Rect::new(panel.x + 28.0, panel.y + 102.0, panel.w - 56.0, 34.0);
    let entries = [
        ("plant", "Plant", ButtonTone::Positive),
        ("tend", "Tend", ButtonTone::Positive),
        ("harvest", "Harvest", ButtonTone::Positive),
        ("listen", "Listen", ButtonTone::Primary),
        ("save", "Save", ButtonTone::Secondary),
        ("load", "Load", ButtonTone::Secondary),
    ];
    let gap = 6.0;
    let width = (content.w - gap * (entries.len() - 1) as f32) / entries.len() as f32;
    for (index, (id, label, tone)) in entries.iter().enumerate() {
        if super::virtual_button(
            Rect::new(
                content.x + index as f32 * (width + gap),
                content.y,
                width,
                content.h,
            ),
            label,
            *id != "load" || ctx.save_exists,
            *tone,
            mouse,
        ) {
            actions.push(match *id {
                "save" => UiAction::Save,
                "load" => UiAction::Load,
                _ => UiAction::Interact((*id).to_owned()),
            });
        }
    }
    draw_ui_text_ex(
        "LOCAL LEDGER",
        content.x,
        content.y + 76.0,
        TextStyle::new(9.0, GOLD).params(),
    );
    draw_ui_text_ex(
        &ellipsize(ctx.status_message, 110),
        content.x,
        content.y + 98.0,
        TextStyle::new(13.0, CREAM).params(),
    );
    if super::virtual_button(
        Rect::new(content.x, content.y + 128.0, 220.0, 30.0),
        "New evening",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::NewEvening);
    }
    if super::virtual_button(
        Rect::new(content.x + 232.0, content.y + 128.0, 280.0, 30.0),
        "Reconnect online",
        true,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::UseOnline);
    }
}

fn draw_hud_card(rect: Rect, accent: Color) {
    draw_surface(
        rect,
        &SurfaceStyle::new(HUD_CARD)
            .with_shadow(vec2(0.0, 3.0), Color::new(0.0, 0.0, 0.0, 0.24))
            .with_left_accent(3.0, accent),
    );
}

fn draw_hud_badge(rect: Rect, label: &str, fill: Color, text: Color) {
    draw_surface(
        rect,
        &SurfaceStyle::new(fill).with_top_highlight(1.0, Color::new(1.0, 1.0, 1.0, 0.18)),
    );
    draw_text_centered_in_box_ex(
        label,
        rect.x + 5.0,
        rect.y,
        rect.w - 10.0,
        rect.h,
        TextStyle::new(10.0, text),
    );
}

fn connection_fill(ctx: &UiContext<'_>) -> Color {
    match ctx.connection {
        ConnectionState::Online => Color::new(0.10, 0.25, 0.18, 0.96),
        ConnectionState::Connecting => Color::new(0.18, 0.21, 0.14, 0.96),
        ConnectionState::Degraded => Color::new(0.28, 0.20, 0.10, 0.96),
        ConnectionState::Offline => Color::new(0.25, 0.15, 0.13, 0.96),
    }
}

fn ellipsize(value: &str, max_chars: usize) -> String {
    let line = value.lines().next().unwrap_or_default();
    let mut chars = line.chars();
    let compact: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{compact}…")
    } else {
        compact
    }
}
