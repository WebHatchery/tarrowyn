use super::*;

impl OnlineClient {
    pub fn queue_movement(&mut self, dx: i32, dy: i32) {
        if self.state != ConnectionState::Online {
            return;
        }
        if self
            .projection
            .player
            .as_ref()
            .is_some_and(|player| player.knocked_out)
        {
            self.status_message = "Choose a recovery prompt before walking.".to_owned();
            return;
        }
        if self.projection.authoritative_player_position().is_none() {
            self.status_message =
                "Your position is still loading; wait for the authoritative road snapshot."
                    .to_owned();
            return;
        }
        if self.phase4.regional_movement_locked() {
            self.status_message =
                "Your regional journey is underway; tap Travel or Recover before walking."
                    .to_owned();
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
        if self.farming_pending() && self.farming_queue.len() < queue::MAX_PENDING_COMMANDS {
            self.status_message =
                "That farm action is already waiting for the ledger; wait for its result."
                    .to_owned();
            return;
        }
        let target = if action == FarmingAction::TendAnimal {
            self.projection
                .animals
                .iter()
                .find(|animal| {
                    animal
                        .position
                        .manhattan_distance(tarrowyn_protocol::Position {
                            x: self.projection.player_position.x,
                            y: self.projection.player_position.y,
                        })
                        <= 1
                })
                .map(|animal| animal.position)
        } else {
            self.projection
                .world
                .tiles
                .iter_with_pos()
                .filter(|(pos, tile)| {
                    if **tile != TileKind::Field
                        || pos.manhattan_distance(&self.projection.player_position) > 1
                    {
                        return false;
                    }
                    let crop = self.projection.world.crops.get(*pos).copied().flatten();
                    match action {
                        FarmingAction::Plant => crop.is_none(),
                        FarmingAction::Tend => crop.is_some_and(|crop| !crop.mature()),
                        FarmingAction::Harvest => crop.is_some_and(|crop| crop.mature()),
                        FarmingAction::TendAnimal => false,
                    }
                })
                .min_by_key(|(pos, _)| pos.manhattan_distance(&self.projection.player_position))
                .map(|(pos, _)| tarrowyn_protocol::Position { x: pos.x, y: pos.y })
        };
        let Some(target) = target else {
            self.status_message = match action {
                FarmingAction::Plant => {
                    "Stand beside an empty shared field plot before planting.".to_owned()
                }
                FarmingAction::Tend => "Stand beside a growing crop before tending it.".to_owned(),
                FarmingAction::Harvest => {
                    "Stand beside a mature crop before harvesting it.".to_owned()
                }
                FarmingAction::TendAnimal => {
                    "Stand beside Bellweather near the shared fields before caring for it."
                        .to_owned()
                }
            };
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

    pub(crate) fn farming_pending(&self) -> bool {
        self.pending_farming.is_some() || !self.farming_queue.is_empty()
    }
}
