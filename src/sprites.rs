//! Runtime sprite assets and atlas crops used by the world map.

use macroquad::prelude::*;
use macroquad_toolkit::assets::AssetManager;

const NPC_ATLAS_KEY: &str = "hearth_npcs";
const MONSTER_KEY: &str = "brambleback";
const ITEM_ATLAS_KEY: &str = "hearth_items";

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
}

impl SpriteAssets {
    pub fn from_manager(assets: &AssetManager) -> Self {
        Self {
            npc_atlas: assets.get_texture(NPC_ATLAS_KEY).cloned(),
            monster: assets.get_texture(MONSTER_KEY).cloned(),
            item_atlas: assets.get_texture(ITEM_ATLAS_KEY).cloned(),
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
