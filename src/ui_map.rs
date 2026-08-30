use super::*;

pub(crate) fn draw_map(ctx: &UiContext<'_>, rect: Rect) {
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

    ui_regional::draw_map_overlay(ctx, &view, rect);
    if should_draw_player_marker(ctx.offline, ctx.player_position_authoritative) {
        draw_character(&view, ctx.player_position, CREAM, true);
    }
    for (index, player) in ctx.remote_players.iter().enumerate() {
        if ctx.own_account_id == Some(player.account_id.as_str()) {
            continue;
        }
        draw_remote_character(&view, player, index, ctx.server_tick);
    }
}

pub(crate) fn should_draw_player_marker(offline: bool, authoritative: bool) -> bool {
    offline || authoritative
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

pub(crate) fn draw_landmark(view: &MapView, tile: TilePos, label: &str, color: Color) {
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

fn draw_circle_at(center: Vec2, radius: f32, color: Color) {
    draw_circle(center.x, center.y, radius, color);
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MapView {
    origin: Vec2,
    tile_size: f32,
    width: usize,
    height: usize,
}

impl MapView {
    pub(crate) fn new(ctx: &UiContext<'_>, rect: Rect) -> Self {
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

    pub(crate) fn tile_rect(self, pos: TilePos) -> Rect {
        Rect::new(
            self.origin.x + pos.x as f32 * self.tile_size,
            self.origin.y + pos.y as f32 * self.tile_size,
            (self.tile_size - 2.0).max(6.0),
            (self.tile_size - 2.0).max(6.0),
        )
    }

    pub(crate) fn tile_at(self, point: Vec2) -> Option<TilePos> {
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
