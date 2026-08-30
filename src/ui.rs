//! Virtual-resolution UI for the Phase 0 first-evening client.

use crate::data::ActionDef;
use crate::network::{ConnectionState, RemotePlayer};
use crate::state::{tile_color, CropState, TileKind};
use macroquad::prelude::*;
use macroquad_toolkit::grid::TilePos;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;
use macroquad_toolkit::ui::RectExt;

#[path = "ui_crafting.rs"]
mod ui_crafting;
#[path = "ui_map.rs"]
mod ui_map;
#[path = "ui_online.rs"]
mod ui_online;
#[path = "ui_regional.rs"]
mod ui_regional;
pub(super) use ui_map::{draw_landmark, draw_map, MapView};

pub const LOGICAL_WIDTH: f32 = 1280.0;
pub const LOGICAL_HEIGHT: f32 = 720.0;

#[path = "ui_context.rs"]
mod ui_context;
pub use ui_context::{UiAction, UiContext};

const PANEL: Color = Color::new(0.055, 0.075, 0.09, 0.97);
const PANEL_LIGHT: Color = Color::new(0.08, 0.11, 0.13, 0.98);
const LINE: Color = Color::new(0.32, 0.48, 0.50, 0.75);
const CREAM: Color = Color::new(0.91, 0.87, 0.73, 1.0);
const MINT: Color = Color::new(0.50, 0.82, 0.68, 1.0);
const GOLD: Color = Color::new(0.90, 0.69, 0.30, 1.0);

pub fn draw_game_ui(ctx: UiContext<'_>) -> Vec<UiAction> {
    let mouse = ctx.ui.mouse_position();
    let mut actions = Vec::new();

    draw_header(&ctx);
    let map_rect = draw_world_panel(&ctx, mouse);
    if ctx.crafting.is_none() && !ctx.knocked_out && is_mouse_button_released(MouseButton::Left) {
        if let Some(tile) = MapView::new(&ctx, map_rect).tile_at(mouse) {
            actions.push(UiAction::MoveTo(tile));
        }
    }
    draw_sidebar(&ctx, mouse, &mut actions);
    ui_online::draw_regional_inspection(&ctx, mouse, &mut actions);
    ui_online::draw_skill_selection(&ctx, mouse, &mut actions);
    if let Some(crafting) = ctx.crafting {
        ui_crafting::draw(crafting, mouse, &mut actions);
        actions
            .retain(|action| matches!(action, UiAction::Interact(id) if id == "crafting-timing"));
    }
    draw_footer(&ctx);

    if ctx.regional_inspection.is_some() {
        actions.retain(|action| {
            matches!(action, UiAction::RegionalEvent(_))
                || matches!(
                    action,
                    UiAction::Interact(id)
                        if matches!(
                            id.as_str(),
                            "region-details"
                                | "route-repair"
                                | "route-escort"
                                | "route-improve"
                        )
                )
        });
    }
    if ctx.skill_selection_open {
        actions.retain(|action| {
            matches!(action, UiAction::Practice(_))
                || matches!(action, UiAction::Interact(id) if id == "skill-close")
        });
    }

    actions
}

