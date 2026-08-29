//! Local Phase 0 world state and the persistence shape that will become a
//! client projection when the authoritative server arrives.

use crate::data::{ActionDef, ActionKind, GameConfig};
use macroquad::prelude::Color;
use macroquad_toolkit::grid::{FlatGrid, TilePos};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use tarrowyn_protocol::TimeOfDay;

pub const TAVERN_TILE: TilePos = TilePos { x: 8, y: 5 };

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileKind {
    Meadow,
    Path,
    Field,
    Forest,
    Water,
    Stone,
}

impl TileKind {
    pub fn is_walkable(self) -> bool {
        !matches!(self, Self::Water)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CropKind {
    Wheat,
    Turnip,
    Moonberry,
}

impl CropKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Wheat => "Wheat",
            Self::Turnip => "Turnip",
            Self::Moonberry => "Moonberry",
        }
    }

    fn from_seed_index(index: u32) -> Self {
        match index % 3 {
            0 => Self::Wheat,
            1 => Self::Turnip,
            _ => Self::Moonberry,
        }
    }
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub wheat: u32,
    pub turnips: u32,
    pub moonberries: u32,
    pub seeds: u32,
}

impl Inventory {
    pub fn total_crops(&self) -> u32 {
        self.wheat
            .saturating_add(self.turnips)
            .saturating_add(self.moonberries)
    }

