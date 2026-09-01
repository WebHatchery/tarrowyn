//! Floating status, quick actions, and the expandable tools deck.

use super::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

#[path = "ui_hud/tools.rs"]
mod tools;

pub(crate) const HUD_HEIGHT: f32 = 132.0;

const HUD_FILL: Color = Color::new(0.025, 0.040, 0.043, 0.90);
const HUD_CARD: Color = Color::new(0.035, 0.058, 0.060, 0.88);

pub(super) fn blocks_map_click(point: Vec2, menu_open: bool) -> bool {
    menu_open
        || point.y >= LOGICAL_HEIGHT - HUD_HEIGHT
        || Rect::new(18.0, 16.0, 282.0, 56.0).contains_point(point)
        || Rect::new(316.0, 16.0, 408.0, 56.0).contains_point(point)
        || Rect::new(764.0, 16.0, 330.0, 56.0).contains_point(point)
        || Rect::new(1106.0, 16.0, 148.0, 56.0).contains_point(point)
        || Rect::new(1158.0, 82.0, 104.0, 104.0).contains_point(point)
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
        Rect::new(18.0, 16.0, 282.0, 56.0),
        Color::new(0.88, 0.64, 0.25, 0.90),
    );
    draw_ui_text_ex(
        ctx.identity_name.unwrap_or("Guest traveller"),
        34.0,
        39.0,
        TextStyle::new(17.0, CREAM).params(),
    );
    draw_ui_text_ex(
        if ctx.offline {
            "LOCAL EVENING  /  THE HEARTH"
        } else {
            "SHARED ROAD  /  THE HEARTH"
        },
        34.0,
        58.0,
        TextStyle::new(10.0, dark::TEXT_DIM).params(),
    );

    draw_hud_card(Rect::new(316.0, 16.0, 408.0, 56.0), MINT);
    let overview = ctx
        .stats
        .lines()
        .next()
        .unwrap_or("The ledger is arriving…");
    let inventory = ctx
        .stats
        .lines()
        .find(|line| line.starts_with("Wheat") || line.starts_with("Gold"))
        .unwrap_or("Inventory is arriving…");
    draw_ui_text_ex(overview, 334.0, 40.0, TextStyle::new(12.0, CREAM).params());
    draw_ui_text_ex(
        inventory,
        334.0,
        59.0,
        TextStyle::new(10.0, dark::TEXT_DIM).params(),
    );

    draw_hud_card(Rect::new(740.0, 16.0, 354.0, 56.0), GOLD);
    let feed = ctx
        .tavern_notices
        .iter()
        .rev()
        .find(|notice| !notice.text.trim().is_empty())
        .map(|notice| notice.text.as_str())
        .or_else(|| ctx.tavern_rumours.first().map(String::as_str))
        .or_else(|| ctx.chat.last().map(|message| message.text.as_str()))
        .unwrap_or(ctx.status_message);
    draw_ui_text_ex(
        "LATEST FROM THE HEARTH",
        758.0,
        35.0,
        TextStyle::new(9.0, GOLD).params(),
    );
    draw_ui_text_ex(
        &ellipsize(feed, 48),
        758.0,
        56.0,
        TextStyle::new(10.0, CREAM).params(),
    );

    let day_label = ctx
        .calendar_season
        .map(|season| format!("DAY {}  /  {}", ctx.day, season.to_ascii_uppercase()))
        .unwrap_or_else(|| format!("DAY {}", ctx.day));
    draw_hud_badge(
        Rect::new(1106.0, 16.0, 148.0, 26.0),
        &day_label,
        Color::new(0.12, 0.20, 0.20, 0.94),
        CREAM,
    );
    draw_hud_badge(
        Rect::new(1106.0, 46.0, 72.0, 26.0),
        &format_clock(ctx.clock_minutes),
        if ctx.night {
            Color::new(0.13, 0.15, 0.28, 0.96)
        } else {
            Color::new(0.27, 0.20, 0.10, 0.96)
        },
        CREAM,
    );
    draw_hud_badge(
        Rect::new(1184.0, 46.0, 70.0, 26.0),
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
        if ctx.offline {
            "FIELD COMMAND"
        } else {
            "ROAD COMMAND"
        },
        24.0,
        dock.y + 23.0,
        TextStyle::new(11.0, MINT).params(),
    );
    draw_ui_text_ex(
        &ellipsize(command_hint(ctx), 112),
        155.0,
        dock.y + 23.0,
        TextStyle::new(10.0, dark::TEXT_DIM).params(),
    );
    draw_ui_text_ex(
        "MORE TOOLS",
        1144.0,
        dock.y + 23.0,
        TextStyle::new(9.0, GOLD).params(),
    );

    let content = Rect::new(24.0, dock.y + 40.0, LOGICAL_WIDTH - 48.0, 34.0);
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

    let detail = deck_detail(ctx);
    draw_ui_text_ex(
        &ellipsize(&detail, 156),
        24.0,
        dock.y + 102.0,
        TextStyle::new(10.0, CREAM).params(),
    );
    draw_ui_text_ex(
        if ctx.offline {
            "Tap a walkable tile to travel  •  progress is stored locally"
        } else {
            "Tap a walkable tile to travel  •  the shared road resolves each step"
        },
        24.0,
        dock.y + 123.0,
        TextStyle::new(10.0, dark::TEXT_DIM).params(),
    );
    draw_ui_text_ex(
        if ctx.offline {
            "LOCAL"
        } else {
            "LIVE PRESENCE"
        },
        1154.0,
        dock.y + 102.0,
        TextStyle::new(9.0, if ctx.offline { GOLD } else { MINT }).params(),
    );
    draw_ui_text_ex(
        &format!(
            "{} companion{} nearby",
            super::ui_online::visible_companion_count(
                ctx.remote_players,
                ctx.own_account_id,
                ctx.server_tick
            ),
            if super::ui_online::visible_companion_count(
                ctx.remote_players,
                ctx.own_account_id,
                ctx.server_tick,
            ) == 1
            {
                ""
            } else {
                "s"
            }
        ),
        1154.0,
        dock.y + 123.0,
        TextStyle::new(10.0, dark::TEXT_DIM).params(),
    );

    draw_move_pad(
        1158.0,
        82.0,
        mouse,
        actions,
        !ctx.menu_open && super::ui_online::movement_enabled(ctx),
    );
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

fn command_hint(ctx: &UiContext<'_>) -> &'static str {
    if ctx.offline {
        "Choose a field action, or tap a tile to walk the first evening."
    } else if ctx.knocked_out {
        "Recovery is ready in the tools deck."
    } else {
        "Shape the road with the people and places around you."
    }
}

fn deck_detail(ctx: &UiContext<'_>) -> String {
    if ctx.offline {
        return format!(
            "{} ready plot{}  •  {} saved slot{}",
            ctx.world
                .crops
                .data()
                .iter()
                .filter_map(|crop| *crop)
                .filter(crate::state::CropState::mature)
                .count(),
            if ctx
                .world
                .crops
                .data()
                .iter()
                .filter_map(|crop| *crop)
                .filter(crate::state::CropState::mature)
                .count()
                == 1
            {
                ""
            } else {
                "s"
            },
            ctx.save_slots.len(),
            if ctx.save_slots.len() == 1 { "" } else { "s" },
        );
    }
    super::online_footer_detail(
        ctx.stats,
        super::ui_online::visible_player_count(ctx.remote_players, ctx.server_tick),
        super::pending_trade_detail(ctx.trades, ctx.own_account_id).as_deref(),
    )
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
