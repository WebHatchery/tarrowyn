use super::*;
use crate::state::TileKind;

pub(super) const PLAYER_MOVEMENT_SPEED: f32 = 4.0;
pub(super) const TELEPORT_SNAP_DISTANCE: f32 = 4.0;
const MAX_MOVEMENT_SUBSTEP: f32 = 0.2;

impl Game {
    pub(super) fn read_keyboard_input(&mut self, dt: f32) {
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
        let direction = keyboard_movement_direction(|key| is_key_down(key));
        if direction.length_squared() > f32::EPSILON {
            self.advance_player_movement(direction, dt);
            self.movement_advanced_this_frame = true;
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

impl Game {
    pub(super) fn advance_player_movement(&mut self, direction: Vec2, dt: f32) {
        let direction = normalized_movement_direction(direction);
        if direction == Vec2::ZERO || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let mut remaining = PLAYER_MOVEMENT_SPEED * dt;
        while remaining > 0.0 {
            let distance = remaining.min(MAX_MOVEMENT_SUBSTEP);
            let _ = self.advance_movement_axis(direction.x * distance, true);
            self.advance_movement_axis(direction.y * distance, false);
            remaining -= distance;
        }
    }

    fn advance_movement_axis(&mut self, amount: f32, horizontal: bool) -> bool {
        if amount.abs() <= f32::EPSILON {
            return true;
        }
        let mut candidate = self.rendered_player_position;
        if horizontal {
            candidate.x += amount;
        } else {
            candidate.y += amount;
        }
        let current_tile = rendered_tile(self.rendered_player_position);
        let candidate_tile = rendered_tile(candidate);
        if candidate_tile == current_tile {
            self.rendered_player_position = candidate;
            return true;
        }
        if !self
            .mode
            .projection
            .world
            .tiles
            .get(candidate_tile)
            .is_some_and(|tile| *tile != TileKind::Water)
        {
            return false;
        }
        let dx = candidate_tile.x - current_tile.x;
        let dy = candidate_tile.y - current_tile.y;
        if self.mode.queue_movement(dx, dy) {
            self.rendered_player_position = candidate;
            true
        } else {
            false
        }
    }
}

pub(super) fn keyboard_movement_direction(mut down: impl FnMut(KeyCode) -> bool) -> Vec2 {
    let x = i32::from(down(KeyCode::Right)) - i32::from(down(KeyCode::Left));
    let y = i32::from(down(KeyCode::Down)) - i32::from(down(KeyCode::Up));
    vec2(x as f32, y as f32)
}

pub(super) fn normalized_movement_direction(direction: Vec2) -> Vec2 {
    if !direction.is_finite() || direction.length_squared() <= f32::EPSILON {
        Vec2::ZERO
    } else {
        direction.normalize()
    }
}

pub(super) fn rendered_tile(position: Vec2) -> TilePos {
    TilePos::new(position.x.round() as i32, position.y.round() as i32)
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
