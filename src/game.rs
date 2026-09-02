//! Client loop, online authority boundary, and presentation services.

use crate::data::GameData;
use crate::network::{ConnectionState, CraftingView, NetworkNotice, OnlineClient};
use crate::sprites::SpriteAssets;
use crate::ui::{self, UiAction, UiContext};
use macroquad::prelude::*;
use macroquad_toolkit::assets::AssetManager;
use macroquad_toolkit::camera::{Camera2D, Camera2DConfig, CameraBounds};
use macroquad_toolkit::events::EventBus;
use macroquad_toolkit::grid::TilePos;
use macroquad_toolkit::notifications::{
    NotificationAnchor, NotificationManager, NotificationRenderConfig,
};
use macroquad_toolkit::prelude::{begin_virtual_ui_frame, dark, end_virtual_ui_frame};
use tarrowyn_protocol::{
    FarmingAction, FoundationCacheAction, FoundationForgeAction, FoundationResourceKind,
    TradeAction, TradeBundle, TradeRequest,
};

#[path = "game/actions.rs"]
mod actions;
#[path = "game/input.rs"]
mod input;

#[cfg(test)]
#[path = "game/tests.rs"]
mod tests;

pub struct Game {
    mode: OnlineClient,
    sprites: SpriteAssets,
    notifications: NotificationManager,
    camera: Camera2D,
    events: EventBus<UiAction>,
    chat_draft: String,
    regional_inspection_open: bool,
    skill_selection_open: bool,
    school_selection_open: bool,
    chronicle_open: bool,
    chronicle_query: String,
    account_open: bool,
    menu_open: bool,
    art_catalog_open: bool,
    art_catalog_page: usize,
    movement_frame_seconds: f32,
    movement_advanced_this_frame: bool,
    rendered_player_position: Vec2,
}

impl Game {
    pub async fn new() -> Self {
        let data = GameData::load().unwrap_or_else(|err| {
            panic!("Tarrowyn embedded data failed to load: {err}");
        });
        let mut assets = AssetManager::new();
        let placeholder = Image::gen_image_color(16, 16, Color::new(0.24, 0.42, 0.35, 1.0));
        assets.set_placeholder_texture_direct(Texture2D::from_image(&placeholder));
        assets.load_texture_configs(&data.texture_manifest).await;
        let sprites = SpriteAssets::from_manager(&assets);

        let server_url = data.config.connection_url();
        let mode = OnlineClient::new(&server_url, &data.config);
        let player_position = mode.projection.player_position;
        let camera = Camera2D::with_config(
            vec2(player_position.x as f32, player_position.y as f32),
            1.0,
            Camera2DConfig {
                min_zoom: 0.9,
                max_zoom: 1.15,
                drag_button: None,
                keyboard_pan_enabled: false,
                mouse_drag_enabled: false,
                mouse_wheel_zoom_enabled: false,
                bounds: Some(CameraBounds::new(
                    vec2(0.0, 0.0),
                    vec2(
                        data.config.world_width as f32 - 1.0,
                        data.config.world_height as f32 - 1.0,
                    ),
                )),
                ..Default::default()
            },
        );

        Self {
            mode,
            sprites,
            notifications: NotificationManager::new(),
            camera,
            events: EventBus::new(),
            chat_draft: String::new(),
            regional_inspection_open: false,
            skill_selection_open: false,
            school_selection_open: false,
            chronicle_open: false,
            chronicle_query: String::new(),
            account_open: false,
            menu_open: false,
            art_catalog_open: false,
            art_catalog_page: 0,
            movement_frame_seconds: 0.0,
            movement_advanced_this_frame: false,
            rendered_player_position: vec2(player_position.x as f32, player_position.y as f32),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);
        self.movement_frame_seconds = dt.max(0.0);
        self.movement_advanced_this_frame = false;
        for notice in self.mode.update(dt) {
            self.show_network_notice(notice);
        }
        self.apply_movement_correction();

        self.read_keyboard_input(dt);
        let actions: Vec<UiAction> = self.events.drain().collect();
        for action in actions {
            self.apply_action(action);
        }
    }

    pub fn begin_capture_scene(&mut self, scene: &str) {
        self.art_catalog_open = scene.starts_with("art-");
        self.art_catalog_page = if scene.contains("combat") {
            1
        } else if scene.contains("portraits") {
            2
        } else {
            0
        };
    }

