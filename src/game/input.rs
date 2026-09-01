use super::*;

pub(super) const MOVEMENT_REPEAT_SECONDS: f32 = 0.08;
pub(super) const RENDERED_MOVEMENT_SPEED: f32 = 9.0;
pub(super) const TELEPORT_SNAP_DISTANCE: f32 = 4.0;

impl Game {
    pub(super) fn read_keyboard_input(&mut self) {
        if self.chronicle_open {
            while let Some(character) = get_char_pressed() {
                if !character.is_control() && self.chronicle_query.chars().count() < 80 {
                    self.chronicle_query.push(character);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                self.chronicle_query.pop();
            }
            if is_key_pressed(KeyCode::Enter) {
                self.events
                    .push(UiAction::Interact("chronicle-search".to_owned()));
            }
            return;
        }
        if keyboard_gameplay_blocked(
            self.account_open,
            self.regional_inspection_open,
            self.skill_selection_open,
            self.school_selection_open,
            self.crafting_open(),
        ) {
            return;
        }
        if self.menu_open || self.art_catalog_open {
            return;
        }
        if self.movement_repeat_timer <= 0.0 {
            let mut moved = false;
            for key in [KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right] {
                if is_key_down(key) {
                    if let Some((dx, dy)) = keyboard_direction(key) {
                        self.mode.queue_movement(dx, dy);
                        moved = true;
                    }
                }
            }
            if moved {
                self.movement_repeat_timer = MOVEMENT_REPEAT_SECONDS;
            }
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
        if is_key_pressed(KeyCode::Equal) {
            self.events.push(UiAction::Zoom(0.05));
        }
        if is_key_pressed(KeyCode::Minus) {
            self.events.push(UiAction::Zoom(-0.05));
        }
    }
}

pub(super) fn keyboard_direction(key: KeyCode) -> Option<(i32, i32)> {
    match key {
        KeyCode::Up => Some((0, -1)),
        KeyCode::Down => Some((0, 1)),
        KeyCode::Left => Some((-1, 0)),
        KeyCode::Right => Some((1, 0)),
        _ => None,
    }
}

impl Game {
    fn crafting_open(&self) -> bool {
        let client = &self.mode;
        super::online_crafting_view(
            client.state,
            client.projection.authoritative_player_position().is_some(),
            client.crafting_view(),
        )
        .is_some()
    }
}

pub(super) fn keyboard_gameplay_blocked(
    account_open: bool,
    regional_inspection_open: bool,
    skill_selection_open: bool,
    school_selection_open: bool,
    crafting_open: bool,
) -> bool {
    account_open
        || regional_inspection_open
        || skill_selection_open
        || school_selection_open
        || crafting_open
}
