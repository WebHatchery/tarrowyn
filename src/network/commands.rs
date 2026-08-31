use super::*;
use tarrowyn_protocol::{CropKind, FarmPlot, Position};

pub(super) const MAX_COMMAND_RETRIES: u8 = 3;
pub(super) const COMMAND_RETRY_DELAY_SECONDS: f32 = 1.0;

pub(super) fn movement_success_notice(position: Position) -> String {
    format!("Moved to tile ({}, {}).", position.x, position.y)
}

pub(super) fn farming_success_notice(
    action: FarmingAction,
    plot: Option<FarmPlot>,
    animal: Option<&FarmAnimal>,
) -> String {
    if action == FarmingAction::TendAnimal {
        return animal
            .map(|animal| {
                format!(
                    "Cared for {} • condition {}/{}.",
                    animal.name, animal.condition, animal.max_condition
                )
            })
            .unwrap_or_else(|| "The shared road accepted animal care.".to_owned());
    }
    let Some(plot) = plot else {
        return "The shared road accepted the farm action.".to_owned();
    };
    let Some(crop) = plot.crop else {
        return "The shared road accepted the farm action.".to_owned();
    };
    let crop_name = match crop.kind {
        CropKind::Wheat => "Wheat",
        CropKind::Turnip => "Turnip",
        CropKind::Moonberry => "Moonberry",
    };
    match action {
        FarmingAction::Plant => format!(
            "Planted {crop_name} at plot ({}, {}).",
            plot.position.x, plot.position.y
        ),
        FarmingAction::Tend => format!(
            "Tended {crop_name} at plot ({}, {}); growth stage {}/3.",
            plot.position.x, plot.position.y, crop.stage
        ),
        FarmingAction::Harvest => format!(
            "Harvested {crop_name} from plot ({}, {}).",
            plot.position.x, plot.position.y
        ),
        FarmingAction::TendAnimal => unreachable!("animal care is handled above"),
    }
}

