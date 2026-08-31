//! Client loop, online authority boundary, offline fixture, and presentation services.

use crate::data::GameData;
use crate::network::{ConnectionState, CraftingView, NetworkNotice, OnlineClient};
use crate::state::{migrate_save_value, GameSession, SaveData};
use crate::ui::{self, UiAction, UiContext};
use macroquad::prelude::*;
use macroquad_toolkit::assets::AssetManager;
use macroquad_toolkit::camera::{Camera2D, Camera2DConfig, CameraBounds};
use macroquad_toolkit::events::EventBus;
use macroquad_toolkit::grid::TilePos;
use macroquad_toolkit::notifications::{
    NotificationAnchor, NotificationManager, NotificationRenderConfig,
};
use macroquad_toolkit::persistence::{
    delete_slot, get_save_slots, load_from_slot_with_migration, save_to_slot_with_version,
    slot_exists,
};
use macroquad_toolkit::prelude::{begin_virtual_ui_frame, dark, end_virtual_ui_frame};
use tarrowyn_protocol::{FarmingAction, TradeAction, TradeBundle, TradeRequest};

#[path = "game/actions.rs"]
mod actions;
#[path = "game/input.rs"]
mod input;
#[path = "game/offline.rs"]
mod offline;

#[cfg(test)]
#[path = "game/tests.rs"]
mod tests;

enum ClientMode {
    Online(Box<OnlineClient>),
    Offline(GameSession),
}

pub struct Game {
    data: GameData,
    mode: ClientMode,
    server_url: String,
    assets: AssetManager,
    notifications: NotificationManager,
    camera: Camera2D,
    events: EventBus<UiAction>,
    save_exists: bool,
    save_slots: Vec<String>,
    chat_draft: String,
    regional_inspection_open: bool,
    skill_selection_open: bool,
    school_selection_open: bool,
    chronicle_open: bool,
    chronicle_query: String,
    account_open: bool,
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

        let offline = std::env::var("TARROWYN_OFFLINE")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let server_url = data.config.connection_url();
        let mode = if offline {
            ClientMode::Offline(GameSession::new(&data.config))
        } else {
            ClientMode::Online(Box::new(OnlineClient::new(&server_url, &data.config)))
        };
        let player_position = match &mode {
            ClientMode::Online(client) => client.projection.player_position,
            ClientMode::Offline(session) => session.player.position,
        };
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

