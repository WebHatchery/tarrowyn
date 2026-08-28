//! Virtual-resolution UI for the Phase 0 first-evening client.

use crate::data::{ActionDef, GameData};
use crate::network::{ConnectionState, CraftingView, RemotePlayer};
use crate::state::{tile_color, CropState, TileKind, WorldState};
use macroquad::prelude::*;
use macroquad_toolkit::grid::TilePos;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;
use macroquad_toolkit::ui::{RectExt, VirtualUi};
use tarrowyn_protocol::{ChatMessage, ChronicleEntry, OpportunitySignal, WildernessZone};

#[path = "ui_crafting.rs"]
mod ui_crafting;
#[path = "ui_online.rs"]
mod ui_online;

pub const LOGICAL_WIDTH: f32 = 1280.0;
pub const LOGICAL_HEIGHT: f32 = 720.0;

const PANEL: Color = Color::new(0.055, 0.075, 0.09, 0.97);
const PANEL_LIGHT: Color = Color::new(0.08, 0.11, 0.13, 0.98);
const LINE: Color = Color::new(0.32, 0.48, 0.50, 0.75);
const CREAM: Color = Color::new(0.91, 0.87, 0.73, 1.0);
const MINT: Color = Color::new(0.50, 0.82, 0.68, 1.0);
const GOLD: Color = Color::new(0.90, 0.69, 0.30, 1.0);

#[derive(Debug, Clone, PartialEq)]
pub enum UiAction {
    NewEvening,
    UseOnline,
    UseOffline,
    Reconnect,
    Save,
    Load,
    DeleteSave,
    Move(i32, i32),
    MoveTo(TilePos),
    Interact(String),
    SendChat,
    QuickChat(String),
    Zoom(f32),
}

pub struct UiContext<'a> {
    pub data: &'a GameData,
    pub world: &'a WorldState,
    pub player_position: TilePos,
    pub day: u32,
    pub clock_minutes: u32,
    pub night: bool,
    pub stats: &'a str,
    pub own_account_id: Option<&'a str>,
    pub remote_players: &'a [RemotePlayer],
    pub farm_animals: &'a [tarrowyn_protocol::FarmAnimal],
    pub chat: &'a [ChatMessage],
    pub chat_draft: &'a str,
    pub server_tick: u64,
    pub connection: ConnectionState,
    pub status_message: &'a str,
    pub identity_name: Option<&'a str>,
    pub offline: bool,
    pub save_exists: bool,
    pub save_slots: &'a [String],
    pub loaded_assets: usize,
    pub camera_zoom: f32,
    pub wilderness: Option<&'a WildernessZone>,
    pub chronicle: &'a [ChronicleEntry],
    pub opportunities: &'a [OpportunitySignal],
    pub phase4_summary: &'a str,
    pub phase5_summary: &'a str,
    pub crafting: Option<CraftingView>,
    pub knocked_out: bool,
    pub ui: &'a VirtualUi,
}

