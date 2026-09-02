//! Full-bleed presentation layer for the Tarrowyn client.

use crate::network::{ConnectionState, RemotePlayer};
use macroquad::prelude::*;
use macroquad_toolkit::grid::TilePos;
use macroquad_toolkit::prelude::*;

#[path = "ui_art.rs"]
mod ui_art;
#[path = "ui_crafting.rs"]
mod ui_crafting;
#[path = "ui_foundation.rs"]
mod ui_foundation;
#[path = "ui_hud.rs"]
mod ui_hud;
#[path = "ui_map.rs"]
mod ui_map;
#[path = "ui_online.rs"]
mod ui_online;
#[path = "ui_regional.rs"]
mod ui_regional;
pub(super) use ui_map::{draw_landmark, draw_map, MapView};

#[cfg(test)]
pub(crate) use ui_online::sidebar_modal_control_enabled;

pub const LOGICAL_WIDTH: f32 = 1280.0;
pub const LOGICAL_HEIGHT: f32 = 720.0;

#[path = "ui_context.rs"]
mod ui_context;
pub use ui_context::{UiAction, UiContext};

#[cfg(test)]
#[path = "ui/tests.rs"]
mod tests;

const CREAM: Color = Color::new(0.91, 0.87, 0.73, 1.0);
const MINT: Color = Color::new(0.50, 0.82, 0.68, 1.0);
const GOLD: Color = Color::new(0.90, 0.69, 0.30, 1.0);
const PANEL: Color = Color::new(0.055, 0.075, 0.09, 0.97);
const LINE: Color = Color::new(0.32, 0.48, 0.50, 0.75);

pub fn draw_game_ui(ctx: UiContext<'_>) -> Vec<UiAction> {
    let mouse = ctx.ui.mouse_position();
    let mut actions = Vec::new();
    let map_rect = draw_world_stage(&ctx);

    if !ctx.art_catalog_open
        && !ui_hud::blocks_map_click(mouse, ctx.menu_open)
        && !ctx.crafting.is_some()
        && !ui_online::gameplay_modal_open(&ctx)
        && ui_online::movement_enabled(&ctx)
        && is_mouse_button_down(MouseButton::Left)
    {
        if let Some(tile) = MapView::new(&ctx, map_rect).tile_at(mouse) {
            if let Some(command) =
                ui_foundation::property_touch_command(ctx.property, ctx.player_position, tile)
            {
                actions.push(UiAction::Interact(command));
            } else {
                actions.push(UiAction::MoveTo(tile));
            }
        }
    }

    if !ctx.menu_open && !ctx.art_catalog_open {
        draw_map_tooltip(&ctx, map_rect, mouse);
    }
    ui_hud::draw(&ctx, mouse, &mut actions);
    if ctx.art_catalog_open {
        ui_art::draw(&ctx, mouse, &mut actions);
    }
    ui_online::draw_account(&ctx, mouse, &mut actions);
    ui_online::draw_regional_inspection(&ctx, mouse, &mut actions);
    ui_online::draw_skill_selection(&ctx, mouse, &mut actions);
    ui_online::draw_school_selection(&ctx, mouse, &mut actions);
    ui_online::draw_chronicle(&ctx, mouse, &mut actions);
    if let Some(crafting) = ctx.crafting {
        ui_crafting::draw(crafting, mouse, &mut actions);
        actions.retain(crafting_action_allowed);
    }

    if ctx.regional_inspection.is_some() {
        actions.retain(regional_inspection_action_allowed);
    }
    if ctx.skill_selection_open {
        actions.retain(|action| {
            matches!(action, UiAction::Practice(_))
                || matches!(action, UiAction::Interact(id) if id == "skill-close")
                || is_recovery_action(action)
        });
    }
    if ctx.school_selection_open {
        actions.retain(school_selection_action_allowed);
    }
    if ctx.chronicle_open {
        actions.retain(|action| {
            matches!(action, UiAction::Interact(id) if matches!(
                id.as_str(),
                "chronicle-close" | "chronicle-search" | "chronicle-search-next"
            ) || id.starts_with("chronicle-key-")
                || is_recovery_action(action))
        });
    }
    if ctx.account_open {
        actions.retain(|action| {
            matches!(action, UiAction::Interact(id) if id == "account-close")
                || is_recovery_action(action)
        });
    }
    if ctx.art_catalog_open {
        actions.retain(|action| {
            matches!(
                action,
                UiAction::Interact(id)
                    if matches!(
                        id.as_str(),
                        "art-catalog-close" | "art-page-prev" | "art-page-next"
                    )
            )
        });
    }

    actions
}

