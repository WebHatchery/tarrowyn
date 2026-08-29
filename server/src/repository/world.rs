//! Shared map construction, crop growth, and field environmental pressure.

use super::models::RepositoryState;
use crate::config::ServerConfig;
use tarrowyn_protocol::{
    CropState, FarmPlot, FarmingAction, FieldWeather, Position, TileKind, WorldEvent, WorldTile,
};

pub(super) fn farming_notice(action: FarmingAction) -> &'static str {
    match action {
        FarmingAction::Plant => "A new promise is planted in the shared fields.",
        FarmingAction::Tend => "Someone has tended the fields; the next harvest looks steadier.",
        FarmingAction::Harvest => "A fresh crop reaches the Hearth's stores.",
        FarmingAction::TendAnimal => "Bellweather the goat is cared for beside the shared fields.",
    }
}

pub(super) fn field_weather_for_day(day: u32) -> FieldWeather {
    match day % 5 {
        0 => FieldWeather::HeavyRain,
        3 => FieldWeather::DryWind,
        _ => FieldWeather::Clear,
    }
}

pub(super) fn field_pest_pressure_for_day(day: u32) -> u8 {
    match day % 4 {
        0 => 2,
        2 => 1,
        _ => 0,
    }
}

pub(super) fn grow_plots(state: &mut RepositoryState, config: &ServerConfig) {
    let weather_pressure = field_weather_for_day(state.clock.day).pressure();
    let pest_pressure = field_pest_pressure_for_day(state.clock.day);
    let environmental_pressure = weather_pressure.saturating_add(pest_pressure).min(2);
    let mut changed = Vec::new();
    for plot in &mut state.plots {
        let Some(mut crop) = plot.crop else { continue };
        let age = state.tick.saturating_sub(crop.planted_tick) as f32
            * config.world_seconds_per_tick.max(0.0);
        let stage =
            ((age / config.crop_stage_seconds.max(1.0)).floor() as u8).min(CropState::MATURE_STAGE);
        if stage > crop.stage {
            let tended_recently = crop
                .last_tended_tick
                .is_some_and(|tick| state.tick.saturating_sub(tick) <= 1);
            if environmental_pressure > 0 && !tended_recently {
                crop.quality = crop.quality.saturating_sub(environmental_pressure);
            }
            crop.stage = stage;
            plot.crop = Some(crop);
            changed.push(*plot);
        }
    }
    for plot in changed {
        super::push_event(state, WorldEvent::Farming(plot));
    }
}

pub(super) fn farm_plots() -> Vec<FarmPlot> {
    crate::content::farm_plot_positions()
        .into_iter()
        .map(|position| FarmPlot {
            position,
            crop: None,
        })
        .collect()
}

pub(super) fn world_tiles(width: u32, height: u32) -> Vec<WorldTile> {
    (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| WorldTile {
                position: Position {
                    x: x as i32,
                    y: y as i32,
                },
                kind: tile_at(
                    Position {
                        x: x as i32,
                        y: y as i32,
                    },
                    width,
                    height,
                ),
            })
        })
        .collect()
}

pub(super) fn tile_at(position: Position, width: u32, height: u32) -> TileKind {
    let x = position.x as u32;
    let y = position.y as u32;
    if x >= width || y >= height {
        return TileKind::Water;
    }
    if (x <= 1 && y <= 4) || (x == 16 && (2..=8).contains(&y)) {
        TileKind::Water
    } else if ((12..=15).contains(&x) && y <= 4) || ((13..=16).contains(&x) && y >= 8) {
        TileKind::Forest
    } else if ((2..16).contains(&x) && y == 6) || (x == 8 && (4..7).contains(&y)) {
        TileKind::Path
    } else if crate::content::farm_plot_positions().contains(&position) {
        TileKind::Field
    } else if x == 10 && y == 3 {
        TileKind::Stone
    } else {
        TileKind::Meadow
    }
}
