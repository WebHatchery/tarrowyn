use super::*;
use crate::sprites::{ArtAtlas, SpriteAssets};

const PAGE_COUNT: usize = 3;
const PAGE_ATLASES: [[ArtAtlas; 4]; PAGE_COUNT] = [
    [
        ArtAtlas::Terrain,
        ArtAtlas::Farming,
        ArtAtlas::Player,
        ArtAtlas::Settlements,
    ],
    [
        ArtAtlas::Combat,
        ArtAtlas::Economy,
        ArtAtlas::UiIcons,
        ArtAtlas::WeatherEvents,
    ],
    [
        ArtAtlas::NpcPortraits,
        ArtAtlas::ExistingNpcs,
        ArtAtlas::ExistingMonster,
        ArtAtlas::ExistingItems,
    ],
];

pub(super) fn draw(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    draw_rectangle(
        0.0,
        0.0,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        Color::new(0.005, 0.012, 0.014, 0.84),
    );
    let panel = Rect::new(28.0, 48.0, 1224.0, 624.0);
    draw_surface(
        panel,
        &SurfaceStyle::new(Color::new(0.025, 0.045, 0.046, 0.99))
            .with_shadow(vec2(0.0, 8.0), Color::new(0.0, 0.0, 0.0, 0.48))
            .with_left_accent(4.0, GOLD)
            .with_top_highlight(2.0, MINT),
    );
    draw_ui_text_ex(
        "ART ATLAS",
        panel.x + 28.0,
        panel.y + 34.0,
        TextStyle::new(22.0, CREAM).params(),
    );
    draw_ui_text_ex(
        "Loaded game-facing sheets • tap Next to inspect every art family",
        panel.x + 30.0,
        panel.y + 56.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );
    draw_ui_text_ex(
        &format!("PAGE {} / {}", ctx.art_catalog_page + 1, PAGE_COUNT),
        panel.right() - 150.0,
        panel.y + 34.0,
        TextStyle::new(11.0, GOLD).params(),
    );

    let cards = card_grid(panel);
    let atlases = PAGE_ATLASES[ctx.art_catalog_page.min(PAGE_COUNT - 1)];
    for (index, atlas) in atlases.into_iter().enumerate() {
        draw_card(ctx.sprites, atlas, cards[index]);
    }

    if super::virtual_button(
        Rect::new(panel.x + 28.0, panel.bottom() - 42.0, 108.0, 28.0),
        "Prev",
        ctx.art_catalog_page > 0,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("art-page-prev".to_owned()));
    }
    if super::virtual_button(
        Rect::new(panel.x + 144.0, panel.bottom() - 42.0, 108.0, 28.0),
        "Next",
        ctx.art_catalog_page + 1 < PAGE_COUNT,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::Interact("art-page-next".to_owned()));
    }
    if super::virtual_button(
        Rect::new(panel.right() - 138.0, panel.bottom() - 42.0, 110.0, 28.0),
        "Back",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("art-catalog-close".to_owned()));
    }
}

fn card_grid(panel: Rect) -> [Rect; 4] {
    let gap = 12.0;
    let width = (panel.w - 56.0 - gap) * 0.5;
    let height = 226.0;
    let x = panel.x + 28.0;
    let y = panel.y + 78.0;
    [
        Rect::new(x, y, width, height),
        Rect::new(x + width + gap, y, width, height),
        Rect::new(x, y + height + gap, width, height),
        Rect::new(x + width + gap, y + height + gap, width, height),
    ]
}

fn draw_card(sprites: &SpriteAssets, atlas: ArtAtlas, card: Rect) {
    draw_surface(
        card,
        &SurfaceStyle::new(Color::new(0.045, 0.068, 0.067, 0.98))
            .with_border(1.0, Color::new(0.22, 0.40, 0.39, 0.82))
            .with_top_highlight(1.0, Color::new(0.50, 0.82, 0.68, 0.28)),
    );
    draw_ui_text_ex(
        atlas.label(),
        card.x + 12.0,
        card.y + 21.0,
        TextStyle::new(12.0, CREAM).params(),
    );
    let preview = Rect::new(card.x + 10.0, card.y + 32.0, card.w - 20.0, card.h - 44.0);
    if !sprites.draw_atlas(atlas, preview) {
        draw_ui_text_ex(
            "asset unavailable",
            preview.x + 12.0,
            preview.center().y,
            TextStyle::new(11.0, dark::TEXT_DIM).params(),
        );
    }
}
