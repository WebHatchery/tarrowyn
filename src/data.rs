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
    /// Local authority-server base URL used by native development builds.
    pub server_url: String,
    /// Optional public or same-origin gateway base URL for published builds.
    /// Empty keeps the local development URL as a safe preview fallback.
    #[serde(default)]
    pub gateway_url: String,
    pub world_width: usize,
    pub world_height: usize,
    pub day_length_seconds: f32,
    pub starting_gold: u32,
    pub starting_seeds: u32,
    pub starting_skill: u32,
}

impl GameConfig {
    /// Select the authority endpoint for this build.
    ///
    /// Native development keeps the local server as its default and accepts a
    /// `TARROWYN_SERVER_URL` override. Published native and WebGL builds use
    /// the embedded gateway when one is configured; WebGL cannot read process
    /// environment at runtime.
    pub fn connection_url(&self) -> String {
        let override_url: Option<String> = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::env::var("TARROWYN_SERVER_URL").ok()
            }
            #[cfg(target_arch = "wasm32")]
            {
                None
            }
        };

        select_connection_url(
            &self.server_url,
            &self.gateway_url,
            override_url.as_deref(),
            cfg!(target_arch = "wasm32") || !cfg!(debug_assertions),
        )
    }
}

fn select_connection_url(
    server_url: &str,
    gateway_url: &str,
    override_url: Option<&str>,
    published: bool,
) -> String {
    if let Some(override_url) = override_url.map(str::trim).filter(|url| !url.is_empty()) {
        return override_url.to_owned();
    }
    let gateway = gateway_url.trim();
    if published && !gateway.is_empty() {
        return gateway.to_owned();
    }
    server_url.to_owned()
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

        let data = Self {
            config,
            actions,
            crops,
            texture_manifest,
        };
        data.validate_required_content()?;
        Ok(data)
    }

    fn validate_required_content(&self) -> Result<(), String> {
        for (id, kind) in [
            ("plant", ActionKind::Plant),
            ("tend", ActionKind::Tend),
            ("harvest", ActionKind::Harvest),
            ("listen", ActionKind::Listen),
        ] {
            let Some(action) = self.actions.get(id) else {
                return Err(format!("actions is missing required entry `{id}`"));
            };
            if action.kind != kind {
                return Err(format!(
                    "actions entry `{id}` has kind {:?}; expected {:?}",
                    action.kind, kind
                ));
            }
        }

        for id in ["wheat", "turnip", "moonberry"] {
            if !self.crops.contains(id) {
                return Err(format!("crops is missing required entry `{id}`"));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
