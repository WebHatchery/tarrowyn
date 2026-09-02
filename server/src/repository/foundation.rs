//! Persistent foundational gathering state and deterministic renewal.

use super::models::RepositoryState;
use tarrowyn_protocol::{
    ApiResponse, FoundationActivityState, FoundationCrudeToolKind, FoundationResourceAction,
    FoundationResourceAmount, FoundationResourceDeposit, FoundationResourceKind,
    FoundationResourceNode, FoundationResourceRequest, FoundationResourceResponse,
    FoundationToolAccess,
};

const RESOURCE_RECOVERY_INTERVAL_TICKS: u64 = 6;

pub(super) fn fresh() -> FoundationActivityState {
    FoundationActivityState {
        resource_nodes: vec![
            FoundationResourceNode {
                node_id: "whisperwood-edge-node".to_owned(),
                landmark_id: "whisperwood-edge".to_owned(),
                deposits: vec![deposit(FoundationResourceKind::Timber, 12)],
                recovery_interval_ticks: RESOURCE_RECOVERY_INTERVAL_TICKS,
                last_recovered_tick: 0,
            },
            FoundationResourceNode {
                node_id: "shallow-stone-seam-node".to_owned(),
                landmark_id: "first-beacon-mine".to_owned(),
                deposits: vec![
                    deposit(FoundationResourceKind::Stone, 10),
                    deposit(FoundationResourceKind::IronOre, 4),
                ],
                recovery_interval_ticks: RESOURCE_RECOVERY_INTERVAL_TICKS,
                last_recovered_tick: 0,
            },
        ],
        crude_tool_access: vec![FoundationToolAccess {
            landmark_id: "first-beacon-tool-rack".to_owned(),
            tools: vec![
                FoundationCrudeToolKind::HandAxe,
                FoundationCrudeToolKind::StonePick,
            ],
            available_to_all: true,
        }],
    }
}

fn deposit(kind: FoundationResourceKind, capacity: u32) -> FoundationResourceDeposit {
    FoundationResourceDeposit {
        kind,
        remaining: capacity,
        capacity,
    }
}

pub(super) fn recover_resource_nodes(state: &mut RepositoryState) {
    for node in &mut state.foundation_activity.resource_nodes {
        let interval = node.recovery_interval_ticks.max(1);
        let elapsed = state.tick.saturating_sub(node.last_recovered_tick);
        let cycles = elapsed / interval;
        if cycles == 0 {
            continue;
        }
        let recovered = u32::try_from(cycles).unwrap_or(u32::MAX);
        for deposit in &mut node.deposits {
            deposit.remaining = deposit
                .remaining
                .saturating_add(recovered)
                .min(deposit.capacity);
        }
        node.last_recovered_tick = node
            .last_recovered_tick
            .saturating_add(cycles.saturating_mul(interval));
    }
}

