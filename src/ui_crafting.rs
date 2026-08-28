use super::*;
use crate::network::CraftingView;

pub(super) fn draw(crafting: CraftingView, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let panel = Rect::new(196.0, 174.0, 888.0, 274.0);
    draw_surface_with_title(
        panel,
        Some("Woodworking bench"),
        &SurfaceStyle::new(PANEL)
            .with_border(2.0, GOLD)
            .with_header(44.0, Color::new(0.13, 0.15, 0.13, 1.0))
            .with_header_divider(1.0, GOLD),
        TextStyle::new(20.0, CREAM),
    );
    draw_ui_text_ex(
        "The moving mark is your moment. Tap SET QUALITY while it crosses the gold band.",
        panel.x + 32.0,
        panel.y + 78.0,
        TextStyle::new(16.0, CREAM).params(),
    );
    draw_ui_text_ex(
        "A wide target keeps one missed tap from wasting the order.",
        panel.x + 32.0,
        panel.y + 100.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    let meter = Rect::new(panel.x + 40.0, panel.y + 124.0, panel.w - 80.0, 30.0);
    draw_rectangle(
        meter.x,
        meter.y,
        meter.w,
        meter.h,
        Color::new(0.04, 0.07, 0.07, 1.0),
    );
    draw_rectangle(
        meter.x + meter.w * crafting.target_start,
        meter.y,
        meter.w * (crafting.target_end - crafting.target_start),
        meter.h,
        Color::new(0.73, 0.54, 0.20, 0.9),
    );
    let marker_x = meter.x + meter.w * crafting.progress;
    draw_rectangle(marker_x - 4.0, meter.y - 8.0, 8.0, meter.h + 16.0, MINT);
    draw_rectangle_lines(meter.x, meter.y, meter.w, meter.h, 1.0, LINE);

    if virtual_button(
        Rect::new(panel.x + 274.0, panel.y + 184.0, 340.0, 56.0),
        "SET QUALITY",
        true,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::Interact("crafting-timing".to_owned()));
    }
}