    fn add_crop(&mut self, kind: CropKind) {
        match kind {
            CropKind::Wheat => self.wheat = self.wheat.saturating_add(1),
            CropKind::Turnip => self.turnips = self.turnips.saturating_add(1),
            CropKind::Moonberry => self.moonberries = self.moonberries.saturating_add(1),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub position: TilePos,
    pub gold: u32,
    pub skill: u32,
    pub reputation: u32,
    pub inventory: Inventory,
    pub seeds_planted: u32,
    pub actions_completed: u32,
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

    fn refresh_reachable(&mut self, player: TilePos) {
        self.reachable = self
            .tiles
            .flood_fill(player, false, |_, tile| tile.is_walkable());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: String,
    pub player: PlayerState,
    pub world: WorldState,
    pub day: u32,
    pub day_seconds: f32,
    pub journal: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GameSession {
    pub player: PlayerState,
    pub world: WorldState,
    pub day: u32,
    pub day_seconds: f32,
    pub journal: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionResult {
    pub success: bool,
    pub message: String,
}

impl GameSession {
    pub fn new(config: &GameConfig) -> Self {
        let mut world = WorldState::new(config);
        let position = TilePos::new(8, 6);
        let crops = [
            (TilePos::new(3, 4), CropKind::Wheat, CropState::MATURE_STAGE),
            (TilePos::new(4, 4), CropKind::Turnip, 2),
            (TilePos::new(5, 4), CropKind::Moonberry, 1),
        ];
        for (tile, kind, stage) in crops {
            world.crops.set(tile, Some(CropState { kind, stage }));
        }
        world.refresh_reachable(position);

        Self {
            player: PlayerState {
                position,
                gold: config.starting_gold,
                skill: config.starting_skill,
                reputation: 0,
                inventory: Inventory {
                    seeds: config.starting_seeds,
                    ..Inventory::default()
                },
                seeds_planted: 0,
                actions_completed: 0,
            },
            world,
            day: 1,
            day_seconds: 0.0,
            journal: vec!["A new evening begins at the Hearth.".to_owned()],
        }
    }

    pub fn from_save(mut save: SaveData) -> Self {
        save.world.refresh_reachable(save.player.position);
        Self {
            player: save.player,
            world: save.world,
            day: save.day.max(1),
            day_seconds: save.day_seconds.max(0.0),
            journal: if save.journal.is_empty() {
                vec!["The local chronicle is quiet.".to_owned()]
            } else {
                save.journal
            },
        }
    }

    pub fn to_save(&self, version: &str) -> SaveData {
        SaveData {
            version: version.to_owned(),
            player: self.player.clone(),
            world: self.world.clone(),
            day: self.day,
            day_seconds: self.day_seconds,
            journal: self.journal.clone(),
        }
    }

    pub fn update_clock(&mut self, config: &GameConfig, dt: f32) -> bool {
        self.day_seconds += dt.max(0.0);
        let day_length = config.day_length_seconds.max(1.0);
        let mut advanced = false;
        while self.day_seconds >= day_length {
            self.day_seconds -= day_length;
            self.day = self.day.saturating_add(1);
            advanced = true;
            self.advance_crops();
        }
        advanced
    }

    pub fn clock_minutes(&self, config: &GameConfig) -> u32 {
        let day_fraction = (self.day_seconds / config.day_length_seconds.max(1.0)).clamp(0.0, 1.0);
        (((6.0 + day_fraction * 24.0) % 24.0) * 60.0) as u32
    }

    pub fn is_night(&self, config: &GameConfig) -> bool {
        self.time_of_day(config).is_night()
    }

    pub fn time_of_day(&self, config: &GameConfig) -> TimeOfDay {
        TimeOfDay::from_clock_minutes(self.clock_minutes(config))
    }

    pub fn move_player(&mut self, dx: i32, dy: i32) -> bool {
        let next = TilePos::new(
            self.player.position.x + dx.signum(),
            self.player.position.y + dy.signum(),
        );
        if !self.world.tiles.is_valid(next)
            || !self
                .world
                .tiles
                .get(next)
                .is_some_and(|tile| tile.is_walkable())
        {
            return false;
        }

        self.player.position = next;
        self.world.refresh_reachable(next);
        true
    }

    pub fn move_toward(&mut self, target: TilePos) -> bool {
        let dx = target.x - self.player.position.x;
        let dy = target.y - self.player.position.y;
        if dx.abs() >= dy.abs() && dx != 0 {
            self.move_player(dx.signum(), 0)
        } else if dy != 0 {
            self.move_player(0, dy.signum())
        } else {
            false
        }
    }

    pub fn apply_action(&mut self, action: &ActionDef) -> ActionResult {
        let result = match action.kind {
            ActionKind::Plant => self.plant_crop(),
            ActionKind::Tend => self.tend_crop(),
            ActionKind::Harvest => self.harvest_crop(),
            ActionKind::Listen => self.listen_at_tavern(),
        };
        if result.success {
            self.player.actions_completed = self.player.actions_completed.saturating_add(1);
            self.record(result.message.clone());
        }
        result
    }

    pub fn crops_ready(&self) -> usize {
        self.world
            .crops
            .data()
            .iter()
            .filter_map(|crop| *crop)
            .filter(CropState::mature)
            .count()
    }

    pub fn last_activity(&self) -> &str {
        self.journal
            .last()
            .map(String::as_str)
            .unwrap_or("The evening is waiting.")
    }

    pub fn format_inventory(&self) -> String {
        format!(
            "Wheat {}  Turnips {}  Moonberries {}  Seeds {}",
            self.player.inventory.wheat,
            self.player.inventory.turnips,
            self.player.inventory.moonberries,
            self.player.inventory.seeds
        )
    }

    fn plant_crop(&mut self) -> ActionResult {
        if self.player.inventory.seeds == 0 {
            return failure("Your seed pouch is empty; reconnect to use the shared-road market.");
        }
        let Some(tile) = self.nearby_field(|crop| crop.is_none()) else {
            return failure("Stand near an empty shared field plot to plant.");
        };
        let kind = CropKind::from_seed_index(self.player.seeds_planted);
        self.world
            .crops
            .set(tile, Some(CropState { kind, stage: 0 }));
        self.player.inventory.seeds -= 1;
        self.player.seeds_planted = self.player.seeds_planted.saturating_add(1);
        success(format!("Planted {} in the shared fields.", kind.name()))
    }

    fn tend_crop(&mut self) -> ActionResult {
        let Some(tile) = self.nearby_field(|crop| crop.is_some()) else {
            return failure("There is no crop close enough to tend.");
        };
        let Some(mut crop) = self.world.crops.get(tile).copied().flatten() else {
            return failure("That plot is empty.");
        };
        if crop.mature() {
            return failure(format!("The {} is ready to harvest.", crop.kind.name()));
        }
        crop.stage = crop.stage.saturating_add(1);
        self.world.crops.set(tile, Some(crop));
        self.player.skill = self.player.skill.saturating_add(1);
        success(format!(
            "Tended the {}. Growth stage {}/3.",
            crop.kind.name(),
            crop.stage
        ))
    }

    fn harvest_crop(&mut self) -> ActionResult {
        let Some(tile) = self.nearby_field(|crop| crop.is_some_and(|candidate| candidate.mature()))
        else {
            return failure("No mature crop is close enough to harvest.");
        };
        let Some(crop) = self.world.crops.get(tile).copied().flatten() else {
            return failure("That plot is empty.");
        };
        self.world.crops.set(tile, None);
        self.player.inventory.add_crop(crop.kind);
        self.player.gold = self.player.gold.saturating_add(2);
        self.player.skill = self.player.skill.saturating_add(1);
        success(format!("Harvested {} and earned 2 gold.", crop.kind.name()))
    }

    fn listen_at_tavern(&mut self) -> ActionResult {
        if self.player.position.manhattan_distance(&TAVERN_TILE) > 1 {
            return failure("The Hearth is just along the path; walk closer to listen.");
        }
        self.player.reputation = self.player.reputation.saturating_add(1);
        success("A traveller mentions lanterns moving beyond Whisperwood.".to_owned())
    }

    fn nearby_field<F>(&self, predicate: F) -> Option<TilePos>
    where
        F: Fn(Option<CropState>) -> bool,
    {
        self.world
            .tiles
            .iter_with_pos()
            .filter(|(pos, tile)| {
                **tile == TileKind::Field
                    && pos.manhattan_distance(&self.player.position) <= 4
                    && self
                        .world
                        .crops
                        .get(*pos)
                        .copied()
                        .flatten()
                        .map_or_else(|| predicate(None), |crop| predicate(Some(crop)))
            })
            .min_by_key(|(pos, _)| pos.manhattan_distance(&self.player.position))
            .map(|(pos, _)| pos)
    }

    fn advance_crops(&mut self) {
        for crop in self.world.crops.data_mut().iter_mut().flatten() {
            crop.stage = crop.stage.saturating_add(1).min(CropState::MATURE_STAGE);
        }
        self.record(format!(
            "Day {} begins; the fields have grown quietly.",
            self.day
        ));
    }

    fn record(&mut self, message: String) {
        self.journal.push(message);
        if self.journal.len() > 8 {
            self.journal.remove(0);
        }
    }
}

fn success(message: String) -> ActionResult {
    ActionResult {
        success: true,
        message,
    }
}

fn failure(message: impl Into<String>) -> ActionResult {
    ActionResult {
        success: false,
        message: message.into(),
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

pub fn migrate_save_value(
    detected_version: Option<String>,
    value: Value,
    config: &GameConfig,
) -> Result<SaveData, String> {
    let payload = value.get("data").cloned().unwrap_or(value);
    let mut save: SaveData = serde_json::from_value(payload).map_err(|err| {
        format!(
            "Unsupported Tarrowyn save format {:?}: {}",
            detected_version, err
        )
    })?;
    save.version = config.version.clone();
    save.day = save.day.max(1);
    save.day_seconds = save
        .day_seconds
        .clamp(0.0, config.day_length_seconds.max(1.0));
    Ok(save)
}

#[cfg(test)]
mod tests;
