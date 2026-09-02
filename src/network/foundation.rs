use super::*;

impl OnlineClient {
    pub fn queue_foundation_interaction(&mut self, interaction_id: &str) -> bool {
        if !self.mutations_ready() || self.pending_foundation.is_some() {
            self.status_message =
                "Wait for the current First Beacon conversation to finish.".to_owned();
            return false;
        }
        let request = FoundationInteractionRequest {
            request_id: self.next_request_id("foundation"),
            interaction_id: interaction_id.to_owned(),
        };
        self.pending_foundation = Some(self.api.post_json("/v1/foundation/interactions", &request));
        self.status_message = "Listening to the First Beacon…".to_owned();
        true
    }

    pub fn foundation_interaction_pending(&self) -> bool {
        self.pending_foundation.is_some()
    }

    pub(super) fn poll_foundation(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let result = self
            .pending_foundation
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = result else { return };
        self.pending_foundation = None;
        match result {
            Ok(response) => {
                self.projection
                    .record_response_version(response.meta.server_tick, response.meta.cursor);
                let detail = format!("{} — {}", response.data.title, response.data.message);
                self.status_message = detail.clone();
                if response.data.accepted {
                    notices.push(NetworkNotice::Info(detail));
                } else {
                    notices.push(NetworkNotice::Warning(detail));
                }
            }
            Err(error) => {
                self.status_message =
                    "The First Beacon conversation could not be confirmed.".to_owned();
                notices.push(NetworkNotice::Warning(short_error(&error)));
            }
        }
    }
}