impl OnlineClient {
    pub(super) fn poll_movement(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let Some(mut pending) = self.pending_movement.take() else {
            return;
        };
        pending.retry_timer = (pending.retry_timer - dt.max(0.0)).max(0.0);
        if pending.retry_timer > 0.0 {
            self.pending_movement = Some(pending);
            return;
        }
        if pending.pending.is_none() {
            let request = pending.request.clone();
            pending.pending = Some(self.api.post_json("/v1/movement", &request));
        }
        let Some(result) = pending
            .pending
            .as_mut()
            .and_then(|request| request.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        else {
            self.pending_movement = Some(pending);
            return;
        };
        pending.pending = None;
        match result {
            Ok(response) => {
                let position = response.data.position;
                let cursor = response.meta.cursor.unwrap_or(self.projection.cursor);
                let current = self
                    .projection
                    .response_is_current(response.meta.server_tick, cursor);
                self.projection
                    .record_response_version(response.meta.server_tick, response.meta.cursor);
                if current {
                    self.projection
                        .set_authoritative_player_position(TilePos::new(position.x, position.y));
                }
                if response.data.accepted {
                    notices.push(NetworkNotice::Info(movement_success_notice(position)));
                } else {
                    notices.push(NetworkNotice::Warning(
                        response
                            .data
                            .reason
                            .unwrap_or_else(|| "The shared road rejected that step.".to_owned()),
                    ));
                }
            }
            Err(error)
                if is_transient_transport_error(&error)
                    && pending.retries < MAX_COMMAND_RETRIES =>
            {
                let retries = pending.retries + 1;
                pending.retries = retries;
                pending.retry_timer = COMMAND_RETRY_DELAY_SECONDS;
                self.pending_movement = Some(pending);
                notices.push(NetworkNotice::Warning(format!(
                    "The movement could not be confirmed; retrying the same step ({retries}/{MAX_COMMAND_RETRIES})."
                )));
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }

    pub(super) fn poll_farming(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let Some(mut pending) = self.pending_farming.take() else {
            return;
        };
        pending.retry_timer = (pending.retry_timer - dt.max(0.0)).max(0.0);
        if pending.retry_timer > 0.0 {
            self.pending_farming = Some(pending);
            return;
        }
        if pending.pending.is_none() {
            let request = pending.request.clone();
            pending.pending = Some(self.api.post_json("/v1/farming/actions", &request));
        }
        let Some(result) = pending
            .pending
            .as_mut()
            .and_then(|request| request.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        else {
            self.pending_farming = Some(pending);
            return;
        };
        pending.pending = None;
        match result {
            Ok(response) => {
                self.projection
                    .record_response_version(response.meta.server_tick, response.meta.cursor);
                self.action_awaiting_confirmation = false;
                self.pending_request_type = None;
                self.pending_request_id = None;
                self.state_refresh = 0.0;
                if response.data.accepted {
                    notices.push(NetworkNotice::Success(farming_success_notice(
                        response.data.action,
                        response.data.plot,
                        response.data.animal.as_ref(),
                    )));
                } else {
                    notices.push(NetworkNotice::Warning(response.data.reason.unwrap_or_else(
                        || "The shared road rejected that farm action.".to_owned(),
                    )));
                }
            }
            Err(error)
                if is_transient_transport_error(&error)
                    && pending.retries < MAX_COMMAND_RETRIES =>
            {
                let retries = pending.retries + 1;
                pending.retries = retries;
                pending.retry_timer = COMMAND_RETRY_DELAY_SECONDS;
                self.pending_farming = Some(pending);
                notices.push(NetworkNotice::Warning(format!(
                    "The farm action could not be confirmed; retrying the same action ({retries}/{MAX_COMMAND_RETRIES})."
                )));
            }
            Err(error) => {
                self.action_awaiting_confirmation = false;
                self.connection_failed(error, notices);
            }
        }
    }

    pub(super) fn poll_chat(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let Some(mut pending) = self.pending_chat.take() else {
            return;
        };
        pending.retry_timer = (pending.retry_timer - dt.max(0.0)).max(0.0);
        if pending.retry_timer > 0.0 {
            self.pending_chat = Some(pending);
            return;
        }
        if pending.pending.is_none() {
            let request = pending.request.clone();
            pending.pending = Some(self.api.post_json("/v1/chat", &request));
        }
        let Some(result) = pending
            .pending
            .as_mut()
            .and_then(|request| request.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        else {
            self.pending_chat = Some(pending);
            return;
        };
        pending.pending = None;
        match result {
            Ok(response) => {
                let message_cursor = response.data.message.as_ref().map(|message| message.cursor);
                let response_cursor = response.meta.cursor.or(message_cursor);
                let projection_current = self
                    .projection
                    .accept_response_version(response.meta.server_tick, response_cursor);
                if response.data.accepted {
                    if projection_current {
                        if let Some(message) = response.data.message {
                            self.projection.push_chat(message);
                        }
                    }
                    notices.push(NetworkNotice::Success(
                        "Message sent to the settlement.".to_owned(),
                    ));
                } else {
                    notices.push(NetworkNotice::Warning(response.data.reason.unwrap_or_else(
                        || "The shared road rejected that message.".to_owned(),
                    )));
                }
            }
            Err(error)
                if is_transient_transport_error(&error)
                    && pending.retries < MAX_COMMAND_RETRIES =>
            {
                let retries = pending.retries + 1;
                pending.retries = retries;
                pending.retry_timer = COMMAND_RETRY_DELAY_SECONDS;
                self.pending_chat = Some(pending);
                notices.push(NetworkNotice::Warning(format!(
                    "The chat message could not be confirmed; retrying the same message ({retries}/{MAX_COMMAND_RETRIES})."
                )));
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }

    pub(super) fn dispatch_requests(&mut self) {
        if self.state_reload_pending {
            if self.pending_state.is_none() {
                self.pending_state = Some(self.api.get("/v1/state"));
            }
            return;
        }
        if self.state != ConnectionState::Online {
            return;
        }
        if self.pending_ops_health.is_none() && self.state_refresh <= 0.0 {
            self.pending_ops_health = Some(self.api.get("/v1/ops/health"));
        }
        if self.phase4.auth_refresh_pending() {
            return;
        }
        if self.pending_state.is_none() && self.state_refresh <= 0.0 {
            self.pending_state = Some(self.api.get("/v1/state"));
        }
        if self.pending_events.is_none() {
            self.pending_events = Some(
                self.api
                    .get(&format!("/v1/events?since={}", self.projection.cursor)),
            );
        }
        if self.phase4.mutation_in_flight()
            || self.frontier.command_in_flight()
            || self.pending_trade.is_some()
        {
            return;
        }
        if self.pending_movement.is_none() {
            if let Some(request) = self.movement_queue.pop_front() {
                self.pending_movement = Some(PendingMovement {
                    pending: Some(self.api.post_json("/v1/movement", &request)),
                    request,
                    retries: 0,
                    retry_timer: 0.0,
                });
            }
        }
        if self.pending_chat.is_none() {
            if let Some(request) = self.chat_queue.pop_front() {
                self.pending_chat = Some(PendingChat {
                    pending: Some(self.api.post_json("/v1/chat", &request)),
                    request,
                    retries: 0,
                    retry_timer: 0.0,
                });
            }
        }
        if self.pending_farming.is_none() {
            if let Some(request) = self.farming_queue.pop_front() {
                self.pending_request_type = Some(format!("farming::{:?}", request.action));
                self.pending_request_id = Some(request.request_id.clone());
                self.pending_farming = Some(PendingFarming {
                    pending: Some(self.api.post_json("/v1/farming/actions", &request)),
                    request,
                    retries: 0,
                    retry_timer: 0.0,
                });
            }
        }
    }
}
