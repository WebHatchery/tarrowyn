//! Embedded design data for the Phase 0 client.

use macroquad_toolkit::assets::TextureConfig;
use macroquad_toolkit::data_loader::{
    load_embedded_json, load_embedded_json_labeled, DataRegistry,
};
use serde::{Deserialize, Serialize};

const GAME_CONFIG_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/game_config.json");
const ACTIONS_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/actions.json");
const CROPS_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/crops.json");
const TEXTURE_MANIFEST_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/texture_manifest.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub game_name: String,
    pub display_name: String,
    pub save_slot: String,
    pub version: String,
    pub world_width: usize,
    pub world_height: usize,
    pub day_length_seconds: f32,
    pub starting_gold: u32,
    pub starting_seeds: u32,
    pub starting_skill: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: ActionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Plant,
    Tend,
    Harvest,
    Listen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropDef {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct GameData {
    pub config: GameConfig,
    pub actions: DataRegistry<ActionDef>,
    pub crops: DataRegistry<CropDef>,
    pub texture_manifest: Vec<TextureConfig>,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let config = load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?;
        let actions = DataRegistry::from_embedded_json(ACTIONS_JSON, "id")?;
        let crops = DataRegistry::from_embedded_json(CROPS_JSON, "id")?;
        let texture_manifest = load_embedded_json(TEXTURE_MANIFEST_JSON)?;

        Ok(Self {
            config,
            actions,
            crops,
            texture_manifest,
        })
    }
}

#[cfg(test)]
mod tests;