pub fn draw_game_ui(ctx: UiContext<'_>) -> Vec<UiAction> {
    let mouse = ctx.ui.mouse_position();
    let mut actions = Vec::new();

    draw_header(&ctx);
    let map_rect = draw_world_panel(&ctx, mouse);
    if ctx.crafting.is_none() && is_mouse_button_released(MouseButton::Left) {
        if let Some(tile) = MapView::new(&ctx, map_rect).tile_at(mouse) {
            actions.push(UiAction::MoveTo(tile));
        }
    }
    draw_sidebar(&ctx, mouse, &mut actions);
    if let Some(crafting) = ctx.crafting {
        ui_crafting::draw(crafting, mouse, &mut actions);
        actions
            .retain(|action| matches!(action, UiAction::Interact(id) if id == "crafting-timing"));
    }
    draw_footer(&ctx);

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
    draw_badge(
        Rect::new(rect.right() - 326.0, rect.y + 18.0, 105.0, 28.0),
        &format!("Day {}", ctx.day),
        Color::new(0.16, 0.24, 0.25, 1.0),
        CREAM,
    );
    draw_badge(
        Rect::new(rect.right() - 211.0, rect.y + 18.0, 94.0, 28.0),
        &time,
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
        draw_tooltip("Tap a walkable tile to take one step toward it.", mouse);
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

fn draw_map(ctx: &UiContext<'_>, rect: Rect) {
    let view = MapView::new(ctx, rect);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.04, 0.09, 0.10, 1.0),
    );

    for (pos, tile) in ctx.world.tiles.iter_with_pos() {
        let tile_rect = view.tile_rect(pos);
        if !rect.overlaps(&tile_rect) {
            continue;
        }
        let fill = tile_color(*tile);
        draw_rectangle(tile_rect.x, tile_rect.y, tile_rect.w, tile_rect.h, fill);
        draw_rectangle_lines(
            tile_rect.x,
            tile_rect.y,
            tile_rect.w,
            tile_rect.h,
            1.0,
            Color::new(0.05, 0.11, 0.11, 0.25),
        );
        draw_tile_detail(*tile, tile_rect);
        if let Some(Some(crop)) = ctx.world.crops.get(pos) {
            draw_crop(crop, tile_rect);
        }
    }
    for animal in ctx.farm_animals {
        let tile = TilePos::new(animal.position.x, animal.position.y);
        let tile_rect = view.tile_rect(tile);
        if rect.overlaps(&tile_rect) {
            draw_farm_animal(animal, tile_rect);
        }
    }

    if ctx.night {
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.03, 0.04, 0.13, 0.32),
        );
    }

    draw_landmark(
        &view,
        TilePos::new(8, 5),
        "THE HEARTH",
        Color::new(0.76, 0.46, 0.25, 1.0),
    );
    draw_landmark(
        &view,
        TilePos::new(4, 4),
        "SHARED FIELDS",
        Color::new(0.78, 0.69, 0.30, 1.0),
    );
    draw_landmark(
        &view,
        TilePos::new(14, 3),
        "WHISPERWOOD",
        Color::new(0.45, 0.78, 0.58, 1.0),
    );
    draw_character(&view, ctx.player_position, CREAM, true);
    for (index, player) in ctx.remote_players.iter().enumerate() {
        if ctx.own_account_id == Some(player.account_id.as_str()) {
            continue;
        }
        draw_remote_character(&view, player, index, ctx.server_tick);
    }
}

fn draw_tile_detail(tile: TileKind, rect: Rect) {
    let center = rect.center();
    match tile {
        TileKind::Water => {
            draw_line(
                rect.x + 5.0,
                center.y - 4.0,
                rect.right() - 5.0,
                center.y - 4.0,
                1.0,
                Color::new(0.45, 0.78, 0.86, 0.55),
            );
            draw_line(
                rect.x + 10.0,
                center.y + 5.0,
                rect.right() - 3.0,
                center.y + 5.0,
                1.0,
                Color::new(0.45, 0.78, 0.86, 0.35),
            );
        }
        TileKind::Forest => {
            draw_circle_at(
                center + vec2(-4.0, 3.0),
                rect.w * 0.22,
                Color::new(0.07, 0.18, 0.13, 1.0),
            );
            draw_circle_at(
                center + vec2(5.0, -4.0),
                rect.w * 0.25,
                Color::new(0.09, 0.24, 0.17, 1.0),
            );
        }
        TileKind::Field => {
            for offset in [-0.25, 0.0, 0.25] {
                draw_line(
                    center.x + rect.w * offset,
                    rect.y + 6.0,
                    center.x + rect.w * offset,
                    rect.bottom() - 6.0,
                    1.0,
                    Color::new(0.78, 0.59, 0.26, 0.4),
                );
            }
        }
        TileKind::Stone => {
            draw_circle_at(center, rect.w * 0.22, Color::new(0.52, 0.55, 0.52, 0.85));
        }
        TileKind::Meadow | TileKind::Path => {
            draw_circle_at(
                center + vec2(-8.0, 7.0),
                1.5,
                Color::new(0.72, 0.79, 0.47, 0.55),
            );
        }
    }
}

