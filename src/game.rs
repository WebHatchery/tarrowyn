//! Client loop, online authority boundary, offline fixture, and presentation services.

use crate::data::GameData;
use crate::network::{ConnectionState, NetworkNotice, OnlineClient};
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
        let server_url = std::env::var("TARROWYN_SERVER_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8787".to_owned());
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
                if self.regional_inspection_open && client.state == ConnectionState::Online =>
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
                    format!("{stats}\nKNOCKED OUT • choose Recover below")
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
                    remote_players: &client.projection.players,
                    farm_animals: &client.projection.animals,
                    trades: &client.projection.trades,
                    chat: &client.projection.chat,
                    chat_draft: &self.chat_draft,
                    server_tick: client.projection.server_tick,
                    connection: client.state,
                    status_message: &client.status_message,
                    identity_name: identity,
                    offline: false,
                    save_exists: false,
                    save_slots: &self.save_slots,
                    loaded_assets: self.assets.len(),
                    camera_zoom: self.camera.zoom,
                    wilderness: client.projection.wilderness.as_ref(),
                    regional_region: client.phase5_region(),
                    regional_inspection: regional_inspection.as_deref(),
                    chronicle: &client.projection.chronicle,
                    chronicle_summary: client.projection.chronicle_summary.as_ref(),
                    opportunities: &client.projection.opportunities,
                    phase4_summary: &client.phase4_summary(),
                    phase5_summary: &client.phase5_summary(),
                    account_deletion_armed: client.account_deletion_armed(),
                    crafting: client.crafting_view(),
                    combat: client.combat_state(),
                    storm_magic_unlocked: client.storm_magic_unlocked(),
                    knocked_out: client
                        .projection
                        .player
                        .as_ref()
                        .is_some_and(|player| player.knocked_out),
                    has_open_market_order: client.has_open_market_order(),
                    can_abandon_claim: client.can_abandon_claim(),
                    can_transfer_claim: client.can_transfer_claim()
                        && client.projection.players.iter().any(|player| {
                            Some(player.account_id.as_str()) != own_account_id
                                && !player.stale(client.projection.server_tick)
                                && player.position == client.projection.player_position
                        }),
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
                    remote_players: &[],
                    farm_animals: &[],
                    trades: &[],
                    chat: &[],
                    chat_draft: &self.chat_draft,
                    server_tick: 0,
                    connection: ConnectionState::Offline,
                    status_message: session.last_activity(),
                    identity_name: Some("Local first-evening fixture"),
                    offline: true,
                    save_exists: self.save_exists,
                    save_slots: &self.save_slots,
                    loaded_assets: self.assets.len(),
                    camera_zoom: self.camera.zoom,
                    wilderness: None,
                    regional_region: None,
                    regional_inspection: None,
                    chronicle: &[],
                    chronicle_summary: None,
                    opportunities: &[],
                    phase4_summary: "Phase 4 ledgers are available only on the shared road.",
                    phase5_summary:
                        "Regional map and production account are available on the shared road.",
                    account_deletion_armed: false,
                    crafting: None,
                    combat: None,
                    storm_magic_unlocked: false,
                    knocked_out: false,
                    has_open_market_order: false,
                    can_abandon_claim: false,
                    can_transfer_claim: false,
                    knowledge_label: "Knowledge",
                    travel_label: "Travel",
                    can_travel: false,
                    can_recover_travel: false,
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

    fn read_keyboard_input(&mut self) {
        let movement = if is_key_pressed(KeyCode::W) || is_key_pressed(KeyCode::Up) {
            Some((0, -1))
        } else if is_key_pressed(KeyCode::D) || is_key_pressed(KeyCode::Right) {
            Some((1, 0))
        } else if is_key_pressed(KeyCode::S) || is_key_pressed(KeyCode::Down) {
            Some((0, 1))
        } else if is_key_pressed(KeyCode::A) || is_key_pressed(KeyCode::Left) {
            Some((-1, 0))
        } else {
            None
        };
        if let Some((dx, dy)) = movement {
            self.events.push(UiAction::Move(dx, dy));
        }
        while let Some(character) = get_char_pressed() {
            if !character.is_control() && self.chat_draft.chars().count() < 160 {
                self.chat_draft.push(character);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            self.chat_draft.pop();
        }
        if is_key_pressed(KeyCode::Enter) && !self.chat_draft.trim().is_empty() {
            self.events.push(UiAction::SendChat);
        }
        if is_key_pressed(KeyCode::F5) {
            self.events.push(UiAction::Save);
        }
        if is_key_pressed(KeyCode::F9) {
            self.events.push(UiAction::Load);
        }
        if is_key_pressed(KeyCode::Equal) {
            self.events.push(UiAction::Zoom(0.05));
        }
        if is_key_pressed(KeyCode::Minus) {
            self.events.push(UiAction::Zoom(-0.05));
        }
    }

    fn apply_action(&mut self, action: UiAction) {
        match action {
            UiAction::UseOffline => {
                self.mode = ClientMode::Offline(GameSession::new(&self.data.config));
                self.regional_inspection_open = false;
                self.chat_draft.clear();
                self.sync_camera(TilePos::new(8, 6));
                self.notifications
                    .info("Offline fixture enabled; no online state is used.");
            }
            UiAction::UseOnline => {
                self.mode = ClientMode::Online(Box::new(OnlineClient::new(
                    &self.server_url,
                    &self.data.config,
                )));
                self.regional_inspection_open = false;
                self.chat_draft.clear();
                self.sync_camera(TilePos::new(8, 6));
                self.notifications.info("Connecting to the shared road…");
            }
            UiAction::Reconnect => match &mut self.mode {
                ClientMode::Online(client) => {
                    self.regional_inspection_open = false;
                    if !client.reconnect() {
                        self.notifications
                            .warning("Wait for the reconnect cooldown to finish.");
                    }
                }
                ClientMode::Offline(_) => {
                    self.mode = ClientMode::Online(Box::new(OnlineClient::new(
                        &self.server_url,
                        &self.data.config,
                    )));
                    self.regional_inspection_open = false;
                    self.sync_camera(TilePos::new(8, 6));
                    self.notifications.info("Connecting to the shared road…");
                }
            },
            UiAction::NewEvening => match &mut self.mode {
                ClientMode::Offline(session) => {
                    *session = GameSession::new(&self.data.config);
                    self.sync_camera(TilePos::new(8, 6));
                    self.notifications
                        .info("A fresh offline first evening begins at the Hearth.");
                }
                ClientMode::Online(_) => self
                    .notifications
                    .warning("The server owns the online world; use Reconnect to recover it."),
            },
            UiAction::Save => self.save_game(),
            UiAction::Load => self.load_game(),
            UiAction::DeleteSave => self.delete_save(),
            UiAction::Move(dx, dy) => self.queue_movement(dx, dy),
            UiAction::MoveTo(tile) => self.move_toward(tile),
            UiAction::Interact(id) => self.interact(&id),
            UiAction::SendChat => {
                let text = self.chat_draft.trim().to_owned();
                if let ClientMode::Online(client) = &mut self.mode {
                    client.queue_chat(&text);
                    self.chat_draft.clear();
                }
            }
            UiAction::QuickChat(text) => {
                if let ClientMode::Online(client) = &mut self.mode {
                    client.queue_chat(&text);
                }
            }
            UiAction::Zoom(delta) => {
                self.camera.zoom = (self.camera.zoom + delta).clamp(0.9, 1.15);
            }
        }
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

    fn interact(&mut self, id: &str) {
        if id == "region-details" {
            if matches!(&self.mode, ClientMode::Online(_)) {
                self.regional_inspection_open = !self.regional_inspection_open;
            }
            return;
        }
        if matches!(id, "logout" | "delete-account") {
            self.regional_inspection_open = false;
        }
        if let ClientMode::Online(client) = &mut self.mode {
            match id {
                "plant" => client.queue_farming(FarmingAction::Plant),
                "tend" => client.queue_farming(FarmingAction::Tend),
                "harvest" => client.queue_farming(FarmingAction::Harvest),
                "animal" => client.queue_farming(FarmingAction::TendAnimal),
                "listen" => client.refresh_tavern(),
                "trade" => {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let pending_trade_id = own.and_then(|account_id| {
                        client
                            .pending_trade_for(account_id)
                            .map(|trade| trade.trade_id.clone())
                    });
                    if let Some(trade_id) = pending_trade_id {
                        client.queue_trade(TradeRequest {
                            request_id: String::new(),
                            action: TradeAction::Review,
                            trade_id: Some(trade_id),
                            recipient_account_id: None,
                            offer: None,
                            request: None,
                        });
                    } else {
                        let target = client.projection.players.iter().find(|player| {
                            Some(player.account_id.as_str()) != own
                                && !player.stale(client.projection.server_tick)
                        });
                        if let Some(target) = target {
                            client.queue_trade(TradeRequest {
                                request_id: String::new(),
                                action: TradeAction::Create,
                                trade_id: None,
                                recipient_account_id: Some(target.account_id.clone()),
                                offer: Some(TradeBundle {
                                    seeds: 1,
                                    ..TradeBundle::default()
                                }),
                                request: Some(TradeBundle {
                                    gold: 2,
                                    ..TradeBundle::default()
                                }),
                            });
                        } else {
                            self.notifications
                                .warning("Another player must be present before offering a seed.");
                        }
                    }
                }
                "accept-trade" => {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let incoming_trade_id = own.and_then(|account_id| {
                        client
                            .incoming_trade_for(account_id)
                            .map(|trade| trade.trade_id.clone())
                    });
                    if let Some(trade_id) = incoming_trade_id {
                        client.queue_trade(TradeRequest {
                            request_id: String::new(),
                            action: TradeAction::Accept,
                            trade_id: Some(trade_id),
                            recipient_account_id: None,
                            offer: None,
                            request: None,
                        });
                    } else {
                        self.notifications
                            .warning("No pending trade is waiting for this character.");
                    }
                }
                "cancel-trade" => {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let pending_trade_id = own.and_then(|account_id| {
                        client
                            .pending_trade_for(account_id)
                            .map(|trade| trade.trade_id.clone())
                    });
                    if let Some(trade_id) = pending_trade_id {
                        client.queue_trade(TradeRequest {
                            request_id: String::new(),
                            action: TradeAction::Cancel,
                            trade_id: Some(trade_id),
                            recipient_account_id: None,
                            offer: None,
                            request: None,
                        });
                    } else {
                        self.notifications
                            .warning("No pending trade is waiting to be cancelled.");
                    }
                }
                "contract" => client.queue_contract_cycle(),
                "strike" => client.queue_combat(
                    tarrowyn_protocol::CombatAction::Strike,
                    tarrowyn_protocol::WeaponKind::IronSword,
                ),
                "recover" => client.queue_recovery(tarrowyn_protocol::RecoveryChoice::AskRescuer),
                "claim" => client.queue_claim_cycle(),
                "abandon-claim" => client
                    .queue_claim_action(tarrowyn_protocol::ClaimLifecycleAction::Abandon, None),
                "transfer-claim" => {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let target = client
                        .projection
                        .players
                        .iter()
                        .find(|player| {
                            Some(player.account_id.as_str()) != own
                                && !player.stale(client.projection.server_tick)
                                && player.position == client.projection.player_position
                        })
                        .map(|player| player.account_id.clone());
                    if let Some(target) = target {
                        client.queue_claim_action(
                            tarrowyn_protocol::ClaimLifecycleAction::Transfer,
                            Some(target),
                        );
                    } else {
                        self.notifications.warning(
                            "Stand beside another recognised player before transferring a lease.",
                        );
                    }
                }
                "expedition" => client.queue_expedition_cycle(),
                "chronicle" => client.refresh_tavern(),
                "report" => client.queue_report(),
                "practice" => client.queue_phase4("practice"),
                "knowledge" => {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let target = client
                        .projection
                        .players
                        .iter()
                        .find(|player| {
                            Some(player.account_id.as_str()) != own
                                && !player.stale(client.projection.server_tick)
                                && player.position == client.projection.player_position
                        })
                        .map(|player| player.account_id.clone());
                    client.queue_knowledge_cycle(target);
                }
                "town-hall" | "tax-rate" | "registry" | "order" | "households" | "local-fight"
                | "technique" | "guard" | "item" | "reposition" | "spell" => {
                    client.queue_phase4(id)
                }
                "crafting-timing" => client.queue_crafting_timing(),
                "school" => {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let target = client
                        .projection
                        .players
                        .iter()
                        .find(|player| {
                            Some(player.account_id.as_str()) != own
                                && !player.stale(client.projection.server_tick)
                                && player.position == client.projection.player_position
                        })
                        .map(|player| player.account_id.clone());
                    match target {
                        Some(target) if client.queue_skill_teach(&target) => {}
                        Some(_) => self.notifications.warning(
                            "No mastered discipline is ready, or the school ledger is busy.",
                        ),
                        None => self
                            .notifications
                            .warning("Another nearby player must be present for a school lesson."),
                    }
                }
                "travel" | "recover-travel" | "route-repair" | "route-escort" | "route-improve"
                | "market-region" | "region-event" | "cancel-market" | "account" | "logout"
                | "delete-account" => client.queue_phase5(id),
                _ => self.notifications.warning(format!("Unknown action: {id}")),
            }
            return;
        }
        let ClientMode::Offline(session) = &mut self.mode else {
            return;
        };
        let Some(action) = self.data.actions.get(id) else {
            self.notifications.warning(format!("Unknown action: {id}"));
            return;
        };
        let result = session.apply_action(action);
        if result.success {
            self.notifications.success(result.message);
        } else {
            self.notifications.warning(result.message);
        }
    }

    fn save_game(&mut self) {
        let ClientMode::Offline(session) = &self.mode else {
            self.notifications
                .warning("Online world state is not stored in local save slots.");
            return;
        };
        let save = session.to_save(&self.data.config.version);
        match save_to_slot_with_version(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &save,
            &self.data.config.version,
        ) {
            Ok(()) => {
                self.notifications
                    .success("The offline fixture is written to local memory.");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.danger(format!("Save failed: {err}")),
        }
    }

    fn load_game(&mut self) {
        if !matches!(self.mode, ClientMode::Offline(_)) {
            self.notifications
                .warning("Online world state is not loaded from local save slots.");
            return;
        }
        let loaded: Result<SaveData, String> = load_from_slot_with_migration(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &self.data.config.version,
            |version, value| migrate_save_value(version, value, &self.data.config),
        );
        match loaded {
            Ok(save) => {
                let position = save.player.position;
                self.mode = ClientMode::Offline(GameSession::from_save(save));
                self.sync_camera(position);
                self.notifications
                    .success("The offline chronicle is restored.");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.warning(format!("Load failed: {err}")),
        }
    }

    fn delete_save(&mut self) {
        match delete_slot(&self.data.config.game_name, &self.data.config.save_slot) {
            Ok(()) => {
                self.notifications.info("The offline save was cleared.");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.danger(format!("Delete failed: {err}")),
        }
    }

    fn refresh_save_state(&mut self) {
        self.save_exists = slot_exists(&self.data.config.game_name, &self.data.config.save_slot);
        self.save_slots = get_save_slots(&self.data.config.game_name);
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
