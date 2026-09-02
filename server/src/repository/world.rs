//! Shared map construction, crop growth, and field environmental pressure.

use super::models::RepositoryState;
use crate::config::ServerConfig;
use tarrowyn_protocol::{
    CropState, FarmPlot, FarmingAction, FieldWeather, Position, TileKind, WorldEvent, WorldTile,
};

const MAX_OFFLINE_CROP_MILLIS: u64 = 7 * 24 * 60 * 60 * 1_000;

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
    let changed = advance_plot_growth(state, config, 1);
    for plot in changed {
        super::push_event(state, WorldEvent::Farming(plot));
    }
}

pub(super) fn apply_offline_crop_growth(
    state: &mut RepositoryState,
    config: &ServerConfig,
    persisted_at_unix_millis: u64,
    now_unix_millis: u64,
) {
    if persisted_at_unix_millis == 0 || now_unix_millis <= persisted_at_unix_millis {
        return;
    }
    let elapsed_millis = now_unix_millis
        .saturating_sub(persisted_at_unix_millis)
        .min(MAX_OFFLINE_CROP_MILLIS);
    let tick_millis = config
        .tick_interval
        .as_millis()
        .max(1)
        .min(u128::from(u64::MAX)) as u64;
    let elapsed_ticks = elapsed_millis / tick_millis;
    if elapsed_ticks > 0 {
        advance_plot_growth(state, config, elapsed_ticks);
    }
}

fn advance_plot_growth(
    state: &mut RepositoryState,
    config: &ServerConfig,
    elapsed_ticks: u64,
) -> Vec<FarmPlot> {
    let weather_pressure = field_weather_for_day(state.clock.day).pressure();
    let pest_pressure = field_pest_pressure_for_day(state.clock.day);
    let environmental_pressure = weather_pressure.saturating_add(pest_pressure).min(2);
    let mut changed = Vec::new();
    for plot in &mut state.plots {
        let Some(mut crop) = plot.crop else { continue };
        crop.growth_ticks = crop.growth_ticks.saturating_add(elapsed_ticks);
        let age = crop.growth_ticks as f32 * config.world_seconds_per_tick.max(0.0);
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
            changed.push(FarmPlot {
                position: plot.position,
                crop: Some(crop),
            });
        }
        plot.crop = Some(crop);
    }
    changed
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

pub(super) fn restore_plots(
    stored: Vec<FarmPlot>,
    stored_tick: u64,
    migrate_legacy_growth: bool,
) -> Vec<FarmPlot> {
    if stored.is_empty() || is_empty_legacy_plot_layout(&stored) {
        farm_plots()
    } else {
        stored
            .into_iter()
            .map(|mut plot| {
                if migrate_legacy_growth {
                    if let Some(crop) = plot.crop.as_mut() {
                        crop.growth_ticks = stored_tick.saturating_sub(crop.planted_tick);
                    }
                }
                plot
            })
            .collect()
    }
}

fn is_empty_legacy_plot_layout(plots: &[FarmPlot]) -> bool {
    const LEGACY_POSITIONS: [Position; 6] = [
        Position { x: 3, y: 4 },
        Position { x: 3, y: 5 },
        Position { x: 4, y: 4 },
        Position { x: 4, y: 5 },
        Position { x: 5, y: 4 },
        Position { x: 5, y: 5 },
    ];
    plots.len() == LEGACY_POSITIONS.len()
        && plots
            .iter()
            .zip(LEGACY_POSITIONS)
            .all(|(plot, position)| plot.position == position && plot.crop.is_none())
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
    let Some((x, y)) = position_in_world(position, width, height) else {
        return TileKind::Water;
    };
    if x >= width || y >= height {
        return TileKind::Water;
    }
    if (x <= 1 && y <= 4) || (x == 16 && (2..=8).contains(&y)) {
        return TileKind::Water;
    }
    if ((12..=15).contains(&x) && y <= 4) || ((13..=16).contains(&x) && y >= 8) {
        return TileKind::Forest;
    }
    if ((2..16).contains(&x) && y == 6) || (x == 8 && (4..7).contains(&y)) {
        return TileKind::Path;
    }
    if crate::content::farm_plot_positions().contains(&position) {
        return TileKind::Field;
    }
    if x == 10 && y == 3 {
        return TileKind::Stone;
    }
    TileKind::Meadow
}

pub(super) fn position_in_world(position: Position, width: u32, height: u32) -> Option<(u32, u32)> {
    let x = u32::try_from(position.x).ok()?;
    let y = u32::try_from(position.y).ok()?;
    if x >= width || y >= height {
        return None;
    }
    Some((x, y))
}
