//! The Years of Tarrowyn — Phase 6 release-candidate client entry point.

use macroquad::prelude::*;
use macroquad_toolkit::capture;
use macroquad_toolkit::prelude::dark;

mod data;
mod game;
mod network;
mod state;
mod ui;

use game::Game;

const UI_FONT_SIZES: &[u16] = &[8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 20, 28];

fn window_conf() -> Conf {
    capture::capture_window_conf(
        "TARROWYN",
        "The Years of Tarrowyn — Phase 6",
        ui::LOGICAL_WIDTH as i32,
        ui::LOGICAL_HEIGHT as i32,
    )
}

#[macroquad::main(window_conf)]
async fn main() {
    macroquad_toolkit::ui::prewarm_default_ui_font(UI_FONT_SIZES)
        .expect("toolkit UI font should prewarm");
    clear_background(dark::BACKGROUND);
    macroquad_toolkit::ui::draw_default_ui_font_atlas_warmup(UI_FONT_SIZES);
    next_frame().await;

    let mut game = Game::new().await;

    if let Some(configs) = capture::CaptureConfig::all_from_env("TARROWYN") {
        for config in configs {
            capture::run_capture_once(&config, |dt| {
                game.update(dt);
                game.draw();
            })
            .await;
        }
        return;
    }

    loop {
        let dt = get_frame_time().min(0.1);
        game.update(dt);
        game.draw();
        next_frame().await;
    }
}
