//! Runtime sprite assets and atlas crops used by the world map.

use macroquad::prelude::*;
use macroquad_toolkit::assets::AssetManager;
use macroquad_toolkit::grid::TilePos;

use crate::state::{CropKind, TileKind};

const NPC_ATLAS_KEY: &str = "hearth_npcs";
const MONSTER_KEY: &str = "brambleback";
const ITEM_ATLAS_KEY: &str = "hearth_items";

const TERRAIN_KEY: &str = "terrain_atlas_v2";
const FARMING_KEY: &str = "farming_atlas";
const PLAYER_KEY: &str = "player_atlas";
const SETTLEMENTS_KEY: &str = "settlements_atlas";
const COMBAT_KEY: &str = "combat_atlas";
const ECONOMY_KEY: &str = "items_economy_atlas";
const UI_ICONS_KEY: &str = "ui_icons_atlas";
const WEATHER_KEY: &str = "weather_events_atlas";
const PORTRAITS_KEY: &str = "npc_portraits";

#[derive(Debug, Clone, Copy)]
pub enum NpcSprite {
    Iven,
    Sella,
}

impl NpcSprite {
    fn atlas_index(self) -> f32 {
        match self {
            Self::Iven => 0.0,
            Self::Sella => 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ItemSprite {
    Wheat,
    Turnips,
    Moonberries,
    Seeds,
    Timber,
    Stone,
    Bandages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtAtlas {
    Terrain,
    Farming,
    Player,
    Settlements,
    Combat,
    Economy,
    UiIcons,
    WeatherEvents,
    NpcPortraits,
    ExistingNpcs,
    ExistingMonster,
    ExistingItems,
}

impl ArtAtlas {
    pub fn label(self) -> &'static str {
        match self {
            Self::Terrain => "Terrain tiles",
            Self::Farming => "Crops & farming",
            Self::Player => "Player actions",
            Self::Settlements => "Settlements & routes",
            Self::Combat => "Brambleback & combat",
            Self::Economy => "Resources & economy",
            Self::UiIcons => "UI & action icons",
            Self::WeatherEvents => "Weather & events",
            Self::NpcPortraits => "Dialogue portraits",
            Self::ExistingNpcs => "Existing NPCs",
            Self::ExistingMonster => "Existing Brambleback",
            Self::ExistingItems => "Existing items",
        }
    }

    fn grid(self) -> (f32, f32) {
        match self {
            Self::Settlements => (8.0, 6.0),
            Self::NpcPortraits => (4.0, 2.0),
            Self::ExistingNpcs => (2.0, 1.0),
            Self::ExistingItems => (7.0, 1.0),
            Self::ExistingMonster => (1.0, 1.0),
            _ => (8.0, 8.0),
        }
    }
}

impl ItemSprite {
    fn atlas_index(self) -> f32 {
        match self {
            Self::Wheat => 0.0,
            Self::Turnips => 1.0,
            Self::Moonberries => 2.0,
            Self::Seeds => 3.0,
            Self::Timber => 4.0,
            Self::Stone => 5.0,
            Self::Bandages => 6.0,
        }
    }
}

#[derive(Clone)]
pub struct SpriteAssets {
    npc_atlas: Option<Texture2D>,
    monster: Option<Texture2D>,
    item_atlas: Option<Texture2D>,
    terrain: Option<Texture2D>,
    farming: Option<Texture2D>,
    player: Option<Texture2D>,
    settlements: Option<Texture2D>,
    combat: Option<Texture2D>,
    economy: Option<Texture2D>,
    ui_icons: Option<Texture2D>,
    weather: Option<Texture2D>,
    portraits: Option<Texture2D>,
}

impl SpriteAssets {
    pub fn from_manager(assets: &AssetManager) -> Self {
        Self {
            npc_atlas: assets.get_texture(NPC_ATLAS_KEY).cloned(),
            monster: assets.get_texture(MONSTER_KEY).cloned(),
            item_atlas: assets.get_texture(ITEM_ATLAS_KEY).cloned(),
            terrain: assets.get_texture(TERRAIN_KEY).cloned(),
            farming: assets.get_texture(FARMING_KEY).cloned(),
            player: assets.get_texture(PLAYER_KEY).cloned(),
            settlements: assets.get_texture(SETTLEMENTS_KEY).cloned(),
            combat: assets.get_texture(COMBAT_KEY).cloned(),
            economy: assets.get_texture(ECONOMY_KEY).cloned(),
            ui_icons: assets.get_texture(UI_ICONS_KEY).cloned(),
            weather: assets.get_texture(WEATHER_KEY).cloned(),
            portraits: assets.get_texture(PORTRAITS_KEY).cloned(),
        }
    }

    pub fn draw_atlas(self: &Self, atlas: ArtAtlas, rect: Rect) -> bool {
        let Some(texture) = self.texture(atlas) else {
            return false;
        };
        draw_region(
            texture,
            Rect::new(0.0, 0.0, texture.width(), texture.height()),
            rect.center(),
            rect.size(),
            WHITE,
        );
        true
    }

    pub fn draw_atlas_cell(
        self: &Self,
        atlas: ArtAtlas,
        index: usize,
        center: Vec2,
        size: Vec2,
        tint: Color,
    ) -> bool {
        let Some(texture) = self.texture(atlas) else {
            return false;
        };
        let (columns, rows) = atlas.grid();
        let cell_width = texture.width() / columns;
        let cell_height = texture.height() / rows;
        let column = index as f32 % columns;
        let row = (index as f32 / columns).floor();
        if row >= rows {
            return false;
        }
        let edge_bleed = if atlas == ArtAtlas::Terrain { 2.5 } else { 0.0 };
        draw_region(
            texture,
            Rect::new(
                column * cell_width + edge_bleed,
                row * cell_height + edge_bleed,
                cell_width - edge_bleed * 2.0,
                cell_height - edge_bleed * 2.0,
            ),
            center,
            size,
            tint,
        );
        true
    }

    pub fn draw_terrain_tile(
        &self,
        tile: TileKind,
        position: TilePos,
        center: Vec2,
        size: Vec2,
        night: bool,
    ) -> bool {
        let variant = (position.x.unsigned_abs() + position.y.unsigned_abs()) as usize % 4;
        let base = match tile {
            TileKind::Meadow => 0,
            TileKind::Path => 8,
            TileKind::Field => 16,
            TileKind::Forest => 24,
            TileKind::Water => 32,
            TileKind::Stone => 40,
        };
        let index = if night {
            56 + variant.min(7)
        } else {
            base + variant
        };
        self.draw_atlas_cell(ArtAtlas::Terrain, index, center, size, WHITE)
    }

    pub fn draw_crop(&self, kind: CropKind, stage: u8, center: Vec2, size: Vec2) -> bool {
        let base = match kind {
            CropKind::Wheat => 0,
            CropKind::Turnip => 8,
            CropKind::Moonberry => 16,
        };
        self.draw_atlas_cell(
            ArtAtlas::Farming,
            base + stage.min(3) as usize,
            center,
            size,
            WHITE,
        )
    }

    fn texture(&self, atlas: ArtAtlas) -> Option<&Texture2D> {
        match atlas {
            ArtAtlas::Terrain => self.terrain.as_ref(),
            ArtAtlas::Farming => self.farming.as_ref(),
            ArtAtlas::Player => self.player.as_ref(),
            ArtAtlas::Settlements => self.settlements.as_ref(),
            ArtAtlas::Combat => self.combat.as_ref(),
            ArtAtlas::Economy => self.economy.as_ref(),
            ArtAtlas::UiIcons => self.ui_icons.as_ref(),
            ArtAtlas::WeatherEvents => self.weather.as_ref(),
            ArtAtlas::NpcPortraits => self.portraits.as_ref(),
            ArtAtlas::ExistingNpcs => self.npc_atlas.as_ref(),
            ArtAtlas::ExistingMonster => self.monster.as_ref(),
            ArtAtlas::ExistingItems => self.item_atlas.as_ref(),
        }
    }

    pub fn draw_npc(&self, sprite: NpcSprite, center: Vec2, size: Vec2) -> bool {
        let Some(texture) = self.npc_atlas.as_ref() else {
            return false;
        };
        let cell_width = texture.width() * 0.5;
        let source = Rect::new(
            sprite.atlas_index() * cell_width,
            0.0,
            cell_width,
            texture.height(),
        );
        draw_region(texture, source, center, size, WHITE);
        true
    }

    pub fn draw_monster(&self, center: Vec2, size: Vec2) -> bool {
        let Some(texture) = self.monster.as_ref() else {
            return false;
        };
        draw_region(
            texture,
            Rect::new(0.0, 0.0, texture.width(), texture.height()),
            center,
            size,
            WHITE,
        );
        true
    }

    pub fn draw_item(&self, sprite: ItemSprite, center: Vec2, size: Vec2) -> bool {
        let Some(texture) = self.item_atlas.as_ref() else {
            return false;
        };
        let cell_width = texture.width() / 7.0;
        let source = Rect::new(
            sprite.atlas_index() * cell_width,
            texture.height() * 0.18,
            cell_width,
            texture.height() * 0.50,
        );
        draw_region(texture, source, center, size, WHITE);
        true
    }
}

fn draw_region(texture: &Texture2D, source: Rect, center: Vec2, size: Vec2, tint: Color) {
    draw_texture_ex(
        texture,
        center.x - size.x * 0.5,
        center.y - size.y * 0.5,
        tint,
        DrawTextureParams {
            dest_size: Some(size),
            source: Some(source),
            ..Default::default()
        },
    );
}
