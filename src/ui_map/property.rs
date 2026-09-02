use super::*;
use tarrowyn_protocol::{FoundationPropertyStage, FoundationPropertySummary};

pub(super) fn draw(ctx: &UiContext<'_>, view: &MapView, map_rect: Rect) {
    for shelter in &ctx.property.properties {
        let Some(stage) = ctx
            .property
            .contract
            .stages
            .iter()
            .find(|stage| stage.stage == shelter.stage)
        else {
            continue;
        };
        let owned = ctx.own_account_id == Some(shelter.owner_account_id.as_str());
        for dy in 0..stage.footprint.height {
            for dx in 0..stage.footprint.width {
                let tile = TilePos::new(
                    shelter.anchor.x + i32::from(dx),
                    shelter.anchor.y + i32::from(dy),
                );
                let rect = view.tile_rect(tile);
                if map_rect.overlaps(&rect) {
                    draw_shelter_tile(rect, shelter.stage, owned, dx, dy);
                }
            }
        }
        let anchor = view.tile_rect(TilePos::new(shelter.anchor.x, shelter.anchor.y));
        if map_rect.overlaps(&anchor) {
            draw_label(anchor, shelter, owned);
        }
    }
    if let Some(preview) = &ctx.property.placement_preview {
        let color = if preview.accepted { MINT } else { RED };
        for dy in 0..preview.footprint.height {
            for dx in 0..preview.footprint.width {
                let rect = view.tile_rect(TilePos::new(
                    preview.anchor.x + i32::from(dx),
                    preview.anchor.y + i32::from(dy),
                ));
                draw_rectangle_lines(
                    rect.x + 2.0,
                    rect.y + 2.0,
                    rect.w - 4.0,
                    rect.h - 4.0,
                    2.0,
                    color,
                );
            }
        }
    }
}

fn draw_shelter_tile(rect: Rect, stage: FoundationPropertyStage, owned: bool, dx: u8, dy: u8) {
    let (wall, roof) = match stage {
        FoundationPropertyStage::Tent => (
            Color::new(0.55, 0.42, 0.25, 1.0),
            Color::new(0.78, 0.67, 0.43, 1.0),
        ),
        FoundationPropertyStage::Camp => (
            Color::new(0.39, 0.28, 0.16, 1.0),
            Color::new(0.66, 0.48, 0.27, 1.0),
        ),
        FoundationPropertyStage::House => (
            Color::new(0.46, 0.31, 0.18, 1.0),
            Color::new(0.30, 0.18, 0.12, 1.0),
        ),
    };
    draw_rectangle(
        rect.x + 2.0,
        rect.y + rect.h * 0.34,
        rect.w - 4.0,
        rect.h * 0.58,
        wall,
    );
    draw_triangle(
        vec2(rect.x, rect.y + rect.h * 0.4),
        vec2(rect.right(), rect.y + rect.h * 0.4),
        vec2(rect.center().x, rect.y + 1.0),
        roof,
    );
    if dx == 0 && dy == 0 {
        draw_rectangle(
            rect.center().x - 2.0,
            rect.bottom() - rect.h * 0.28,
            4.0,
            rect.h * 0.28,
            Color::new(0.10, 0.08, 0.06, 1.0),
        );
    }
    if owned {
        draw_rectangle_lines(
            rect.x + 1.0,
            rect.y + 1.0,
            rect.w - 2.0,
            rect.h - 2.0,
            1.5,
            GOLD,
        );
    }
}

fn draw_label(rect: Rect, shelter: &FoundationPropertySummary, owned: bool) {
    let stage = match shelter.stage {
        FoundationPropertyStage::Tent => "TENT",
        FoundationPropertyStage::Camp => "CAMP",
        FoundationPropertyStage::House => "HOUSE",
    };
    let label = if owned {
        format!("YOUR {stage}")
    } else {
        format!("{}'S {stage}", shelter.owner_name.to_ascii_uppercase())
    };
    draw_text_centered_in_box(
        &label,
        rect.x - rect.w,
        rect.y - 15.0,
        rect.w * 3.0,
        12.0,
        8.0,
        if owned { GOLD } else { CREAM },
    );
}
