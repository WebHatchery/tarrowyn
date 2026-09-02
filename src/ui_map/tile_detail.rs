//! Shape-based terrain detail used when the authored atlas is unavailable.

use super::draw_circle_at;
use crate::state::TileKind;
use macroquad::prelude::*;

pub(super) fn draw(tile: TileKind, rect: Rect) {
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