fn draw_world_stage(ctx: &UiContext<'_>) -> Rect {
    let rect = Rect::new(0.0, 0.0, LOGICAL_WIDTH, LOGICAL_HEIGHT);
    draw_map(ctx, rect);

    // A quiet veil gives the floating HUD enough contrast without fencing the
    // world into another card or adding a decorative frame around the screen.
    draw_rectangle(
        0.0,
        0.0,
        LOGICAL_WIDTH,
        58.0,
        Color::new(0.015, 0.025, 0.028, 0.16),
    );
    rect
}

fn draw_map_tooltip(ctx: &UiContext<'_>, map_rect: Rect, mouse: Vec2) {
    if !map_rect.contains_point(mouse) || ui_hud::blocks_map_click(mouse, false) {
        return;
    }
    draw_tooltip(
        ui_online::movement_tooltip_for_overlay(
            ui_online::gameplay_modal_open(ctx),
            ui_online::movement_tooltip(ctx),
        ),
        mouse,
    );
}

fn is_recovery_action(action: &UiAction) -> bool {
    matches!(action, UiAction::Reconnect)
        || matches!(
            action,
            UiAction::Interact(id)
                if matches!(id.as_str(), "recover-self" | "recover" | "recover-healer")
        )
}

fn school_selection_action_allowed(action: &UiAction) -> bool {
    matches!(action, UiAction::Teach(_))
        || matches!(action, UiAction::Interact(id) if id == "school-close")
        || is_recovery_action(action)
}

fn regional_inspection_action_allowed(action: &UiAction) -> bool {
    matches!(action, UiAction::RegionalEvent(_))
        || matches!(
            action,
            UiAction::Interact(id)
                if matches!(
                    id.as_str(),
                    "region-details" | "route-repair" | "route-escort" | "route-improve"
                )
        )
        || is_recovery_action(action)
}

fn crafting_action_allowed(action: &UiAction) -> bool {
    matches!(action, UiAction::Interact(id) if id == "crafting-timing")
        || is_recovery_action(action)
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
        &SurfaceStyle::new(fill).with_top_highlight(
            1.0,
            if enabled {
                style.border
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            },
        ),
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

fn format_clock(minutes: u32) -> String {
    format!("{:02}:{:02}", (minutes / 60) % 24, minutes % 60)
}

#[cfg(test)]
fn online_footer_detail(stats: &str, visible_players: usize, trade: Option<&str>) -> String {
    let mut lines = stats.lines();
    let overview = lines.next().unwrap_or("Player ledger loading");
    let inventory = lines.nth(2).unwrap_or("Inventory loading");
    let mut detail = format!("{overview} • {visible_players} players\n{inventory}");
    if let Some(trade) = trade {
        detail.push('\n');
        detail.push_str(trade);
    }
    detail
}

#[cfg(test)]
fn pending_trade_detail(
    trades: &[tarrowyn_protocol::TradeOffer],
    own_account_id: Option<&str>,
) -> Option<String> {
    let own_account_id = own_account_id?;
    let trade = trades.iter().find(|trade| {
        trade.status == tarrowyn_protocol::TradeStatus::Pending
            && (trade.creator_account_id == own_account_id
                || trade.recipient_account_id == own_account_id)
    })?;
    let (direction, other_name, offered, requested) =
        if trade.recipient_account_id == own_account_id {
            (
                "from",
                trade.creator_name.as_str(),
                trade.offer,
                trade.request,
            )
        } else {
            (
                "to",
                trade.recipient_name.as_str(),
                trade.offer,
                trade.request,
            )
        };
    Some(format!(
        "Trade {direction} {other_name}: {} for {}",
        trade_bundle_detail(offered),
        trade_bundle_detail(requested)
    ))
}

#[cfg(test)]
fn trade_bundle_detail(bundle: tarrowyn_protocol::TradeBundle) -> String {
    let mut goods = Vec::new();
    for (amount, name) in [
        (bundle.wheat, "wheat"),
        (bundle.turnips, "turnips"),
        (bundle.moonberries, "moonberries"),
        (bundle.seeds, "seeds"),
        (bundle.timber, "timber"),
        (bundle.stone, "stone"),
        (bundle.iron_ore, "iron ore"),
        (bundle.charcoal, "charcoal"),
        (bundle.tool_handles, "tool handles"),
        (bundle.gold, "gold"),
    ] {
        if amount > 0 {
            goods.push(format!("{amount} {name}"));
        }
    }
    if goods.is_empty() {
        "nothing".to_owned()
    } else {
        goods.join(", ")
    }
}
