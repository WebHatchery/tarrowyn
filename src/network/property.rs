use super::*;

impl OnlineClient {
    pub fn foundation_property_pending(&self) -> bool {
        self.pending_foundation_property.is_some()
    }

    pub fn queue_foundation_property(&mut self, mut request: FoundationPropertyRequest) -> bool {
        if !self.mutations_ready() || self.foundation_interaction_pending() {
            self.status_message = "Wait for the current nearby work to finish.".to_owned();
            return false;
        }
        request.request_id = self.next_request_id("foundation-property");
        self.pending_foundation_property = Some(PendingFoundationProperty {
            pending: Some(self.api.post_json("/v1/foundation/properties", &request)),
            request,
            retries: 0,
            retry_timer: 0.0,
        });
        self.status_message = "Checking the shelter ledger…".to_owned();
        true
    }

    pub(super) fn poll_foundation_property_view(
        &mut self,
        dt: f32,
        notices: &mut Vec<NetworkNotice>,
    ) {
        let result = self
            .pending_foundation_property_view
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = result else { return };
        self.pending_foundation_property_view = None;
        match result {
            Ok(response) => {
                let cursor = response.meta.cursor.unwrap_or(self.projection.cursor);
                if self
                    .projection
                    .response_is_current(response.meta.server_tick, cursor)
                {
                    apply_property_projection(&mut self.projection.property, response.data);
                }
                self.projection
                    .record_response_version(response.meta.server_tick, response.meta.cursor);
            }
            Err(error) => notices.push(NetworkNotice::Warning(format!(
                "Personal shelters are temporarily unavailable. {}",
                short_error(&error)
            ))),
        }
    }

    pub(super) fn poll_foundation_property(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let Some(mut pending) = self.pending_foundation_property.take() else {
            return;
        };
        pending.retry_timer = (pending.retry_timer - dt.max(0.0)).max(0.0);
        if pending.retry_timer > 0.0 {
            self.pending_foundation_property = Some(pending);
            return;
        }
        if pending.pending.is_none() {
            pending.pending = Some(
                self.api
                    .post_json("/v1/foundation/properties", &pending.request),
            );
        }
        let Some(result) = pending
            .pending
            .as_mut()
            .and_then(|request| request.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        else {
            self.pending_foundation_property = Some(pending);
            return;
        };
        pending.pending = None;
        match result {
            Ok(response) => {
                let cursor = response.meta.cursor.unwrap_or(self.projection.cursor);
                let current = self
                    .projection
                    .response_is_current(response.meta.server_tick, cursor);
                self.projection
                    .record_response_version(response.meta.server_tick, response.meta.cursor);
                let data = response.data;
                if current {
                    let mut projection = data.projection;
                    if data.action != tarrowyn_protocol::FoundationPropertyAction::PreviewPlacement
                    {
                        projection.placement_preview = None;
                    }
                    apply_property_projection(&mut self.projection.property, projection);
                    self.projection.player = Some(data.player);
                }
                let detail = data
                    .reason
                    .unwrap_or_else(|| property_success_notice(data.action));
                self.status_message = detail.clone();
                notices.push(if data.accepted {
                    NetworkNotice::Success(detail)
                } else {
                    NetworkNotice::Warning(detail)
                });
                self.state_refresh = 0.0;
            }
            Err(error)
                if is_transient_transport_error(&error)
                    && pending.retries < super::commands::MAX_COMMAND_RETRIES =>
            {
                pending.retries += 1;
                pending.retry_timer = super::commands::COMMAND_RETRY_DELAY_SECONDS;
                let retries = pending.retries;
                self.pending_foundation_property = Some(pending);
                notices.push(NetworkNotice::Warning(format!(
                    "The shelter result could not be confirmed; retrying the same request ({retries}/{}).",
                    super::commands::MAX_COMMAND_RETRIES
                )));
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }
}

pub(super) fn apply_property_projection(
    current: &mut FoundationPropertyProjection,
    incoming: FoundationPropertyProjection,
) -> bool {
    let would_lose_newer_state = current.properties.iter().any(|existing| {
        incoming
            .properties
            .iter()
            .find(|candidate| candidate.property_id == existing.property_id)
            .is_none_or(|candidate| candidate.revision < existing.revision)
    });
    if would_lose_newer_state {
        return false;
    }
    let preview = current.placement_preview.clone();
    *current = incoming;
    if current.own_property.is_none() && current.placement_preview.is_none() {
        current.placement_preview = preview;
    }
    true
}

fn property_success_notice(action: tarrowyn_protocol::FoundationPropertyAction) -> String {
    use tarrowyn_protocol::FoundationPropertyAction::*;
    match action {
        PreviewPlacement => "Shelter placement preview updated.",
        PlaceTent => "Personal tent pitched and recorded.",
        Inspect => "Personal shelter inspected.",
        UpgradeWithMaterials => "Shelter improved with carried materials.",
        HireBuilder => "Mara supplied the missing materials.",
        SetAccess => "Shelter access updated.",
        Store => "Material stored in the shelter chest.",
        Collect => "Material collected from the shelter chest.",
        Maintain => "Shelter condition restored.",
    }
    .to_owned()
}
