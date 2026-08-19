//! Client loop, intent application, toolkit services, and local persistence.

use crate::data::GameData;
use crate::state::{migrate_save_value, GameSession, SaveData};
use crate::ui::{self, UiAction, UiContext};
use macroquad::prelude::*;
use macroquad_toolkit::assets::AssetManager;
use macroquad_toolkit::camera::{Camera2D, Camera2DConfig, CameraBounds};
use macroquad_toolkit::events::EventBus;
use macroquad_toolkit::notifications::{
    NotificationAnchor, NotificationManager, NotificationRenderConfig,
};
use macroquad_toolkit::persistence::{
    delete_slot, get_save_slots, load_from_slot_with_migration, save_to_slot_with_version,
    slot_exists,
};
use macroquad_toolkit::prelude::{begin_virtual_ui_frame, dark, end_virtual_ui_frame};

pub struct Game {
    data: GameData,
    session: GameSession,
    assets: AssetManager,
    notifications: NotificationManager,
    camera: Camera2D,
    events: EventBus<UiAction>,
    save_exists: bool,
    save_slots: Vec<String>,
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

        let session = GameSession::new(&data.config);
        let camera = Camera2D::with_config(
            vec2(
                session.player.position.x as f32,
                session.player.position.y as f32,
            ),
            1.0,
            Camera2DConfig {
                min_zoom: 0.9,
                max_zoom: 1.15,
                drag_button: None,
                keyboard_pan_enabled: false,
                mouse_drag_enabled: false,
                mouse_wheel_zoom_enabled: false,
                bounds: Some(CameraBounds::new(vec2(0.0, 0.0), vec2(17.0, 10.0))),
                ..Default::default()
            },
        );

        let notifications = NotificationManager::new();

        let mut game = Self {
            data,
            session,
            assets,
            notifications,
            camera,
            events: EventBus::new(),
            save_exists: false,
            save_slots: Vec::new(),
        };
        game.refresh_save_state();
        game
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);
        if self.session.update_clock(&self.data.config, dt) {
            self.notifications.info(format!(
                "Day {} begins; the settlement stirs.",
                self.session.day
            ));
        }

        self.read_keyboard_intents();
        let actions: Vec<UiAction> = self.events.drain().collect();
        for action in actions {
            self.apply_action(action);
        }
    }

    pub fn draw(&mut self) {
        clear_background(if self.session.is_night(&self.data.config) {
            Color::new(0.025, 0.045, 0.09, 1.0)
        } else {
            dark::BACKGROUND
        });

        let virtual_ui = begin_virtual_ui_frame(ui::LOGICAL_WIDTH, ui::LOGICAL_HEIGHT);
        let context = UiContext {
            data: &self.data,
            session: &self.session,
            save_exists: self.save_exists,
            save_slots: &self.save_slots,
            loaded_assets: self.assets.len(),
            camera_zoom: self.camera.zoom,
            ui: &virtual_ui,
        };
        let actions = ui::draw_game_ui(context);
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

    fn read_keyboard_intents(&mut self) {
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
        if is_key_pressed(KeyCode::F5) {
            self.events.push(UiAction::Save);
        }
        if is_key_pressed(KeyCode::F9) {
            self.events.push(UiAction::Load);
        }
        if is_key_pressed(KeyCode::R) {
            self.events.push(UiAction::NewEvening);
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
            UiAction::NewEvening => {
                self.session = GameSession::new(&self.data.config);
                self.sync_camera();
                self.notifications
                    .info("A fresh first evening begins at the Hearth.");
            }
            UiAction::Save => self.save_game(),
            UiAction::Load => self.load_game(),
            UiAction::DeleteSave => self.delete_save(),
            UiAction::Move(dx, dy) => self.try_move(dx, dy),
            UiAction::MoveTo(tile) => {
                if tile != self.session.player.position && !self.session.move_toward(tile) {
                    self.notifications
                        .warning("Water and the map edge block that route.");
                } else {
                    self.sync_camera();
                }
            }
            UiAction::Interact(id) => self.interact(&id),
            UiAction::Zoom(delta) => {
                self.camera.zoom = (self.camera.zoom + delta).clamp(0.9, 1.15);
            }
        }
    }

    fn try_move(&mut self, dx: i32, dy: i32) {
        if self.session.move_player(dx, dy) {
            self.sync_camera();
        } else {
            self.notifications
                .warning("The river or map edge blocks the way.");
        }
    }

    fn interact(&mut self, id: &str) {
        let Some(action) = self.data.actions.get(id) else {
            self.notifications.warning(format!("Unknown action: {id}"));
            return;
        };
        let result = self.session.apply_action(action);
        if result.success {
            self.notifications.success(result.message);
        } else {
            self.notifications.warning(result.message);
        }
    }

    fn save_game(&mut self) {
        let save = self.session.to_save(&self.data.config.version);
        match save_to_slot_with_version(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &save,
            &self.data.config.version,
        ) {
            Ok(()) => {
                self.notifications
                    .success("The evening is written to local memory.");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.danger(format!("Save failed: {err}")),
        }
    }

    fn load_game(&mut self) {
        let loaded: Result<SaveData, String> = load_from_slot_with_migration(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &self.data.config.version,
            |version, value| migrate_save_value(version, value, &self.data.config),
        );
        match loaded {
            Ok(save) => {
                self.session = GameSession::from_save(save);
                self.sync_camera();
                self.notifications
                    .success("The local chronicle is restored.");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.warning(format!("Load failed: {err}")),
        }
    }

    fn delete_save(&mut self) {
        match delete_slot(&self.data.config.game_name, &self.data.config.save_slot) {
            Ok(()) => {
                self.notifications.info("The local save was cleared.");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.danger(format!("Delete failed: {err}")),
        }
    }

    fn refresh_save_state(&mut self) {
        self.save_exists = slot_exists(&self.data.config.game_name, &self.data.config.save_slot);
        self.save_slots = get_save_slots(&self.data.config.game_name);
    }

    fn sync_camera(&mut self) {
        self.camera.target = vec2(
            self.session.player.position.x as f32,
            self.session.player.position.y as f32,
        );
    }
}