impl super::WorldRepository {
    pub fn foundation_resource(
        &self,
        token: &str,
        request: FoundationResourceRequest,
    ) -> Result<ApiResponse<FoundationResourceResponse>, super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        let identity_key = super::authenticate(&mut state, token, &self.config)?;
        super::validate_request_id(&request.request_id)?;
        if let Some(previous) = state
            .identities
            .get(&identity_key)
            .and_then(|identity| {
                identity
                    .foundation_resource_results
                    .get(&request.request_id)
            })
            .cloned()
        {
            return Ok(ApiResponse {
                meta: super::meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous,
            });
        }
        let node_id = super::validate_bounded_text(
            &request.node_id,
            160,
            "invalid_foundation_resource",
            "A nearby resource selector must contain 1 to 160 characters and no control characters.",
        )?;
        let node_index = state
            .foundation_activity
            .resource_nodes
            .iter()
            .position(|node| node.node_id == node_id)
            .ok_or_else(|| {
                super::RepositoryError::new(
                    404,
                    "foundation_resource_not_found",
                    "That nearby resource node is not part of the First Beacon world.",
                )
            })?;
        let node = state.foundation_activity.resource_nodes[node_index].clone();
        let mut response = FoundationResourceResponse {
            request_id: request.request_id.clone(),
            node_id: request.node_id.clone(),
            action: request.action,
            accepted: false,
            yields: Vec::new(),
            node: node.clone(),
            player: super::player_projection(&state, &identity_key),
            reason: None,
        };
        if state
            .identities
            .get(&identity_key)
            .is_some_and(|identity| identity.knocked_out)
        {
            response.reason =
                Some("Recover at the First Beacon before returning to gathering work.".to_owned());
            return self.store_foundation_resource_result(&mut state, identity_key, response);
        }
        let expected_landmark = match request.action {
            FoundationResourceAction::Log => "whisperwood-edge",
            FoundationResourceAction::Mine => "first-beacon-mine",
        };
        if node.landmark_id != expected_landmark {
            response.reason = Some(
                match request.action {
                    FoundationResourceAction::Log => "Logging requires a marked woodland node.",
                    FoundationResourceAction::Mine => "Mining requires a marked stone seam.",
                }
                .to_owned(),
            );
            return self.store_foundation_resource_result(&mut state, identity_key, response);
        }
        let landmark_position = crate::content::foundation_baseline()
            .landmarks
            .into_iter()
            .find(|landmark| landmark.id == node.landmark_id)
            .map(|landmark| landmark.position)
            .expect("validated foundation node references a landmark");
        let player_position = state
            .identities
            .get(&identity_key)
            .expect("identity exists")
            .position;
        if player_position.manhattan_distance(landmark_position) > 1 {
            response.reason = Some("Stand beside the resource before working it.".to_owned());
            return self.store_foundation_resource_result(&mut state, identity_key, response);
        }
        let required_tool = match request.action {
            FoundationResourceAction::Log => FoundationCrudeToolKind::HandAxe,
            FoundationResourceAction::Mine => FoundationCrudeToolKind::StonePick,
        };
        let tool_available = state
            .foundation_activity
            .crude_tool_access
            .iter()
            .any(|access| access.available_to_all && access.tools.contains(&required_tool));
        if !tool_available {
            response.reason =
                Some("The shared crude tool rack cannot supply this work.".to_owned());
            return self.store_foundation_resource_result(&mut state, identity_key, response);
        }
        let Some(yields) = apply_yields(&mut state, &identity_key, node_index, request.action)
        else {
            response.reason = Some(
                "This resource is depleted; its server-owned recovery is still underway."
                    .to_owned(),
            );
            response.node = state.foundation_activity.resource_nodes[node_index].clone();
            return self.store_foundation_resource_result(&mut state, identity_key, response);
        };
        response.accepted = true;
        response.yields = yields;
        response.node = state.foundation_activity.resource_nodes[node_index].clone();
        let practice = match request.action {
            FoundationResourceAction::Log => "forestry",
            FoundationResourceAction::Mine => "mining",
        };
        super::skills::record_practice(&mut state, &identity_key, practice);
        response.player = super::player_projection(&state, &identity_key);
        self.store_foundation_resource_result(&mut state, identity_key, response)
    }

    fn store_foundation_resource_result(
        &self,
        state: &mut RepositoryState,
        identity_key: String,
        response: FoundationResourceResponse,
    ) -> Result<ApiResponse<FoundationResourceResponse>, super::RepositoryError> {
        let request_id = response.request_id.clone();
        state
            .identities
            .get_mut(&identity_key)
            .expect("identity exists")
            .foundation_resource_results
            .insert(request_id.clone(), response.clone());
        super::models::trim_replay_cache(
            &mut state
                .identities
                .get_mut(&identity_key)
                .expect("identity exists")
                .foundation_resource_results,
        );
        super::record_command_outcome(state, response.accepted);
        self.persist(state)?;
        Ok(ApiResponse {
            meta: super::meta(state.tick, Some(request_id), Some(state.cursor)),
            data: response,
        })
    }
}

fn apply_yields(
    state: &mut RepositoryState,
    identity_key: &str,
    node_index: usize,
    action: FoundationResourceAction,
) -> Option<Vec<FoundationResourceAmount>> {
    let node = &mut state.foundation_activity.resource_nodes[node_index];
    let required = match action {
        FoundationResourceAction::Log => FoundationResourceKind::Timber,
        FoundationResourceAction::Mine => FoundationResourceKind::Stone,
    };
    let required_deposit = node
        .deposits
        .iter_mut()
        .find(|deposit| deposit.kind == required && deposit.remaining > 0)?;
    required_deposit.remaining -= 1;
    let mut yields = vec![FoundationResourceAmount {
        kind: required,
        amount: 2,
    }];
    if action == FoundationResourceAction::Mine {
        if let Some(ore) = node.deposits.iter_mut().find(|deposit| {
            deposit.kind == FoundationResourceKind::IronOre && deposit.remaining > 0
        }) {
            ore.remaining -= 1;
            yields.push(FoundationResourceAmount {
                kind: FoundationResourceKind::IronOre,
                amount: 1,
            });
        }
    }
    let inventory = &mut state
        .identities
        .get_mut(identity_key)
        .expect("identity exists")
        .inventory;
    for yielded in &yields {
        let quantity = match yielded.kind {
            FoundationResourceKind::Timber => &mut inventory.timber,
            FoundationResourceKind::Stone => &mut inventory.stone,
            FoundationResourceKind::IronOre => &mut inventory.iron_ore,
        };
        *quantity = quantity.saturating_add(yielded.amount);
    }
    Some(yields)
}

#[cfg(test)]
mod tests;
