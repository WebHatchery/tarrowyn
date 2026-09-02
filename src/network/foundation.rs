use super::*;

impl OnlineClient {
    pub fn queue_foundation_interaction(&mut self, interaction_id: &str) -> bool {
        if !self.mutations_ready()
            || self.pending_foundation.is_some()
            || self.pending_foundation_resource.is_some()
        {
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
        self.pending_foundation.is_some() || self.pending_foundation_resource.is_some()
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

    pub fn queue_foundation_resource(
        &mut self,
        node_id: &str,
        action: FoundationResourceAction,
    ) -> bool {
        if !self.mutations_ready()
            || self.pending_foundation.is_some()
            || self.pending_foundation_resource.is_some()
        {
            self.status_message = "Wait for the current nearby work to finish.".to_owned();
            return false;
        }
        let request = FoundationResourceRequest {
            request_id: self.next_request_id("foundation-resource"),
            node_id: node_id.to_owned(),
            action,
        };
        self.pending_foundation_resource = Some(PendingFoundationResource {
            pending: Some(self.api.post_json("/v1/foundation/resources", &request)),
            request,
            retries: 0,
            retry_timer: 0.0,
        });
        self.status_message = match action {
            FoundationResourceAction::Log => "Working the Whisperwood edge…",
            FoundationResourceAction::Mine => "Working the shallow stone seam…",
        }
        .to_owned();
        true
    }

    pub(super) fn poll_foundation_resource(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let Some(mut pending) = self.pending_foundation_resource.take() else {
            return;
        };
        pending.retry_timer = (pending.retry_timer - dt.max(0.0)).max(0.0);
        if pending.retry_timer > 0.0 {
            self.pending_foundation_resource = Some(pending);
            return;
        }
        if pending.pending.is_none() {
            pending.pending = Some(
                self.api
                    .post_json("/v1/foundation/resources", &pending.request),
            );
        }
        let Some(result) = pending
            .pending
            .as_mut()
            .and_then(|request| request.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        else {
            self.pending_foundation_resource = Some(pending);
            return;
        };
        pending.pending = None;
        match result {
            Ok(response) => {
                self.projection
                    .record_response_version(response.meta.server_tick, response.meta.cursor);
                let data = response.data;
                if data.accepted {
                    self.projection.player = Some(data.player);
                    if let Some(node) = self
                        .projection
                        .foundation_activity
                        .resource_nodes
                        .iter_mut()
                        .find(|node| node.node_id == data.node.node_id)
                    {
                        *node = data.node;
                    }
                    let detail = foundation_resource_success_notice(&data.yields);
                    self.status_message = detail.clone();
                    notices.push(NetworkNotice::Success(detail));
                } else {
                    let detail = data.reason.unwrap_or_else(|| {
                        "The shared road rejected that gathering action.".to_owned()
                    });
                    self.status_message = detail.clone();
                    notices.push(NetworkNotice::Warning(detail));
                }
                self.state_refresh = 0.0;
            }
            Err(error)
                if is_transient_transport_error(&error)
                    && pending.retries < super::commands::MAX_COMMAND_RETRIES =>
            {
                pending.retries += 1;
                pending.retry_timer = super::commands::COMMAND_RETRY_DELAY_SECONDS;
                let retries = pending.retries;
                self.pending_foundation_resource = Some(pending);
                notices.push(NetworkNotice::Warning(format!(
                    "The gathering result could not be confirmed; retrying the same work ({retries}/{}).",
                    super::commands::MAX_COMMAND_RETRIES
                )));
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }
}

pub(super) fn foundation_resource_success_notice(yields: &[FoundationResourceAmount]) -> String {
    let gathered = yields
        .iter()
        .map(|yielded| {
            let name = match yielded.kind {
                FoundationResourceKind::Timber => "timber",
                FoundationResourceKind::Stone => "stone",
                FoundationResourceKind::IronOre => "iron ore",
            };
            format!("{} {name}", yielded.amount)
        })
        .collect::<Vec<_>>()
        .join(" and ");
    format!("Gathered {gathered} with the shared crude tools.")
}