fn draw_crop(crop: &CropState, rect: Rect) {
    let color = match crop.kind {
        crate::state::CropKind::Wheat => Color::new(0.95, 0.76, 0.26, 1.0),
        crate::state::CropKind::Turnip => Color::new(0.82, 0.62, 0.90, 1.0),
        crate::state::CropKind::Moonberry => Color::new(0.55, 0.68, 0.95, 1.0),
    };
    let size = 3.0 + crop.stage as f32 * 1.5;
    draw_circle_at(rect.center() + vec2(0.0, 3.0), size, color);
    draw_line(
        rect.center().x,
        rect.center().y + 3.0,
        rect.center().x,
        rect.center().y - 7.0,
        2.0,
        MINT,
    );
}

fn draw_farm_animal(animal: &tarrowyn_protocol::FarmAnimal, rect: Rect) {
    let center = rect.center();
    let condition_ratio = animal.condition as f32 / animal.max_condition.max(1) as f32;
    let body = Color::new(0.74, 0.62 + condition_ratio * 0.12, 0.42, 1.0);
    draw_ellipse(center.x, center.y + 5.0, 10.0, 6.0, 0.0, body);
    draw_circle_at(center + vec2(5.0, -2.0), 5.0, body);
    draw_line(
        center.x + 2.0,
        center.y - 7.0,
        center.x + 1.0,
        center.y - 12.0,
        2.0,
        body,
    );
    draw_line(
        center.x + 7.0,
        center.y - 6.0,
        center.x + 9.0,
        center.y - 10.0,
        2.0,
        body,
    );
    draw_circle_at(
        center + vec2(7.0, -3.0),
        1.0,
        Color::new(0.05, 0.07, 0.07, 1.0),
    );
    draw_text_centered_in_box(
        &format!(
            "{} {}/{}",
            animal.name, animal.condition, animal.max_condition
        ),
        center.x - 58.0,
        center.y - 31.0,
        116.0,
        13.0,
        9.0,
        body,
    );
}

fn draw_landmark(view: &MapView, tile: TilePos, label: &str, color: Color) {
    let center = view.tile_rect(tile).center();
    draw_circle_at(
        center + vec2(0.0, -3.0),
        8.0,
        Color::new(0.08, 0.10, 0.11, 0.8),
    );
    draw_rectangle(center.x - 7.0, center.y - 10.0, 14.0, 13.0, color);
    draw_triangle(
        center + vec2(-10.0, -9.0),
        center + vec2(10.0, -9.0),
        center + vec2(0.0, -19.0),
        Color::new(0.30, 0.18, 0.16, 1.0),
    );
    draw_text_centered_in_box(
        label,
        center.x - 58.0,
        center.y - 31.0,
        116.0,
        14.0,
        10.0,
        color,
    );
}

fn draw_character(view: &MapView, tile: TilePos, color: Color, player: bool) {
    let center = view.tile_rect(tile).center();
    draw_ellipse(
        center.x,
        center.y + 9.0,
        10.0,
        4.0,
        0.0,
        Color::new(0.02, 0.04, 0.04, 0.45),
    );
    draw_circle_at(center + vec2(0.0, -4.0), 7.0, color);
    draw_rectangle(center.x - 7.0, center.y + 1.0, 14.0, 12.0, color);
    draw_circle_at(
        center + vec2(0.0, -9.0),
        5.0,
        if player {
            Color::new(0.22, 0.15, 0.12, 1.0)
        } else {
            Color::new(0.16, 0.23, 0.31, 1.0)
        },
    );
    if player {
        draw_rectangle_lines(center.x - 10.0, center.y - 18.0, 20.0, 34.0, 2.0, GOLD);
    }
}

