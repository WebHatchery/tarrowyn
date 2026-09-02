use super::*;
use tarrowyn_protocol::FoundationStorehouseStage;

pub(super) fn draw(ctx: &UiContext<'_>, center: Vec2, rect: Rect) {
    let scale = rect.w.max(18.0);
    let stage = ctx.foundation_activity.storehouse.current_stage;
    if stage == FoundationStorehouseStage::SiteMarked {
        super::draw_foundation_icon(ctx, "construction_space", center, rect);
        return;
    }
    draw_rectangle(
        center.x - scale * 0.40,
        center.y + scale * 0.12,
        scale * 0.80,
        scale * 0.18,
        Color::new(0.46, 0.49, 0.48, 1.0),
    );
    for offset in [-0.30, -0.10, 0.10, 0.30] {
        draw_rectangle(
            center.x + scale * offset - 2.0,
            center.y + scale * 0.10,
            4.0,
            scale * 0.22,
            Color::new(0.68, 0.70, 0.66, 1.0),
        );
    }
    if stage == FoundationStorehouseStage::FoundationLaid {
        return;
    }
    let timber = Color::new(0.47, 0.29, 0.14, 1.0);
    for offset in [-0.32, 0.32] {
        draw_rectangle(
            center.x + scale * offset - scale * 0.05,
            center.y - scale * 0.34,
            scale * 0.10,
            scale * 0.55,
            timber,
        );
    }
    draw_rectangle(
        center.x - scale * 0.37,
        center.y - scale * 0.36,
        scale * 0.74,
        scale * 0.10,
        timber,
    );
    if stage == FoundationStorehouseStage::FrameRaised {
        draw_line(
            center.x - scale * 0.32,
            center.y - scale * 0.30,
            center.x + scale * 0.32,
            center.y + scale * 0.14,
            3.0,
            timber,
        );
        return;
    }
    draw_rectangle(
        center.x - scale * 0.32,
        center.y - scale * 0.27,
        scale * 0.64,
        scale * 0.48,
        Color::new(0.62, 0.43, 0.22, 1.0),
    );
    draw_triangle(
        center + vec2(-scale * 0.43, -scale * 0.26),
        center + vec2(scale * 0.43, -scale * 0.26),
        center + vec2(0.0, -scale * 0.58),
        Color::new(0.25, 0.18, 0.16, 1.0),
    );
    draw_rectangle(
        center.x - scale * 0.10,
        center.y - scale * 0.04,
        scale * 0.20,
        scale * 0.25,
        Color::new(0.15, 0.11, 0.10, 1.0),
    );
    draw_rectangle_lines(
        center.x - scale * 0.32,
        center.y - scale * 0.27,
        scale * 0.64,
        scale * 0.48,
        2.0,
        GOLD,
    );
}