fn draw_header(ctx: &UiContext<'_>) {
    let rect = Rect::new(20.0, 16.0, LOGICAL_WIDTH - 40.0, 64.0);
    draw_surface(
        rect,
        &SurfaceStyle::new(PANEL)
            .with_border(1.0, LINE)
            .with_top_highlight(2.0, Color::new(0.62, 0.82, 0.72, 0.7)),
    );
    draw_ui_text_ex(
        &ctx.data.config.display_name,
        rect.x + 18.0,
        rect.y + 31.0,
        TextStyle::new(28.0, CREAM).params(),
    );
    draw_ui_text_ex(
        if ctx.offline {
            "A local first-evening fixture • never shared with the server"
        } else {
            "The shared road • server-owned world projection"
        },
        rect.x + 20.0,
        rect.y + 51.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    let time = format_clock(ctx.clock_minutes);
    let day_label = ctx
        .calendar_season
        .map(|season| format!("Day {} • {}", ctx.day, season))
        .unwrap_or_else(|| format!("Day {}", ctx.day));
    draw_badge(
        Rect::new(rect.right() - 364.0, rect.y + 18.0, 125.0, 28.0),
        &day_label,
        Color::new(0.16, 0.24, 0.25, 1.0),
        CREAM,
    );
    draw_badge(
        Rect::new(rect.right() - 229.0, rect.y + 18.0, 116.0, 28.0),
        &format!("{} {}", time, ctx.time_of_day.label()),
        if ctx.night {
            Color::new(0.13, 0.16, 0.28, 1.0)
        } else {
            Color::new(0.28, 0.24, 0.14, 1.0)
        },
        CREAM,
    );
    draw_badge(
        Rect::new(rect.right() - 105.0, rect.y + 18.0, 87.0, 28.0),
        ctx.connection.label(),
        match ctx.connection {
            ConnectionState::Online => Color::new(0.16, 0.28, 0.22, 1.0),
            ConnectionState::Connecting => Color::new(0.20, 0.22, 0.18, 1.0),
            ConnectionState::Degraded => Color::new(0.30, 0.22, 0.14, 1.0),
            ConnectionState::Offline => Color::new(0.24, 0.18, 0.16, 1.0),
        },
        if ctx.offline { GOLD } else { MINT },
    );
}

fn draw_world_panel(ctx: &UiContext<'_>, mouse: Vec2) -> Rect {
    let panel = Rect::new(20.0, 96.0, 824.0, 510.0);
    draw_surface_with_title(
        panel,
        Some("The road between the Hearth and Whisperwood"),
        &SurfaceStyle::new(PANEL_LIGHT)
            .with_border(1.0, LINE)
            .with_header(42.0, Color::new(0.09, 0.14, 0.15, 1.0))
            .with_header_divider(1.0, LINE),
        TextStyle::new(17.0, CREAM),
    );

    let map = Rect::new(panel.x + 18.0, panel.y + 61.0, panel.w - 36.0, 398.0);
    draw_map(ctx, map);
    if map.contains_point(mouse) {
        draw_tooltip(
            if ctx.knocked_out {
                "Choose a recovery prompt before walking."
            } else {
                "Tap a walkable tile to take one step toward it."
            },
            mouse,
        );
    }

    draw_text_right(
        &format!(
            "{} ready plots  •  {} reachable tiles  •  zoom {:.1}x",
            ctx.world
                .crops
                .data()
                .iter()
                .filter_map(|crop| *crop)
                .filter(CropState::mature)
                .count(),
            ctx.world.reachable.len(),
            ctx.camera_zoom
        ),
        panel.right() - 18.0,
        panel.bottom() - 17.0,
        TextStyle::new(13.0, dark::TEXT_DIM),
    );
    map
}

fn draw_sidebar(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let panel = Rect::new(864.0, 96.0, 396.0, 510.0);
    draw_surface_with_title(
        panel,
        Some(if ctx.offline {
            "Your evening"
        } else {
            "The shared road"
        }),
        &SurfaceStyle::new(PANEL)
            .with_border(1.0, LINE)
            .with_header(42.0, Color::new(0.09, 0.14, 0.15, 1.0))
            .with_header_divider(1.0, LINE),
        TextStyle::new(17.0, CREAM),
    );

    let content = panel.inset(16.0);
    if ctx.offline {
        draw_offline_sidebar(ctx, content, mouse, actions);
    } else {
        ui_online::draw_sidebar(ctx, content, mouse, actions);
    }
}

fn draw_offline_sidebar(
    ctx: &UiContext<'_>,
    content: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    draw_stats(ctx, Rect::new(content.x, content.y + 34.0, content.w, 56.0));

    draw_ui_text_ex(
        "What will you do?",
        content.x,
        content.y + 113.0,
        TextStyle::new(17.0, CREAM).params(),
    );
    let mut y = content.y + 124.0;
    for id in ["plant", "tend", "harvest", "listen"] {
        if let Some(action) = ctx.data.actions.get(id) {
            let rect = Rect::new(content.x, y, content.w, 38.0);
            if draw_action_card(rect, action, mouse) {
                actions.push(UiAction::Interact(action.id.clone()));
            }
            y += 44.0;
        }
    }

    draw_ui_text_ex(
        "Walk",
        content.x,
        429.0,
        TextStyle::new(16.0, CREAM).params(),
    );
    draw_move_pad(content.x + 77.0, 437.0, mouse, actions, true);

    let save_y = 535.0;
    let half = (content.w - 8.0) * 0.5;
    if virtual_button(
        Rect::new(content.x, save_y, half, 29.0),
        "Save",
        true,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::Save);
    }
    if virtual_button(
        Rect::new(content.x + half + 8.0, save_y, half, 29.0),
        "Load",
        ctx.save_exists,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::Load);
    }
    if virtual_button(
        Rect::new(content.x, 570.0, half, 24.0),
        "New evening",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::NewEvening);
    }
    if virtual_button(
        Rect::new(content.x + half + 8.0, 570.0, half, 24.0),
        "Delete local save",
        ctx.save_exists,
        ButtonTone::Danger,
        mouse,
    ) {
        actions.push(UiAction::DeleteSave);
    }
    if virtual_button(
        Rect::new(content.x, 596.0, content.w, 22.0),
        "Reconnect online",
        true,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::UseOnline);
    }
}

fn draw_stats(ctx: &UiContext<'_>, rect: Rect) {
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.10, 0.14, 0.15, 1.0))
            .with_border(1.0, Color::new(0.32, 0.48, 0.50, 0.45)),
    );
    draw_text_block(
        ctx.stats,
        rect.x + 12.0,
        rect.y + 17.0,
        rect.w - 24.0,
        rect.h - 8.0,
        13.0,
        2.0,
        dark::TEXT,
    );
}