fn draw_remote_character(view: &MapView, player: &RemotePlayer, index: usize, server_tick: u64) {
    let stale = player.stale(server_tick);
    let palette = [
        Color::new(0.56, 0.72, 0.91, 1.0),
        Color::new(0.88, 0.60, 0.76, 1.0),
        Color::new(0.69, 0.82, 0.50, 1.0),
    ];
    let color = if stale {
        Color::new(0.42, 0.45, 0.46, 0.72)
    } else {
        palette[index % palette.len()]
    };
    draw_character(view, player.position, color, false);
    let center = view.tile_rect(player.position).center();
    draw_text_centered_in_box(
        &format!(
            "{}{}",
            player.display_name,
            if stale { " (stale)" } else { "" }
        ),
        center.x - 58.0,
        center.y - 31.0,
        116.0,
        13.0,
        10.0,
        if stale { dark::TEXT_DIM } else { color },
    );
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
    draw_move_pad(content.x + 77.0, 437.0, mouse, actions);

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

fn draw_move_pad(x: f32, y: f32, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let size = 28.0;
    let gap = 4.0;
    let button = |x: f32, y: f32, label: &str, action: &mut Vec<UiAction>| {
        if virtual_button(
            Rect::new(x, y, size, size),
            label,
            true,
            ButtonTone::Secondary,
            mouse,
        ) {
            match label {
                "U" => action.push(UiAction::Move(0, -1)),
                "L" => action.push(UiAction::Move(-1, 0)),
                "R" => action.push(UiAction::Move(1, 0)),
                "D" => action.push(UiAction::Move(0, 1)),
                _ => {}
            }
        }
    };
    button(x + size + gap, y, "U", actions);
    button(x, y + size + gap, "L", actions);
    virtual_button(
        Rect::new(x + size + gap, y + size + gap, size, size),
        "•",
        false,
        ButtonTone::Secondary,
        mouse,
    );
    button(x + (size + gap) * 2.0, y + size + gap, "R", actions);
    button(x + size + gap, y + (size + gap) * 2.0, "D", actions);
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

fn draw_circle_at(center: Vec2, radius: f32, color: Color) {
    draw_circle(center.x, center.y, radius, color);
}

#[derive(Debug, Clone, Copy)]
struct MapView {
    origin: Vec2,
    tile_size: f32,
    width: usize,
    height: usize,
}

impl MapView {
    fn new(ctx: &UiContext<'_>, rect: Rect) -> Self {
        let world = &ctx.world.tiles;
        let base = (rect.w / world.width as f32).min(rect.h / world.height as f32);
        let tile_size = (base * ctx.camera_zoom).max(12.0);
        let focus = ctx.player_position;
        let origin = rect.center()
            - vec2(
                (focus.x as f32 + 0.5) * tile_size,
                (focus.y as f32 + 0.5) * tile_size,
            );
        Self {
            origin,
            tile_size,
            width: world.width,
            height: world.height,
        }
    }

    fn tile_rect(self, pos: TilePos) -> Rect {
        Rect::new(
            self.origin.x + pos.x as f32 * self.tile_size,
            self.origin.y + pos.y as f32 * self.tile_size,
            (self.tile_size - 2.0).max(6.0),
            (self.tile_size - 2.0).max(6.0),
        )
    }

    fn tile_at(self, point: Vec2) -> Option<TilePos> {
        let pos = TilePos::new(
            ((point.x - self.origin.x) / self.tile_size).floor() as i32,
            ((point.y - self.origin.y) / self.tile_size).floor() as i32,
        );
        (pos.x >= 0
            && pos.y >= 0
            && (pos.x as usize) < self.width
            && (pos.y as usize) < self.height)
            .then_some(pos)
    }
}