        let mut game = Self {
            data,
            mode,
            server_url,
            assets,
            notifications: NotificationManager::new(),
            camera,
            events: EventBus::new(),
            save_exists: false,
            save_slots: Vec::new(),
            chat_draft: String::new(),
            regional_inspection_open: false,
            skill_selection_open: false,
            school_selection_open: false,
            chronicle_open: false,
            chronicle_query: String::new(),
            account_open: false,
        };
        game.refresh_save_state();
        game
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);
        match &mut self.mode {
            ClientMode::Offline(session) => {
                if session.update_clock(&self.data.config, dt) {
                    self.notifications.info(format!(
                        "Day {} begins; the fixture settlement stirs.",
                        session.day
                    ));
                }
            }
            ClientMode::Online(client) => {
                for notice in client.update(dt) {
                    self.show_network_notice(notice);
                }
            }
        }

        self.read_keyboard_input();
        let actions: Vec<UiAction> = self.events.drain().collect();
        for action in actions {
            self.apply_action(action);
        }
    }

    pub fn draw(&mut self) {
        let night = match &self.mode {
            ClientMode::Online(client) => client.projection.is_night(),
            ClientMode::Offline(session) => session.is_night(&self.data.config),
        };
        clear_background(if night {
            Color::new(0.025, 0.045, 0.09, 1.0)
        } else {
            dark::BACKGROUND
        });

        let virtual_ui = begin_virtual_ui_frame(ui::LOGICAL_WIDTH, ui::LOGICAL_HEIGHT);
        let regional_inspection = match &self.mode {
            ClientMode::Online(client)
                if self.regional_inspection_open
                    && online_gameplay_modal_visible(
                        client.state,
                        client.projection.authoritative_player_position().is_some(),
                    ) =>
            {
                Some(client.phase5_inspection())
            }
            _ => None,
        };
        let actions = match &self.mode {
            ClientMode::Online(client) => {
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
                    data: &self.data,
                    world: &client.projection.world,
                    player_position: client.projection.player_position,
                    day: client.projection.day,
                    calendar_season: client.phase5_season(),
                    clock_minutes: client.projection.clock_minutes(),
                    time_of_day: client.projection.time_of_day(),
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
                    chat: &client.projection.chat,
                    tavern_notices: &client.projection.feed.notices,
                    tavern_rumours: &client.projection.feed.rumours,
                    chat_draft: &self.chat_draft,
                    server_tick: client.projection.server_tick,
                    connection: client.state,
                    status_message: &client.status_message,
                    expedition: client.projection.expedition.as_ref(),
                    expedition_requirements: client.projection.expedition_requirements,
                    identity_name: identity,
                    offline: false,
                    save_exists: false,
                    save_slots: &self.save_slots,
                    loaded_assets: self.assets.len(),
                    camera_zoom: self.camera.zoom,
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
                    opportunities: &client.projection.opportunities,
                    phase4_summary: &client.phase4_summary(),
                    phase5_summary: &client.phase5_summary(),
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
            }
            ClientMode::Offline(session) => {
                let stats = format!(
                    "Gold {}  Skill {}  Reputation {}\n{}  Total crops {}  Ready {}",
                    session.player.gold,
                    session.player.skill,
                    session.player.reputation,
                    session.format_inventory(),
                    session.player.inventory.total_crops(),
                    session.crops_ready()
                );
                ui::draw_game_ui(UiContext {
                    data: &self.data,
                    world: &session.world,
                    player_position: session.player.position,
                    day: session.day,
                    calendar_season: None,
                    clock_minutes: session.clock_minutes(&self.data.config),
                    time_of_day: session.time_of_day(&self.data.config),
                    night: session.is_night(&self.data.config),
                    stats: &stats,
                    own_account_id: None,
                    player_position_authoritative: false,
                    remote_players: &[],
                    farm_animals: &[],
                    trades: &[],
                    trade_pending: false,
                    farming_pending: false,
                    chat: &[],
                    tavern_notices: &[],
                    tavern_rumours: &[],
                    chat_draft: &self.chat_draft,
                    server_tick: 0,
                    connection: ConnectionState::Offline,
                    status_message: session.last_activity(),
                    expedition: None,
                    expedition_requirements: tarrowyn_protocol::ExpeditionRequirements::default(),
                    identity_name: Some("Local first-evening fixture"),
                    offline: true,
                    save_exists: self.save_exists,
                    save_slots: &self.save_slots,
                    loaded_assets: self.assets.len(),
                    camera_zoom: self.camera.zoom,
                    wilderness: None,
                    regional_region: None,
                    regional_inspection: None,
                    regional_event_choices: &[],
                    skills: &[],
                    skill_selection_open: false,
                    school_selection_open: false,
                    chronicle_open: false,
                    chronicle: &[],
                    chronicle_summary: None,
                    chronicle_query: &self.chronicle_query,
                    chronicle_search: &[],
                    chronicle_search_summary: None,
                    chronicle_search_query: None,
                    chronicle_search_next_cursor: None,
                    chronicle_search_pending: false,
                    opportunities: &[],
                    phase4_summary:
                        "The wider regional ledgers are available only on the shared road.",
                    phase5_summary:
                        "Regional map and linked account are available on the shared road.",
                    account_deletion_armed: false,
                    account_deletion_available: false,
                    account_link_available: false,
                    account_open: false,
                    account_summary: "Account details belong to the shared road.",
                    identity_pending: false,
                    report_pending: false,
                    crafting: None,
                    combat: None,
                    storm_magic_unlocked: false,
                    skill_pending: false,
                    knowledge_pending: false,
                    order_pending: false,
                    combat_pending: false,
                    contract_pending: false,
                    expedition_pending: false,
                    frontier_combat_pending: false,
                    frontier_claim_pending: false,
                    knocked_out: false,
                    recovery_pending: false,
                    has_open_market_order: false,
                    market_pending: false,
                    event_pending: false,
                    route_pending: false,
                    governance_pending: false,
                    can_abandon_claim: false,
                    can_transfer_claim: false,
                    claim_pending: false,
                    knowledge_label: "Knowledge",
                    travel_label: "Travel",
                    can_travel: false,
                    can_recover_travel: false,
                    travel_pending: false,
                    ui: &virtual_ui,
                })
            }
        };
        end_virtual_ui_frame();

        for action in actions {
            self.events.push(action);
        }
        self.notifications
            .draw_with_config(&NotificationRenderConfig {
                anchor: NotificationAnchor::BottomRight,
                margin: 22.0,
                width: 360.0,
                ..Default::default()
            });
    }

    fn queue_movement(&mut self, dx: i32, dy: i32) {
        let position = match &mut self.mode {
            ClientMode::Online(client) => {
                client.queue_movement(dx, dy);
                None
            }
            ClientMode::Offline(session) => {
                if session.move_player(dx, dy) {
                    Some(session.player.position)
                } else {
                    self.notifications
                        .warning("The river or map edge blocks the way.");
                    None
                }
            }
        };
        if let Some(position) = position {
            self.sync_camera(position);
        }
    }

    fn move_toward(&mut self, target: TilePos) {
        let position = match &mut self.mode {
            ClientMode::Online(client) => {
                client.queue_move_toward(target);
                None
            }
            ClientMode::Offline(session) => {
                if target != session.player.position && !session.move_toward(target) {
                    self.notifications
                        .warning("Water and the map edge block that route.");
                    None
                } else {
                    Some(session.player.position)
                }
            }
        };
        if let Some(position) = position {
            self.sync_camera(position);
        }
    }

    fn sync_camera(&mut self, position: TilePos) {
        self.camera.target = vec2(position.x as f32, position.y as f32);
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
