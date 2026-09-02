use super::*;

impl OnlineClient {
    pub fn queue_foundation_interaction(&mut self, interaction_id: &str) -> bool {
        if !self.mutations_ready()
            || self.pending_foundation.is_some()
            || self.pending_foundation_resource.is_some()
            || self.pending_foundation_cache.is_some()
            || self.pending_foundation_forge.is_some()
            || self.pending_foundation_storehouse.is_some()
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
        self.pending_foundation.is_some()
            || self.pending_foundation_resource.is_some()
            || self.pending_foundation_cache.is_some()
            || self.pending_foundation_forge.is_some()
            || self.pending_foundation_storehouse.is_some()
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
            || self.pending_foundation_cache.is_some()
            || self.pending_foundation_forge.is_some()
            || self.pending_foundation_storehouse.is_some()
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

    pub fn queue_foundation_cache(
        &mut self,
        action: FoundationCacheAction,
        resource: Option<FoundationResourceKind>,
    ) -> bool {
        if !self.mutations_ready() || self.foundation_interaction_pending() {
            self.status_message = "Wait for the current nearby work to finish.".to_owned();
            return false;
        }
        let request = FoundationCacheRequest {
            request_id: self.next_request_id("foundation-cache"),
            action,
            resource,
            amount: u32::from(action != FoundationCacheAction::Inspect),
        };
        self.pending_foundation_cache = Some(PendingFoundationCache {
            pending: Some(self.api.post_json("/v1/foundation/cache", &request)),
            request,
            retries: 0,
            retry_timer: 0.0,
        });
        self.status_message = match action {
            FoundationCacheAction::Inspect => "Checking the shared cache…",
            FoundationCacheAction::Deposit => "Storing goods in the shared cache…",
            FoundationCacheAction::Withdraw => "Collecting goods from the shared cache…",
        }
        .to_owned();
        true
    }

    pub(super) fn poll_foundation_cache(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let Some(mut pending) = self.pending_foundation_cache.take() else {
            return;
        };
        pending.retry_timer = (pending.retry_timer - dt.max(0.0)).max(0.0);
        if pending.retry_timer > 0.0 {
            self.pending_foundation_cache = Some(pending);
            return;
        }
        if pending.pending.is_none() {
            pending.pending = Some(self.api.post_json("/v1/foundation/cache", &pending.request));
        }
        let Some(result) = pending
            .pending
            .as_mut()
            .and_then(|request| request.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        else {
            self.pending_foundation_cache = Some(pending);
            return;
        };
        pending.pending = None;
        match result {
            Ok(response) => {
                self.projection
                    .record_response_version(response.meta.server_tick, response.meta.cursor);
                let data = response.data;
                self.projection.foundation_activity.shared_cache = data.cache;
                self.projection.player = Some(data.player);
                let detail = if data.accepted {
                    foundation_cache_success_notice(data.action, pending.request.resource)
                } else {
                    data.reason
                        .unwrap_or_else(|| "The shared cache rejected that request.".to_owned())
                };
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
                self.pending_foundation_cache = Some(pending);
                notices.push(NetworkNotice::Warning(format!(
                    "The cache result could not be confirmed; retrying the same request ({retries}/{}).",
                    super::commands::MAX_COMMAND_RETRIES
                )));
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }

    pub fn queue_foundation_forge(&mut self, action: FoundationForgeAction) -> bool {
        if !self.mutations_ready() || self.foundation_interaction_pending() {
            self.status_message = "Wait for the current nearby work to finish.".to_owned();
            return false;
        }
        let request = FoundationForgeRequest {
            request_id: self.next_request_id("foundation-forge"),
            action,
        };
        self.pending_foundation_forge = Some(PendingFoundationForge {
            pending: Some(self.api.post_json("/v1/foundation/forge", &request)),
            request,
            retries: 0,
            retry_timer: 0.0,
        });
        self.status_message = match action {
            FoundationForgeAction::Inspect => "Reading the rough-forge ledger…",
            FoundationForgeAction::BurnCharcoal => "Banking timber into charcoal…",
            FoundationForgeAction::ShapeHandle => "Shaping a field-tool handle…",
            FoundationForgeAction::ForgeFieldTool => "Forging an iron field tool…",
        }
        .to_owned();
        true
    }

    pub(super) fn poll_foundation_forge(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let Some(mut pending) = self.pending_foundation_forge.take() else {
            return;
        };
        pending.retry_timer = (pending.retry_timer - dt.max(0.0)).max(0.0);
        if pending.retry_timer > 0.0 {
            self.pending_foundation_forge = Some(pending);
            return;
        }
        if pending.pending.is_none() {
            pending.pending = Some(self.api.post_json("/v1/foundation/forge", &pending.request));
        }
        let Some(result) = pending
            .pending
            .as_mut()
            .and_then(|request| request.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        else {
            self.pending_foundation_forge = Some(pending);
            return;
        };
        pending.pending = None;
        match result {
            Ok(response) => {
                self.projection
                    .record_response_version(response.meta.server_tick, response.meta.cursor);
                let data = response.data;
                self.projection.player = Some(data.player.clone());
                let detail = if data.accepted {
                    foundation_forge_success_notice(data.action, &data.player)
                } else {
                    data.reason
                        .unwrap_or_else(|| "The rough forge rejected that request.".to_owned())
                };
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
                self.pending_foundation_forge = Some(pending);
                notices.push(NetworkNotice::Warning(format!(
                    "The forge result could not be confirmed; retrying the same request ({retries}/{}).",
                    super::commands::MAX_COMMAND_RETRIES
                )));
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }

    pub fn queue_foundation_storehouse(
        &mut self,
        landmark_id: &str,
        contribution: Option<FoundationStorehouseContributionInput>,
    ) -> bool {
        if !self.mutations_ready() || self.foundation_interaction_pending() {
            self.status_message = "Wait for the current nearby work to finish.".to_owned();
            return false;
        }
        let action = if contribution.is_some() {
            FoundationStorehouseAction::Contribute
        } else {
            FoundationStorehouseAction::Inspect
        };
        let request = FoundationStorehouseRequest {
            request_id: self.next_request_id("foundation-storehouse"),
            action,
            landmark_id: landmark_id.to_owned(),
            contribution,
        };
        self.pending_foundation_storehouse = Some(PendingFoundationStorehouse {
            pending: Some(self.api.post_json("/v1/foundation/storehouse", &request)),
            request,
            retries: 0,
            retry_timer: 0.0,
        });
        self.status_message = match action {
            FoundationStorehouseAction::Inspect => "Reading Mara's storehouse ledger…",
            FoundationStorehouseAction::Contribute => "Delivering goods to Mara's project…",
        }
        .to_owned();
        true
    }

    pub(super) fn poll_foundation_storehouse(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let Some(mut pending) = self.pending_foundation_storehouse.take() else {
            return;
        };
        pending.retry_timer = (pending.retry_timer - dt.max(0.0)).max(0.0);
        if pending.retry_timer > 0.0 {
            self.pending_foundation_storehouse = Some(pending);
            return;
        }
        if pending.pending.is_none() {
            pending.pending = Some(
                self.api
                    .post_json("/v1/foundation/storehouse", &pending.request),
            );
        }
        let Some(result) = pending
            .pending
            .as_mut()
            .and_then(|request| request.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        else {
            self.pending_foundation_storehouse = Some(pending);
            return;
        };
        pending.pending = None;
        match result {
            Ok(response) => {
                let cursor = response.meta.cursor.unwrap_or(self.projection.cursor);
                let projection_current = self
                    .projection
                    .response_is_current(response.meta.server_tick, cursor);
                self.projection
                    .record_response_version(response.meta.server_tick, response.meta.cursor);
                let data = response.data;
                if projection_current {
                    self.projection.foundation_activity.storehouse = data.storehouse.clone();
                    self.projection.player = Some(data.player);
                }
                let detail = if data.accepted {
                    foundation_storehouse_success_notice(&data.storehouse, data.action)
                } else {
                    data.reason
                        .unwrap_or_else(|| "Mara rejected that storehouse request.".to_owned())
                };
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
                self.pending_foundation_storehouse = Some(pending);
                notices.push(NetworkNotice::Warning(format!(
                    "The contribution could not be confirmed; retrying the same request ({retries}/{}).",
                    super::commands::MAX_COMMAND_RETRIES
                )));
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }
}

pub(super) fn foundation_storehouse_success_notice(
    project: &tarrowyn_protocol::FoundationStorehouseState,
    action: FoundationStorehouseAction,
) -> String {
    let remaining = |kind| {
        let required = project
            .requirements
            .iter()
            .find(|requirement| requirement.kind == kind)
            .map_or(0, |requirement| requirement.units_required);
        let credited = project
            .contributions
            .iter()
            .filter(|contribution| contribution.credited_kind == kind)
            .fold(0_u32, |total, contribution| {
                total.saturating_add(contribution.credited_units)
            });
        required.saturating_sub(credited)
    };
    let stage = project
        .stages
        .iter()
        .find(|gate| gate.stage == project.current_stage)
        .map(|gate| gate.visible_label.as_str())
        .unwrap_or("Storehouse project");
    if project.completion.is_some() {
        return "First Beacon storehouse operational — the public structure is permanently recorded."
            .to_owned();
    }
    let verb = match action {
        FoundationStorehouseAction::Inspect => "Storehouse inspected",
        FoundationStorehouseAction::Contribute => "Contribution accepted",
    };
    format!(
        "{verb} — {stage}; {} timber and {} stone remain.",
        remaining(FoundationResourceKind::Timber),
        remaining(FoundationResourceKind::Stone)
    )
}

pub(super) fn foundation_forge_success_notice(
    action: FoundationForgeAction,
    player: &PlayerProjection,
) -> String {
    let outcome = match action {
        FoundationForgeAction::Inspect => "Rough forge inspected",
        FoundationForgeAction::BurnCharcoal => "Burned 1 timber into charcoal",
        FoundationForgeAction::ShapeHandle => "Shaped 1 timber into a tool handle",
        FoundationForgeAction::ForgeFieldTool => "Forged an iron field tool",
    };
    format!(
        "{outcome}. Materials: {} timber, {} iron ore, {} charcoal, {} handles; {} {}/{}.",
        player.inventory.timber,
        player.inventory.iron_ore,
        player.inventory.charcoal,
        player.inventory.tool_handles,
        player.field_tool_kind.label(),
        player.field_tool_condition,
        player.field_tool_kind.max_condition()
    )
}

pub(super) fn foundation_cache_success_notice(
    action: FoundationCacheAction,
    resource: Option<FoundationResourceKind>,
) -> String {
    let material = match resource {
        Some(FoundationResourceKind::Timber) => "timber",
        Some(FoundationResourceKind::Stone) => "stone",
        Some(FoundationResourceKind::IronOre) => "iron ore",
        None => "material",
    };
    match action {
        FoundationCacheAction::Inspect => "The shared cache ledger is current.".to_owned(),
        FoundationCacheAction::Deposit => format!("Stored 1 {material} in the shared cache."),
        FoundationCacheAction::Withdraw => format!("Collected 1 {material} from the shared cache."),
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
