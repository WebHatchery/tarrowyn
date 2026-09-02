use super::*;
use crate::sprites::SpriteAssets;
use tarrowyn_protocol::{LocationKind, RouteStatus};

pub(super) fn draw_map_overlay(ctx: &UiContext<'_>, view: &MapView, rect: Rect) {
    if !ctx.foundation.landmarks.is_empty() {
        draw_expedition_outpost(ctx, view, rect);
        return;
    }
    let Some(region) = ctx.regional_region else {
        draw_local_landmarks(ctx.sprites, view);
        draw_expedition_outpost(ctx, view, rect);
        return;
    };

    for route in &region.routes {
        let Some(origin) = region
            .locations
            .iter()
            .find(|location| location.location_id == route.origin_location_id)
        else {
            continue;
        };
        let Some(destination) = region
            .locations
            .iter()
            .find(|location| location.location_id == route.destination_location_id)
        else {
            continue;
        };
        let start = view
            .tile_rect(TilePos::new(origin.position.x, origin.position.y))
            .center();
        let end = view
            .tile_rect(TilePos::new(destination.position.x, destination.position.y))
            .center();
        if rect.contains_point(start) || rect.contains_point(end) {
            draw_line(
                start.x,
                start.y,
                end.x,
                end.y,
                3.0,
                route_color(route.status),
            );
        }
    }

    let founded_position = successful_expedition(ctx).map(|expedition| expedition.outpost_position);
    for location in &region.locations {
        if founded_position == Some(location.position) {
            continue;
        }
        let tile = TilePos::new(location.position.x, location.position.y);
        if rect.overlaps(&view.tile_rect(tile)) {
            draw_landmark(
                ctx.sprites,
                view,
                tile,
                &location.name,
                location_color(location.kind),
            );
        }
    }
    draw_expedition_outpost(ctx, view, rect);
}

fn successful_expedition<'a>(ctx: &UiContext<'a>) -> Option<&'a tarrowyn_protocol::Expedition> {
    ctx.expedition
        .filter(|expedition| expedition.status == tarrowyn_protocol::ExpeditionStatus::Succeeded)
}

fn draw_expedition_outpost(ctx: &UiContext<'_>, view: &MapView, rect: Rect) {
    let Some(expedition) = successful_expedition(ctx) else {
        return;
    };
    let tile = TilePos::new(expedition.outpost_position.x, expedition.outpost_position.y);
    if rect.overlaps(&view.tile_rect(tile)) {
        draw_landmark(
            ctx.sprites,
            view,
            tile,
            &expedition.outpost_name,
            Color::new(0.82, 0.68, 0.32, 1.0),
        );
    }
}

fn draw_local_landmarks(sprites: &SpriteAssets, view: &MapView) {
    draw_landmark(
        sprites,
        view,
        TilePos::new(8, 5),
        "THE HEARTH",
        Color::new(0.76, 0.46, 0.25, 1.0),
    );
    draw_landmark(
        sprites,
        view,
        TilePos::new(4, 4),
        "SHARED FIELDS",
        Color::new(0.78, 0.69, 0.30, 1.0),
    );
    draw_landmark(
        sprites,
        view,
        TilePos::new(14, 3),
        "WHISPERWOOD",
        Color::new(0.45, 0.78, 0.58, 1.0),
    );
}

fn route_color(status: RouteStatus) -> Color {
    match status {
        RouteStatus::Operational => Color::new(0.34, 0.80, 0.60, 0.90),
        RouteStatus::Delayed => Color::new(0.88, 0.70, 0.28, 0.90),
        RouteStatus::Threatened => Color::new(0.90, 0.38, 0.28, 0.90),
        RouteStatus::Repairing => Color::new(0.50, 0.67, 0.90, 0.90),
        RouteStatus::Closed => Color::new(0.38, 0.40, 0.42, 0.72),
    }
}

fn location_color(kind: LocationKind) -> Color {
    match kind {
        LocationKind::Settlement => Color::new(0.76, 0.46, 0.25, 1.0),
        LocationKind::Outpost => Color::new(0.45, 0.78, 0.58, 1.0),
        LocationKind::Frontier => Color::new(0.48, 0.68, 0.88, 1.0),
    }
}