    pub fn draw(&mut self) {
        let night = self.mode.projection.is_night();
        clear_background(if night {
            Color::new(0.025, 0.045, 0.09, 1.0)
        } else {
            dark::BACKGROUND
        });

        let virtual_ui = begin_virtual_ui_frame(ui::LOGICAL_WIDTH, ui::LOGICAL_HEIGHT);
        let client = &self.mode;
        let regional_inspection = self
            .regional_inspection_open
            .then(|| {
                online_gameplay_modal_visible(
                    client.state,
                    client.projection.authoritative_player_position().is_some(),
                )
                .then(|| client.phase5_inspection())
            })
            .flatten();
        let actions = {
            let identity = client
                .account
                .as_ref()
                .map(|account| account.display_name.as_str());
            let own_account_id = client
                .account
                .as_ref()
                .map(|account| account.account_id.as_str());
            let (travel_label, can_travel, can_recover_travel) = client.phase5_travel_control();
            let stats = client
                    .projection
                    .player
                    .as_ref()
                    .map(|player| {
                        format!(
                            "Gold {}  Skill {}  Reputation {}\nRank {} • {} credentials\nField tool {}/3 • {} • pests {}/2 • Goat {}/{}\nWheat {}  Turnips {}  Moonberries {}  Seeds {} • Bandages {}",
                            player.gold,
                            player.skill,
                            player.reputation,
                            player.adventurer_rank.label(),
                            player.adventurer_credentials.len(),
                            player.field_tool_condition,
                            player.field_weather.label(),
                            player.field_pest_pressure,
                            player.animal_condition,
                            player.animal_max_condition,
                            player.inventory.wheat,
                            player.inventory.turnips,
                            player.inventory.moonberries,
                            player.inventory.seeds,
                            player.inventory.bandages
                        )
                    })
                    .unwrap_or_else(|| "Waiting for the persistent player ledger…".to_owned());
            let stats = if client
                .projection
                .player
                .as_ref()
                .is_some_and(|player| player.knocked_out)
            {
                format!("{stats}\nKNOCKED OUT • tap Self, Rescuer, or Healer below")
            } else if let Some(trade) = client.projection.trades.first() {
                format!("{stats}\nTrade {}: {:?}", trade.trade_id, trade.status)
            } else {
                stats
            };
            ui::draw_game_ui(UiContext {
                world: &client.projection.world,
                player_position: client.projection.player_position,
                rendered_player_position: self.rendered_player_position,
                day: client.projection.day,
                calendar_season: client.phase5_season(),
                clock_minutes: client.projection.clock_minutes(),
                night: client.projection.is_night(),
                stats: &stats,
                own_account_id,
                player_position_authoritative: client
                    .projection
                    .authoritative_player_position()
                    .is_some(),
                remote_players: &client.projection.players,
                farm_animals: &client.projection.animals,
                trades: &client.projection.trades,
                trade_pending: client.trade_pending(),
                farming_pending: client.farming_pending(),
                foundation: &client.projection.foundation,
                foundation_activity: &client.projection.foundation_activity,
                player_inventory: client
                    .projection
                    .player
                    .as_ref()
                    .map(|player| &player.inventory),
                player_gold: client.projection.player.as_ref().map(|player| player.gold),
                field_tool_condition: client
                    .projection
                    .player
                    .as_ref()
                    .map(|player| player.field_tool_condition),
                field_tool_kind: client
                    .projection
                    .player
                    .as_ref()
                    .map(|player| player.field_tool_kind),
                field_weather: client
                    .projection
                    .player
                    .as_ref()
                    .map(|player| player.field_weather),
                field_pest_pressure: client
                    .projection
                    .player
                    .as_ref()
                    .map(|player| player.field_pest_pressure),
                foundation_interaction_pending: client.foundation_interaction_pending(),
                server_tick: client.projection.server_tick,
                connection: client.state,
                status_message: &client.status_message,
                expedition: client.projection.expedition.as_ref(),
                identity_name: identity,
                sprites: &self.sprites,
                camera_zoom: self.camera.zoom,
                menu_open: self.menu_open,
                art_catalog_open: self.art_catalog_open,
                art_catalog_page: self.art_catalog_page,
                wilderness: client.projection.wilderness.as_ref(),
                regional_region: client.phase5_region(),
                regional_inspection: regional_inspection.as_deref(),
                regional_event_choices: client.phase5_event_choices(),
                skills: client.phase4_skills(),
                skill_selection_open: self.skill_selection_open
                    && online_gameplay_modal_visible(
                        client.state,
                        client.projection.authoritative_player_position().is_some(),
                    ),
                school_selection_open: self.school_selection_open
                    && online_gameplay_modal_visible(
                        client.state,
                        client.projection.authoritative_player_position().is_some(),
                    ),
                chronicle_open: self.chronicle_open
                    && online_gameplay_modal_visible(
                        client.state,
                        client.projection.authoritative_player_position().is_some(),
                    ),
                chronicle: &client.projection.chronicle,
                chronicle_summary: client.projection.chronicle_summary.as_ref(),
                chronicle_query: &self.chronicle_query,
                chronicle_search: &client.projection.chronicle_search,
                chronicle_search_summary: client.projection.chronicle_search_summary.as_ref(),
                chronicle_search_query: client.projection.chronicle_search_query.as_deref(),
                chronicle_search_next_cursor: client.projection.chronicle_search_next_cursor,
                chronicle_search_pending: client.chronicle_search_pending(),
                account_deletion_armed: client.account_deletion_armed(),
                account_deletion_available: client.account_deletion_available(),
                account_link_available: client.account_link_available(),
                account_open: self.account_open && client.state == ConnectionState::Online,
                account_summary: &client.account_summary(),
                identity_pending: client.identity_pending(),
                report_pending: client.report_pending(),
                crafting: online_crafting_view(
                    client.state,
                    client.projection.authoritative_player_position().is_some(),
                    client.crafting_view(),
                ),
                combat: client.combat_state(),
                storm_magic_unlocked: client.storm_magic_unlocked(),
                skill_pending: client.skill_pending(),
                knowledge_pending: client.knowledge_pending(),
                order_pending: client.order_pending(),
                combat_pending: client.combat_pending(),
                contract_pending: client.contract_pending(),
                expedition_pending: client.expedition_pending(),
                frontier_combat_pending: client.frontier_combat_pending(),
                frontier_claim_pending: client.frontier_claim_pending(),
                knocked_out: client
                    .projection
                    .player
                    .as_ref()
                    .is_some_and(|player| player.knocked_out),
                recovery_pending: client.recovery_pending(),
                has_open_market_order: client.has_open_market_order(),
                market_pending: client.market_pending(),
                event_pending: client.event_pending(),
                route_pending: client.route_pending(),
                governance_pending: client.governance_pending(),
                can_abandon_claim: client.can_abandon_claim(),
                can_transfer_claim: client.can_transfer_claim()
                    && client.projection.players.iter().any(|player| {
                        Some(player.account_id.as_str()) != own_account_id
                            && !player.stale(client.projection.server_tick)
                            && player.position == client.projection.player_position
                    }),
                claim_pending: client.claim_pending(),
                knowledge_label: client.knowledge_cycle_label(
                    client.projection.players.iter().any(|player| {
                        Some(player.account_id.as_str()) != own_account_id
                            && !player.stale(client.projection.server_tick)
                            && player.position == client.projection.player_position
                    }),
                ),
                travel_label,
                can_travel,
                can_recover_travel,
                travel_pending: client.travel_pending(),
                ui: &virtual_ui,
            })
        };
        end_virtual_ui_frame();

        for action in actions {
            self.events.push(action);
        }
        if !self.art_catalog_open {
            self.notifications
                .draw_with_config(&NotificationRenderConfig {
                    anchor: NotificationAnchor::BottomRight,
                    margin: 22.0,
                    width: 360.0,
                    ..Default::default()
                });
        }
    }