fn draw_action_card(rect: Rect, action: &ActionDef, mouse: Vec2) -> bool {
    let hovered = rect.contains_point(mouse);
    let fill = if hovered {
        Color::new(0.17, 0.24, 0.23, 1.0)
    } else {
        Color::new(0.10, 0.15, 0.16, 1.0)
    };
    draw_surface(
        rect,
        &SurfaceStyle::new(fill)
            .with_left_accent(4.0, if hovered { GOLD } else { MINT })
            .with_border(1.0, Color::new(0.35, 0.51, 0.50, 0.45)),
    );
    draw_ui_text_ex(
        &action.name,
        rect.x + 13.0,
        rect.y + 18.0,
        TextStyle::new(14.0, CREAM).params(),
    );
    draw_ui_text_ex(
        &action.description,
        rect.x + 13.0,
        rect.y + 34.0,
        TextStyle::new(10.0, dark::TEXT_DIM).params(),
    );
    hovered && is_mouse_button_released(MouseButton::Left)
}

fn draw_move_pad(x: f32, y: f32, mouse: Vec2, actions: &mut Vec<UiAction>, enabled: bool) {
    let size = 28.0;
    let gap = 4.0;
    let button = |x: f32, y: f32, label: &str, action: &mut Vec<UiAction>| {
        if virtual_button(
            Rect::new(x, y, size, size),
            label,
            enabled,
            ButtonTone::Secondary,
            mouse,
        ) {
            match label {
                "^" => action.push(UiAction::Move(0, -1)),
                "<" => action.push(UiAction::Move(-1, 0)),
                ">" => action.push(UiAction::Move(1, 0)),
                "v" => action.push(UiAction::Move(0, 1)),
                _ => {}
            }
        }
    };
    button(x + size + gap, y, "^", actions);
    button(x, y + size + gap, "<", actions);
    virtual_button(
        Rect::new(x + size + gap, y + size + gap, size, size),
        "•",
        false,
        ButtonTone::Secondary,
        mouse,
    );
    button(x + (size + gap) * 2.0, y + size + gap, ">", actions);
    button(x + size + gap, y + (size + gap) * 2.0, "v", actions);
}

fn virtual_button(rect: Rect, label: &str, enabled: bool, tone: ButtonTone, mouse: Vec2) -> bool {
    let style = ButtonStyle::from_tone(tone);
    let hovered = enabled && rect.contains_point(mouse);
    let pressed = hovered && is_mouse_button_down(MouseButton::Left);
    let fill = if !enabled {
        style.disabled
    } else if pressed {
        style.pressed
    } else if hovered {
        style.hovered
    } else {
        style.normal
    };
    draw_surface(
        rect,
        &SurfaceStyle::new(fill).with_border(1.0, style.border),
    );
    draw_text_centered_in_box_ex(
        label,
        rect.x + 5.0,
        rect.y + if pressed { 1.0 } else { 0.0 },
        rect.w - 10.0,
        rect.h,
        TextStyle::new(
            if label.len() > 2 { 13.0 } else { 18.0 },
            if enabled {
                style.text_color
            } else {
                dark::TEXT_DIM
            },
        ),
    );
    hovered && is_mouse_button_released(MouseButton::Left)
}

fn draw_footer(ctx: &UiContext<'_>) {
    let rect = Rect::new(20.0, 620.0, LOGICAL_WIDTH - 40.0, 84.0);
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.045, 0.065, 0.075, 0.98))
            .with_border(1.0, Color::new(0.32, 0.48, 0.50, 0.6)),
    );
    draw_ui_text_ex(
        if ctx.offline {
            "OFFLINE DEVELOPMENT FIXTURE"
        } else {
            "SERVER CHRONICLE"
        },
        rect.x + 16.0,
        rect.y + 22.0,
        TextStyle::new(12.0, MINT).params(),
    );
    draw_text_block(
        ctx.status_message,
        rect.x + 16.0,
        rect.y + 38.0,
        540.0,
        28.0,
        14.0,
        2.0,
        CREAM,
    );
    draw_ui_text_ex(
        if ctx.offline {
            "LOCAL ONLY"
        } else {
            "AUTHORITATIVE ROAD"
        },
        rect.x + 600.0,
        rect.y + 22.0,
        TextStyle::new(12.0, GOLD).params(),
    );
    draw_text_block(
        &format!(
            "{}\n{} players • {} crop types • {} assets • {} saved slot(s)",
            if ctx.offline {
                "No online state is mixed into this fixture"
            } else {
                "Movement, clock, presence, and chat come from the server"
            },
            ctx.remote_players.len(),
            ctx.data.crops.len(),
            ctx.loaded_assets,
            ctx.save_slots.len()
        ),
        rect.x + 600.0,
        rect.y + 38.0,
        rect.w - 616.0,
        30.0,
        13.0,
        2.0,
        dark::TEXT_DIM,
    );
}

fn format_clock(minutes: u32) -> String {
    format!("{:02}:{:02}", (minutes / 60) % 24, minutes % 60)
}
