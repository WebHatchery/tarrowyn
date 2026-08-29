use super::*;

impl OnlineClient {
    pub fn queue_movement(&mut self, dx: i32, dy: i32) {
        if self.state != ConnectionState::Online {
            return;
        }
        let request_id = self.next_request_id("move");
        if !queue::try_push(
            &mut self.movement_queue,
            MovementIntent { request_id, dx, dy },
        ) {
            self.status_message =
                "The movement queue is full; wait for the road to clear before walking again."
                    .to_owned();
        }
    }

    pub fn queue_move_toward(&mut self, target: TilePos) {
        let dx = target.x - self.projection.player_position.x;
        let dy = target.y - self.projection.player_position.y;
        if dx.abs() >= dy.abs() && dx != 0 {
            self.queue_movement(dx.signum(), 0);
        } else if dy != 0 {
            self.queue_movement(0, dy.signum());
        }
    }

    pub fn queue_chat(&mut self, text: &str) {
        if self.state != ConnectionState::Online {
            return;
        }
        let text: String = text.chars().take(MAX_CHAT_MESSAGE_LENGTH).collect();
        if text.trim().is_empty() {
            return;
        }
        let request_id = self.next_request_id("chat");
        if !queue::try_push(
            &mut self.chat_queue,
            ChatRequest {
                request_id,
                channel: "settlement".to_owned(),
                text,
            },
        ) {
            self.status_message =
                "The chat channel is busy; wait for current messages before trying again."
                    .to_owned();
        }
    }

    pub fn queue_farming(&mut self, action: FarmingAction) {
        if self.state != ConnectionState::Online {
            return;
        }
        let Some(target) = self
            .projection
            .world
            .tiles
            .iter_with_pos()
            .filter(|(pos, tile)| {
                **tile == TileKind::Field
                    && pos.manhattan_distance(&self.projection.player_position) <= 1
            })
            .map(|(pos, _)| pos)
            .next()
        else {
            return;
        };
        let request_id = self.next_request_id("farm");
        let queued = queue::try_push(
            &mut self.farming_queue,
            FarmingRequest {
                request_id: request_id.clone(),
                action,
                position: tarrowyn_protocol::Position {
                    x: target.x,
                    y: target.y,
                },
            },
        );
        if queued {
            self.pending_request_type = Some(format!("farming::{action:?}"));
            self.pending_request_id = Some(request_id);
            self.action_awaiting_confirmation = true;
            self.status_message = "Command sent; waiting for the settlement ledger…".to_owned();
        } else {
            self.status_message =
                "The settlement ledger is busy; wait for current actions before trying again."
                    .to_owned();
        }
    }
}