    fn move_toward(&mut self, target: TilePos) {
        if self.movement_advanced_this_frame {
            return;
        }
        let target = vec2(target.x as f32, target.y as f32);
        let offset = target - self.rendered_player_position;
        if offset.length_squared() <= f32::EPSILON {
            return;
        }
        let maximum_travel = offset.length();
        self.advance_player_movement(
            offset,
            self.movement_frame_seconds
                .min(maximum_travel / input::PLAYER_MOVEMENT_SPEED),
        );
    }

    fn apply_movement_correction(&mut self) {
        if let Some(position) = self.mode.take_movement_correction() {
            self.rendered_player_position = vec2(position.x as f32, position.y as f32);
            return;
        }
        let authoritative = self.mode.projection.player_position;
        let target = vec2(authoritative.x as f32, authoritative.y as f32);
        if !self.mode.movement_prediction_active()
            && target.distance(self.rendered_player_position) > input::TELEPORT_SNAP_DISTANCE
        {
            self.rendered_player_position = target;
        }
    }

    fn show_network_notice(&mut self, notice: NetworkNotice) {
        match notice {
            NetworkNotice::Info(message) => self.notifications.info(message),
            NetworkNotice::Success(message) => self.notifications.success(message),
            NetworkNotice::Warning(message) => self.notifications.warning(message),
            NetworkNotice::Danger(message) => self.notifications.danger(message),
        }
    }
}

fn online_gameplay_modal_visible(
    connection: ConnectionState,
    position_authoritative: bool,
) -> bool {
    connection == ConnectionState::Online && position_authoritative
}

fn online_crafting_view(
    connection: ConnectionState,
    position_authoritative: bool,
    crafting: Option<CraftingView>,
) -> Option<CraftingView> {
    online_gameplay_modal_visible(connection, position_authoritative)
        .then_some(crafting)
        .flatten()
}
