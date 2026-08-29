use super::*;

pub(super) const MAX_COMMAND_RETRIES: u8 = 3;

impl OnlineClient {
    pub(super) fn poll_movement(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let Some(mut pending) = self.pending_movement.take() else {
            return;
        };
        let Some(result) = pending.pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS) else {
            self.pending_movement = Some(pending);
            return;
        };
        match result {
            Ok(response) => {
                let position = response.data.position;
                self.projection.player_position = TilePos::new(position.x, position.y);
                if response.data.accepted {
                    notices.push(NetworkNotice::Info(
                        "The server accepted that step.".to_owned(),
                    ));
                } else {
                    notices.push(NetworkNotice::Warning(
                        response
                            .data
                            .reason
                            .unwrap_or_else(|| "The server rejected that step.".to_owned()),
                    ));
                }
            }
            Err(error)
                if is_transient_transport_error(&error)
                    && pending.retries < MAX_COMMAND_RETRIES =>
            {
                let retries = pending.retries + 1;
                let request = pending.request;
                self.pending_movement = Some(PendingMovement {
                    pending: self.api.post_json("/v1/movement", &request),
                    request,
                    retries,
                });
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
        let Some(result) = pending.pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS) else {
            self.pending_farming = Some(pending);
            return;
        };
        match result {
            Ok(response) => {
                self.action_awaiting_confirmation = false;
                self.pending_request_type = None;
                self.pending_request_id = None;
                self.state_refresh = 0.0;
                if response.data.accepted {
                    notices.push(NetworkNotice::Success(
                        "The server accepted the farm action.".to_owned(),
                    ));
                } else {
                    notices.push(NetworkNotice::Warning(response.data.reason.unwrap_or_else(
                        || "The server rejected that farm action.".to_owned(),
                    )));
                }
            }
            Err(error)
                if is_transient_transport_error(&error)
                    && pending.retries < MAX_COMMAND_RETRIES =>
            {
                let retries = pending.retries + 1;
                let request = pending.request;
                self.pending_farming = Some(PendingFarming {
                    pending: self.api.post_json("/v1/farming/actions", &request),
                    request,
                    retries,
                });
                notices.push(NetworkNotice::Warning(format!(
                    "The farm action could not be confirmed; retrying the same request ({retries}/{MAX_COMMAND_RETRIES})."
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
        let Some(result) = pending.pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS) else {
            self.pending_chat = Some(pending);
            return;
        };
        match result {
            Ok(response) => {
                if response.data.accepted {
                    if let Some(message) = response.data.message {
                        self.projection.push_chat(message);
                    }
                    notices.push(NetworkNotice::Success(
                        "Message sent to the settlement.".to_owned(),
                    ));
                } else {
                    notices.push(NetworkNotice::Warning(
                        response
                            .data
                            .reason
                            .unwrap_or_else(|| "The server rejected that message.".to_owned()),
                    ));
                }
            }
            Err(error)
                if is_transient_transport_error(&error)
                    && pending.retries < MAX_COMMAND_RETRIES =>
            {
                let retries = pending.retries + 1;
                let request = pending.request;
                self.pending_chat = Some(PendingChat {
                    pending: self.api.post_json("/v1/chat", &request),
                    request,
                    retries,
                });
                notices.push(NetworkNotice::Warning(format!(
                    "The chat message could not be confirmed; retrying the same message ({retries}/{MAX_COMMAND_RETRIES})."
                )));
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }

    pub(super) fn dispatch_requests(&mut self) {
        if self.state != ConnectionState::Online {
            return;
        }
        if self.pending_state.is_none() && self.state_refresh <= 0.0 {
            self.pending_state = Some(self.api.get("/v1/state"));
        }
        if self.pending_ops_health.is_none() && self.state_refresh <= 0.0 {
            self.pending_ops_health = Some(self.api.get("/v1/ops/health"));
        }
        if self.pending_events.is_none() {
            self.pending_events = Some(
                self.api
                    .get(&format!("/v1/events?since={}", self.projection.cursor)),
            );
        }
        if self.pending_movement.is_none() {
            if let Some(request) = self.movement_queue.pop_front() {
                self.pending_movement = Some(PendingMovement {
                    pending: self.api.post_json("/v1/movement", &request),
                    request,
                    retries: 0,
                });
            }
        }
        if self.pending_chat.is_none() {
            if let Some(request) = self.chat_queue.pop_front() {
                self.pending_chat = Some(PendingChat {
                    pending: self.api.post_json("/v1/chat", &request),
                    request,
                    retries: 0,
                });
            }
        }
        if self.pending_farming.is_none() {
            if let Some(request) = self.farming_queue.pop_front() {
                self.pending_request_type = Some(format!("farming::{:?}", request.action));
                self.pending_request_id = Some(request.request_id.clone());
                self.pending_farming = Some(PendingFarming {
                    pending: self.api.post_json("/v1/farming/actions", &request),
                    request,
                    retries: 0,
                });
            }
        }
    }
}
