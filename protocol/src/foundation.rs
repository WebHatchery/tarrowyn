use crate::{PlayerProjection, Position};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationBaseline {
    pub fixture_id: String,
    pub schema_version: u32,
    pub settlement_id: String,
    pub landmarks: Vec<FoundationLandmark>,
    pub interactions: Vec<FoundationInteraction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationLandmark {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub position: Position,
    pub visible: bool,
    pub permanent: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationInteraction {
    pub id: String,
    pub landmark_id: String,
    pub action: String,
    pub authority: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationInteractionRequest {
    pub request_id: String,
    pub interaction_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationInteractionResponse {
    pub request_id: String,
    pub interaction_id: String,
    pub landmark_id: String,
    pub accepted: bool,
    pub title: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationResourceKind {
    Timber,
    Stone,
    IronOre,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationResourceAmount {
    pub kind: FoundationResourceKind,
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationResourceNode {
    pub node_id: String,
    pub landmark_id: String,
    pub deposits: Vec<FoundationResourceDeposit>,
    pub recovery_interval_ticks: u64,
    pub last_recovered_tick: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationResourceDeposit {
    pub kind: FoundationResourceKind,
    pub remaining: u32,
    pub capacity: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationCrudeToolKind {
    HandAxe,
    StonePick,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationToolAccess {
    pub landmark_id: String,
    pub tools: Vec<FoundationCrudeToolKind>,
    pub available_to_all: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationActivityState {
    pub resource_nodes: Vec<FoundationResourceNode>,
    pub crude_tool_access: Vec<FoundationToolAccess>,
    #[serde(default)]
    pub shared_cache: FoundationSharedCache,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationSharedCache {
    pub landmark_id: String,
    pub inventory: crate::Inventory,
    pub capacity: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationCacheAction {
    Inspect,
    Deposit,
    Withdraw,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationCacheRequest {
    pub request_id: String,
    pub action: FoundationCacheAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<FoundationResourceKind>,
    #[serde(default)]
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationCacheResponse {
    pub request_id: String,
    pub action: FoundationCacheAction,
    pub accepted: bool,
    pub cache: FoundationSharedCache,
    pub player: PlayerProjection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationResourceAction {
    Log,
    Mine,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationResourceRequest {
    pub request_id: String,
    pub node_id: String,
    pub action: FoundationResourceAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationResourceResponse {
    pub request_id: String,
    pub node_id: String,
    pub action: FoundationResourceAction,
    pub accepted: bool,
    pub yields: Vec<FoundationResourceAmount>,
    pub node: FoundationResourceNode,
    pub player: PlayerProjection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationFieldToolKind {
    #[default]
    Crude,
    Iron,
}

impl FoundationFieldToolKind {
    pub const fn max_condition(self) -> u8 {
        match self {
            Self::Crude => 3,
            Self::Iron => 6,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Crude => "crude field tool",
            Self::Iron => "iron field tool",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationForgeAction {
    Inspect,
    BurnCharcoal,
    ShapeHandle,
    ForgeFieldTool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationForgeMaterialKind {
    Timber,
    IronOre,
    Charcoal,
    ToolHandle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationForgeMaterialAmount {
    pub kind: FoundationForgeMaterialKind,
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationForgeRecipe {
    pub action: FoundationForgeAction,
    pub label: String,
    pub costs: Vec<FoundationForgeMaterialAmount>,
    pub produces: Vec<FoundationForgeMaterialAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<FoundationFieldToolKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationForgeState {
    pub landmark_id: String,
    pub recipes: Vec<FoundationForgeRecipe>,
    pub crude_tool_action_capacity: u8,
    pub improved_tool_action_capacity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationForgeRequest {
    pub request_id: String,
    pub action: FoundationForgeAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationForgeResponse {
    pub request_id: String,
    pub action: FoundationForgeAction,
    pub accepted: bool,
    pub forge: FoundationForgeState,
    pub player: PlayerProjection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests;
