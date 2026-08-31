use super::*;

impl Game {
    pub(super) fn save_game(&mut self) {
        let ClientMode::Offline(session) = &self.mode else {
            self.notifications
                .warning("Online world progress is saved on the shared road, not in local slots.");
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
                    .success("The offline first evening is saved on this device.");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.danger(format!("Save failed: {err}")),
        }
    }

    pub(super) fn load_game(&mut self) {
        if !matches!(self.mode, ClientMode::Offline(_)) {
            self.notifications
                .warning("Online world progress is loaded from the shared road, not local slots.");
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

    pub(super) fn delete_save(&mut self) {
        match delete_slot(&self.data.config.game_name, &self.data.config.save_slot) {
            Ok(()) => {
                self.notifications.info("The offline save was cleared.");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.warning(format!("Delete failed: {err}")),
        }
    }

    pub(super) fn refresh_save_state(&mut self) {
        self.save_exists = slot_exists(&self.data.config.game_name, &self.data.config.save_slot);
        self.save_slots = get_save_slots(&self.data.config.game_name);
    }
}
