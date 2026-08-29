use super::*;

impl Game {
    pub(super) fn read_keyboard_input(&mut self) {
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
}
