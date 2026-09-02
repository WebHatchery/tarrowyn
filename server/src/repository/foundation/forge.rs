//! Authoritative rough-forge recipes and replay-safe player material work.

use super::super::{models::RepositoryState, *};
use tarrowyn_protocol::{
    ApiResponse, FoundationFieldToolKind, FoundationForgeAction, FoundationForgeMaterialAmount,
    FoundationForgeMaterialKind, FoundationForgeRecipe, FoundationForgeRequest,
    FoundationForgeResponse, FoundationForgeState,
};

const FORGE_LANDMARK_ID: &str = "first-beacon-forge";
const CHARCOAL_TIMBER_COST: u32 = 1;
const HANDLE_TIMBER_COST: u32 = 1;
const TOOL_IRON_COST: u32 = 2;
const TOOL_CHARCOAL_COST: u32 = 1;
const TOOL_HANDLE_COST: u32 = 1;

pub(super) fn forge_state() -> FoundationForgeState {
    FoundationForgeState {
        landmark_id: FORGE_LANDMARK_ID.to_owned(),
        recipes: vec![
            FoundationForgeRecipe {
                action: FoundationForgeAction::BurnCharcoal,
                label: "Burn charcoal".to_owned(),
                costs: vec![material(
                    FoundationForgeMaterialKind::Timber,
                    CHARCOAL_TIMBER_COST,
                )],
                produces: vec![material(FoundationForgeMaterialKind::Charcoal, 1)],
                tool: None,
            },
            FoundationForgeRecipe {
                action: FoundationForgeAction::ShapeHandle,
                label: "Shape tool handle".to_owned(),
                costs: vec![material(
                    FoundationForgeMaterialKind::Timber,
                    HANDLE_TIMBER_COST,
                )],
                produces: vec![material(FoundationForgeMaterialKind::ToolHandle, 1)],
                tool: None,
            },
            FoundationForgeRecipe {
                action: FoundationForgeAction::ForgeFieldTool,
                label: "Forge iron field tool".to_owned(),
                costs: vec![
                    material(FoundationForgeMaterialKind::IronOre, TOOL_IRON_COST),
                    material(FoundationForgeMaterialKind::Charcoal, TOOL_CHARCOAL_COST),
                    material(FoundationForgeMaterialKind::ToolHandle, TOOL_HANDLE_COST),
                ],
                produces: Vec::new(),
                tool: Some(FoundationFieldToolKind::Iron),
            },
        ],
        crude_tool_action_capacity: FoundationFieldToolKind::Crude.max_condition(),
        improved_tool_action_capacity: FoundationFieldToolKind::Iron.max_condition(),
    }
}

fn material(kind: FoundationForgeMaterialKind, amount: u32) -> FoundationForgeMaterialAmount {
    FoundationForgeMaterialAmount { kind, amount }
}

impl WorldRepository {
    pub fn foundation_forge(
        &self,
        token: &str,
        request: FoundationForgeRequest,
    ) -> Result<ApiResponse<FoundationForgeResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        let identity_key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        if let Some(previous) = state
            .identities
            .get(&identity_key)
            .and_then(|identity| identity.foundation_forge_results.get(&request.request_id))
            .cloned()
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous,
            });
        }
        let mut response = FoundationForgeResponse {
            request_id: request.request_id.clone(),
            action: request.action,
            accepted: false,
            forge: forge_state(),
            player: player_projection(&state, &identity_key),
            reason: None,
        };
        let identity = state
            .identities
            .get(&identity_key)
            .expect("identity exists");
        if identity.knocked_out {
            response.reason = Some("Recover before working at the rough forge.".to_owned());
            return self.store_forge_result(&mut state, identity_key, response);
        }
        let forge_position = crate::content::foundation_baseline()
            .landmarks
            .into_iter()
            .find(|landmark| landmark.id == FORGE_LANDMARK_ID)
            .map(|landmark| landmark.position)
            .expect("validated rough forge landmark exists");
        if identity.position.manhattan_distance(forge_position) > 1 {
            response.reason = Some("Stand beside the rough forge before working it.".to_owned());
            return self.store_forge_result(&mut state, identity_key, response);
        }

        response.reason = apply_action(&mut state, &identity_key, request.action);
        response.accepted = response.reason.is_none();
        if response.accepted && request.action != FoundationForgeAction::Inspect {
            skills::record_practice(&mut state, &identity_key, "smithing");
        }
        response.player = player_projection(&state, &identity_key);
        self.store_forge_result(&mut state, identity_key, response)
    }

    fn store_forge_result(
        &self,
        state: &mut RepositoryState,
        identity_key: String,
        response: FoundationForgeResponse,
    ) -> Result<ApiResponse<FoundationForgeResponse>, RepositoryError> {
        let request_id = response.request_id.clone();
        state
            .identities
            .get_mut(&identity_key)
            .expect("identity exists")
            .foundation_forge_results
            .insert(request_id.clone(), response.clone());
        super::super::models::trim_replay_cache(
            &mut state
                .identities
                .get_mut(&identity_key)
                .expect("identity exists")
                .foundation_forge_results,
        );
        record_command_outcome(state, response.accepted);
        self.persist(state)?;
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request_id), Some(state.cursor)),
            data: response,
        })
    }
}

fn apply_action(
    state: &mut RepositoryState,
    identity_key: &str,
    action: FoundationForgeAction,
) -> Option<String> {
    let identity = state
        .identities
        .get_mut(identity_key)
        .expect("identity exists");
    match action {
        FoundationForgeAction::Inspect => None,
        FoundationForgeAction::BurnCharcoal => {
            if identity.inventory.timber < CHARCOAL_TIMBER_COST {
                return Some("Burning charcoal needs 1 gathered timber.".to_owned());
            }
            identity.inventory.timber -= CHARCOAL_TIMBER_COST;
            identity.inventory.charcoal = identity.inventory.charcoal.saturating_add(1);
            None
        }
        FoundationForgeAction::ShapeHandle => {
            if identity.inventory.timber < HANDLE_TIMBER_COST {
                return Some("Shaping a handle needs 1 gathered timber.".to_owned());
            }
            identity.inventory.timber -= HANDLE_TIMBER_COST;
            identity.inventory.tool_handles = identity.inventory.tool_handles.saturating_add(1);
            None
        }
        FoundationForgeAction::ForgeFieldTool => forge_field_tool(identity),
    }
}

fn forge_field_tool(identity: &mut super::super::models::Identity) -> Option<String> {
    if identity.field_tool_kind == FoundationFieldToolKind::Iron
        && identity.field_tool_condition == FoundationFieldToolKind::Iron.max_condition()
    {
        return Some("The iron field tool is already in full working order.".to_owned());
    }
    if identity.inventory.iron_ore < TOOL_IRON_COST
        || identity.inventory.charcoal < TOOL_CHARCOAL_COST
        || identity.inventory.tool_handles < TOOL_HANDLE_COST
    {
        return Some(
            "Forging the iron field tool needs 2 iron ore, 1 charcoal, and 1 tool handle."
                .to_owned(),
        );
    }
    identity.inventory.iron_ore -= TOOL_IRON_COST;
    identity.inventory.charcoal -= TOOL_CHARCOAL_COST;
    identity.inventory.tool_handles -= TOOL_HANDLE_COST;
    identity.field_tool_kind = FoundationFieldToolKind::Iron;
    identity.field_tool_condition = FoundationFieldToolKind::Iron.max_condition();
    None
}

#[cfg(test)]
#[path = "forge/tests.rs"]
mod tests;
