//! Client-side world types used by the authoritative shared-road projection.

use crate::data::GameConfig;
use macroquad::prelude::Color;
use macroquad_toolkit::grid::{FlatGrid, TilePos};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileKind {
    Meadow,
    Path,
    Field,
    Forest,
    Water,
    Stone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CropKind {
    Wheat,
    Turnip,
    Moonberry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CropState {
    pub kind: CropKind,
    pub stage: u8,
}

impl CropState {
    pub const MATURE_STAGE: u8 = 3;

    pub fn mature(&self) -> bool {
        self.stage >= Self::MATURE_STAGE
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub tiles: FlatGrid<TileKind>,
    pub crops: FlatGrid<Option<CropState>>,
    #[serde(default)]
    pub reachable: HashSet<TilePos>,
}

impl WorldState {
    pub fn new(config: &GameConfig) -> Self {
        let mut tiles = FlatGrid::new(config.world_width, config.world_height, TileKind::Meadow);

        for y in 0..config.world_height {
            for x in 0..config.world_width {
                let pos = TilePos::new(x as i32, y as i32);
                if (x <= 1 && y <= 4) || (x == 16 && (2..=8).contains(&y)) {
                    tiles.set(pos, TileKind::Water);
                } else if ((12..=15).contains(&x) && y <= 4) || ((13..=16).contains(&x) && y >= 8) {
                    tiles.set(pos, TileKind::Forest);
                }
            }
        }

        for x in 2..16 {
            tiles.set(TilePos::new(x, 6), TileKind::Path);
        }
        for y in 4..7 {
            tiles.set(TilePos::new(8, y), TileKind::Path);
        }
        for x in 3..6 {
            for y in 4..6 {
                tiles.set(TilePos::new(x, y), TileKind::Field);
            }
        }
        tiles.set(TilePos::new(10, 3), TileKind::Stone);

        Self {
            crops: FlatGrid::new(config.world_width, config.world_height, None),
            tiles,
            reachable: HashSet::new(),
        }
    }
}

pub fn tile_color(tile: TileKind) -> Color {
    match tile {
        TileKind::Meadow => Color::new(0.25, 0.42, 0.29, 1.0),
        TileKind::Path => Color::new(0.60, 0.48, 0.31, 1.0),
        TileKind::Field => Color::new(0.48, 0.37, 0.22, 1.0),
        TileKind::Forest => Color::new(0.12, 0.28, 0.22, 1.0),
        TileKind::Water => Color::new(0.13, 0.31, 0.45, 1.0),
        TileKind::Stone => Color::new(0.33, 0.35, 0.36, 1.0),
    }
}
