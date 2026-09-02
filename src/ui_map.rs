use super::*;
use crate::sprites::{ArtAtlas, FoundationSprite, ItemSprite, NpcSprite, SpriteAssets};
use crate::state::{tile_color, CropState, TileKind};

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
        draw_rectangle(
            tile_rect.x,
            tile_rect.y,
            tile_rect.w,
            tile_rect.h,
            tile_color(*tile),
        );
        let textured = ctx.sprites.draw_terrain_tile(
            *tile,
            pos,
            tile_rect.center(),
            vec2(tile_rect.w + 1.0, tile_rect.h + 1.0),
            ctx.night,
        );
        if !textured {
            draw_tile_detail(*tile, tile_rect);
        }
        if !textured {
            draw_rectangle_lines(
                tile_rect.x,
                tile_rect.y,
                tile_rect.w,
                tile_rect.h,
                0.45,
                Color::new(0.04, 0.09, 0.09, 0.20),
            );
        }
        if let Some(Some(crop)) = ctx.world.crops.get(pos) {
            draw_crop(ctx, crop, tile_rect);
        }
    }
    for animal in ctx.farm_animals {
        let tile = TilePos::new(animal.position.x, animal.position.y);
        let tile_rect = view.tile_rect(tile);
        if rect.overlaps(&tile_rect) {
            draw_farm_animal(ctx.sprites, animal, tile_rect);
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
    if ctx.foundation.landmarks.is_empty() {
        draw_map_item_markers(ctx, &view, rect);
        draw_fixed_npcs(ctx, &view, rect);
    } else {
        draw_foundation_landmarks(ctx, &view, rect);
    }
    if ctx.foundation.landmarks.is_empty() {
        draw_wilderness_monster(ctx, &view, rect);
    }
    if should_draw_player_marker(ctx.player_position_authoritative) {
        draw_character_at_position(
            ctx.sprites,
            &view,
            ctx.rendered_player_position,
            CREAM,
            true,
        );
    }
    for (index, player) in ctx.remote_players.iter().enumerate() {
        if ctx.own_account_id == Some(player.account_id.as_str()) {
            continue;
        }
        draw_remote_character(ctx.sprites, &view, player, index, ctx.server_tick);
    }
}

fn draw_foundation_landmarks(ctx: &UiContext<'_>, view: &MapView, rect: Rect) {
    let context_id = super::ui_foundation::nearby_context(ctx.foundation, ctx.player_position)
        .map(|context| context.landmark.id.as_str());
    for landmark in ctx
        .foundation
        .landmarks
        .iter()
        .filter(|landmark| landmark.visible)
    {
        let tile = TilePos::new(landmark.position.x, landmark.position.y);
        let tile_rect = view.tile_rect(tile);
        if !rect.overlaps(&tile_rect) {
            continue;
        }
        let center = tile_rect.center();
        draw_ellipse(
            center.x,
            tile_rect.bottom() - 2.0,
            tile_rect.w * 0.34,
            tile_rect.h * 0.11,
            0.0,
            Color::new(0.01, 0.02, 0.02, 0.48),
        );
        draw_foundation_icon(ctx, landmark.kind.as_str(), center, tile_rect);
        let contextual = context_id == Some(landmark.id.as_str());
        let anchor_label = match landmark.kind.as_str() {
            "npc" => Some("MARA"),
            "noticeboard" => Some("NEEDS"),
            _ => None,
        };
        if contextual || anchor_label.is_some() {
            let label = anchor_label.unwrap_or(&landmark.name);
            draw_text_centered_in_box(
                &label.to_ascii_uppercase(),
                center.x - tile_rect.w * 1.35,
                center.y - tile_rect.h * 1.0,
                tile_rect.w * 2.7,
                12.0,
                8.0,
                if contextual { GOLD } else { CREAM },
            );
        }
    }
}

fn draw_foundation_icon(ctx: &UiContext<'_>, kind: &str, center: Vec2, rect: Rect) {
    let scale = rect.w.max(18.0);
    if FoundationSprite::from_kind(kind)
        .is_some_and(|sprite| ctx.sprites.draw_foundation(sprite, center, scale))
    {
        return;
    }
    match kind {
        "npc" => {
            if !ctx.sprites.draw_npc(
                NpcSprite::Iven,
                center + vec2(0.0, -rect.h * 0.22),
                vec2(scale * 0.9, scale * 1.25),
            ) {
                draw_circle_at(center + vec2(0.0, -5.0), scale * 0.18, MINT);
                draw_rectangle(
                    center.x - scale * 0.16,
                    center.y,
                    scale * 0.32,
                    scale * 0.38,
                    MINT,
                );
            }
        }
        "beacon" => {
            draw_circle_at(center + vec2(0.0, -scale * 0.28), scale * 0.22, GOLD);
            draw_circle_lines(center.x, center.y - scale * 0.28, scale * 0.34, 2.0, CREAM);
            draw_rectangle(center.x - 3.0, center.y - 2.0, 6.0, scale * 0.42, CREAM);
        }
        "tent_settlement" => {
            for offset in [-0.24, 0.18] {
                let x = center.x + scale * offset;
                draw_triangle(
                    vec2(x - scale * 0.25, center.y + scale * 0.22),
                    vec2(x + scale * 0.25, center.y + scale * 0.22),
                    vec2(x, center.y - scale * 0.32),
                    Color::new(0.82, 0.70, 0.49, 1.0),
                );
            }
        }
        "gathering_place" => {
            draw_triangle(
                center + vec2(-scale * 0.22, scale * 0.2),
                center + vec2(scale * 0.22, scale * 0.2),
                center + vec2(0.0, -scale * 0.34),
                Color::new(0.96, 0.43, 0.16, 1.0),
            );
            draw_circle_at(center + vec2(0.0, -scale * 0.05), scale * 0.12, GOLD);
        }
        "noticeboard" => {
            draw_rectangle(
                center.x - scale * 0.3,
                center.y - scale * 0.32,
                scale * 0.6,
                scale * 0.46,
                Color::new(0.45, 0.28, 0.14, 1.0),
            );
            draw_rectangle(
                center.x - 2.0,
                center.y + scale * 0.1,
                4.0,
                scale * 0.38,
                Color::new(0.35, 0.22, 0.12, 1.0),
            );
            draw_rectangle(
                center.x - scale * 0.2,
                center.y - scale * 0.23,
                scale * 0.4,
                scale * 0.22,
                CREAM,
            );
        }
        "rough_forge" => {
            draw_rectangle(
                center.x - scale * 0.32,
                center.y - scale * 0.05,
                scale * 0.64,
                scale * 0.32,
                Color::new(0.18, 0.20, 0.21, 1.0),
            );
            draw_circle_at(
                center + vec2(scale * 0.13, -scale * 0.1),
                scale * 0.13,
                Color::new(0.94, 0.34, 0.12, 1.0),
            );
        }
        "construction_space" => {
            draw_rectangle_lines(
                center.x - scale * 0.36,
                center.y - scale * 0.25,
                scale * 0.72,
                scale * 0.55,
                2.0,
                GOLD,
            );
            draw_line(
                center.x - scale * 0.3,
                center.y + scale * 0.2,
                center.x + scale * 0.3,
                center.y - scale * 0.18,
                2.0,
                GOLD,
            );
        }
        "shared_storage" | "crude_tools" => {
            draw_rectangle(
                center.x - scale * 0.3,
                center.y - scale * 0.18,
                scale * 0.6,
                scale * 0.42,
                Color::new(0.46, 0.31, 0.16, 1.0),
            );
            draw_rectangle_lines(
                center.x - scale * 0.3,
                center.y - scale * 0.18,
                scale * 0.6,
                scale * 0.42,
                2.0,
                CREAM,
            );
        }
        "farmland" => {
            draw_rectangle(
                center.x - scale * 0.4,
                center.y - scale * 0.25,
                scale * 0.8,
                scale * 0.5,
                Color::new(0.55, 0.38, 0.16, 1.0),
            );
            for offset in [-0.2, 0.0, 0.2] {
                draw_line(
                    center.x + scale * offset,
                    center.y - scale * 0.2,
                    center.x + scale * offset,
                    center.y + scale * 0.2,
                    1.0,
                    GOLD,
                );
            }
        }
        "woodland" => {
            draw_circle_at(center + vec2(0.0, -scale * 0.12), scale * 0.34, MINT);
            draw_rectangle(
                center.x - 3.0,
                center.y + scale * 0.05,
                6.0,
                scale * 0.34,
                Color::new(0.38, 0.24, 0.12, 1.0),
            );
        }
        "mineable_ground" => {
            draw_triangle(
                center + vec2(-scale * 0.38, scale * 0.24),
                center + vec2(scale * 0.38, scale * 0.24),
                center + vec2(0.0, -scale * 0.34),
                Color::new(0.46, 0.49, 0.50, 1.0),
            );
        }
        _ => draw_circle_at(center, scale * 0.24, MINT),
    }
}

pub(crate) fn should_draw_player_marker(authoritative: bool) -> bool {
    authoritative
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

fn draw_crop(ctx: &UiContext<'_>, crop: &CropState, rect: Rect) {
    if ctx.sprites.draw_crop(
        crop.kind,
        crop.stage,
        rect.center() + vec2(0.0, 1.0),
        vec2(rect.w * 0.88, rect.h * 0.88),
    ) {
        return;
    }
    let sprite = match crop.kind {
        crate::state::CropKind::Wheat => ItemSprite::Wheat,
        crate::state::CropKind::Turnip => ItemSprite::Turnips,
        crate::state::CropKind::Moonberry => ItemSprite::Moonberries,
    };
    if ctx.sprites.draw_item(
        sprite,
        rect.center() + vec2(0.0, 1.0),
        vec2(rect.w * 0.82, rect.h * 0.82),
    ) {
        return;
    }
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

fn draw_map_item_markers(ctx: &UiContext<'_>, view: &MapView, rect: Rect) {
    let markers = [
        (TilePos::new(6, 5), ItemSprite::Seeds),
        (TilePos::new(8, 4), ItemSprite::Bandages),
        (TilePos::new(10, 3), ItemSprite::Stone),
        (TilePos::new(14, 3), ItemSprite::Timber),
    ];
    for (tile, sprite) in markers {
        let tile_rect = view.tile_rect(tile);
        if !rect.overlaps(&tile_rect) {
            continue;
        }
        let center = tile_rect.center() + vec2(0.0, -tile_rect.h * 0.12);
        draw_ellipse(
            center.x,
            tile_rect.bottom() - 3.0,
            tile_rect.w * 0.28,
            tile_rect.h * 0.10,
            0.0,
            Color::new(0.02, 0.04, 0.04, 0.38),
        );
        ctx.sprites
            .draw_item(sprite, center, vec2(tile_rect.w * 0.62, tile_rect.h * 0.62));
    }
}

fn draw_fixed_npcs(ctx: &UiContext<'_>, view: &MapView, rect: Rect) {
    let npcs = [
        (TilePos::new(7, 5), NpcSprite::Iven, "IVEN"),
        (TilePos::new(9, 5), NpcSprite::Sella, "SELLA"),
    ];
    for (tile, sprite, label) in npcs {
        let tile_rect = view.tile_rect(tile);
        if !rect.overlaps(&tile_rect) {
            continue;
        }
        let center = tile_rect.center() + vec2(0.0, -tile_rect.h * 0.28);
        draw_ellipse(
            center.x,
            tile_rect.bottom() - 2.0,
            tile_rect.w * 0.28,
            tile_rect.h * 0.10,
            0.0,
            Color::new(0.02, 0.04, 0.04, 0.42),
        );
        if !ctx
            .sprites
            .draw_npc(sprite, center, vec2(tile_rect.w * 0.92, tile_rect.h * 1.35))
        {
            draw_character(ctx.sprites, view, tile, MINT, false);
        }
        draw_text_centered_in_box(
            label,
            center.x - tile_rect.w,
            center.y - tile_rect.h * 0.95,
            tile_rect.w * 2.0,
            12.0,
            8.0,
            CREAM,
        );
    }
}

fn draw_wilderness_monster(ctx: &UiContext<'_>, view: &MapView, rect: Rect) {
    let Some(zone) = ctx.wilderness.filter(|zone| zone.threat_active) else {
        return;
    };
    let tile = TilePos::new(zone.position.x, zone.position.y);
    let tile_rect = view.tile_rect(tile);
    if !rect.overlaps(&tile_rect) {
        return;
    }
    let center = tile_rect.center() + vec2(0.0, -tile_rect.h * 0.18);
    draw_ellipse(
        center.x,
        tile_rect.bottom() - 2.0,
        tile_rect.w * 0.42,
        tile_rect.h * 0.14,
        0.0,
        Color::new(0.01, 0.02, 0.02, 0.55),
    );
    if !ctx
        .sprites
        .draw_monster(center, vec2(tile_rect.w * 1.65, tile_rect.h * 1.65))
    {
        draw_circle_at(
            center,
            tile_rect.w * 0.44,
            Color::new(0.32, 0.16, 0.13, 1.0),
        );
    }
    draw_text_centered_in_box(
        &format!("BRAMBLEBACK • {}/3", zone.monster_health),
        center.x - tile_rect.w * 2.0,
        center.y - tile_rect.h * 1.2,
        tile_rect.w * 4.0,
        13.0,
        9.0,
        Color::new(0.94, 0.66, 0.42, 1.0),
    );
}

fn draw_farm_animal(sprites: &SpriteAssets, animal: &tarrowyn_protocol::FarmAnimal, rect: Rect) {
    let center = rect.center();
    let condition_ratio = animal.condition as f32 / animal.max_condition.max(1) as f32;
    let goat_pose = if condition_ratio >= 0.8 {
        34
    } else if condition_ratio >= 0.4 {
        33
    } else {
        32
    };
    if sprites.draw_atlas_cell(
        ArtAtlas::Farming,
        goat_pose,
        center,
        vec2(rect.w * 0.86, rect.h * 0.86),
        WHITE,
    ) {
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
            Color::new(0.92, 0.86, 0.68, 1.0),
        );
        return;
    }
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

pub(crate) fn draw_landmark(
    sprites: &SpriteAssets,
    view: &MapView,
    tile: TilePos,
    label: &str,
    color: Color,
) {
    let center = view.tile_rect(tile).center();
    draw_circle_at(
        center + vec2(0.0, -3.0),
        8.0,
        Color::new(0.08, 0.10, 0.11, 0.8),
    );
    let label_lower = label.to_ascii_lowercase();
    let settlement_cell = if label_lower.contains("hearth") {
        0
    } else if label_lower.contains("whisperwood") || label_lower.contains("watch") {
        8
    } else if label_lower.contains("saltmere") || label_lower.contains("ferry") {
        16
    } else if label_lower.contains("field") {
        6
    } else {
        24
    };
    if !sprites.draw_atlas_cell(
        ArtAtlas::Settlements,
        settlement_cell,
        center + vec2(0.0, -4.0),
        vec2(view.tile_size * 1.45, view.tile_size * 1.45),
        WHITE,
    ) {
        draw_rectangle(center.x - 7.0, center.y - 10.0, 14.0, 13.0, color);
        draw_triangle(
            center + vec2(-10.0, -9.0),
            center + vec2(10.0, -9.0),
            center + vec2(0.0, -19.0),
            Color::new(0.30, 0.18, 0.16, 1.0),
        );
    }
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

fn draw_character(
    sprites: &SpriteAssets,
    view: &MapView,
    tile: TilePos,
    color: Color,
    player: bool,
) {
    draw_character_at_position(
        sprites,
        view,
        vec2(tile.x as f32, tile.y as f32),
        color,
        player,
    );
}

fn draw_character_at_position(
    sprites: &SpriteAssets,
    view: &MapView,
    position: Vec2,
    color: Color,
    player: bool,
) {
    let center = view.world_position_center(position);
    let scale = (view.tile_size / 34.0).clamp(0.8, 1.8);
    draw_ellipse(
        center.x,
        center.y + 9.0 * scale,
        10.0 * scale,
        4.0 * scale,
        0.0,
        Color::new(0.02, 0.04, 0.04, 0.45),
    );
    let player_cell = if player { 48 } else { 0 };
    if sprites.draw_atlas_cell(
        ArtAtlas::Player,
        player_cell,
        center + vec2(0.0, -2.0 * scale),
        vec2(26.0 * scale, 34.0 * scale),
        if player { WHITE } else { color },
    ) {
        if player {
            draw_rectangle_lines(
                center.x - 12.0 * scale,
                center.y - 19.0 * scale,
                24.0 * scale,
                38.0 * scale,
                2.0,
                GOLD,
            );
        }
        return;
    }
    draw_circle_at(center + vec2(0.0, -4.0 * scale), 7.0 * scale, color);
    draw_rectangle(
        center.x - 7.0 * scale,
        center.y + 1.0 * scale,
        14.0 * scale,
        12.0 * scale,
        color,
    );
    draw_circle_at(
        center + vec2(0.0, -9.0 * scale),
        5.0 * scale,
        if player {
            Color::new(0.22, 0.15, 0.12, 1.0)
        } else {
            Color::new(0.16, 0.23, 0.31, 1.0)
        },
    );
    if player {
        draw_rectangle_lines(
            center.x - 10.0 * scale,
            center.y - 18.0 * scale,
            20.0 * scale,
            34.0 * scale,
            2.0,
            GOLD,
        );
    }
}

fn draw_remote_character(
    sprites: &SpriteAssets,
    view: &MapView,
    player: &RemotePlayer,
    index: usize,
    server_tick: u64,
) {
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
    draw_character(sprites, view, player.position, color, false);
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
        let focus = ctx.rendered_player_position;
        let origin = rect.center() - vec2((focus.x + 0.5) * tile_size, (focus.y + 0.5) * tile_size);
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
            (self.tile_size - 0.8).max(6.0),
            (self.tile_size - 0.8).max(6.0),
        )
    }

    fn world_position_center(self, position: Vec2) -> Vec2 {
        self.origin
            + vec2(
                (position.x + 0.5) * self.tile_size - 0.4,
                (position.y + 0.5) * self.tile_size - 0.4,
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
