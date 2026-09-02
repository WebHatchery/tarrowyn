use super::FoundationResourceKind;
use crate::{Inventory, PlayerProjection, Position};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FoundationPropertyStage {
    Tent,
    Camp,
    House,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationPropertyDirection {
    North,
    East,
    South,
    West,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationPropertyAccess {
    OwnerOnly,
    GuestsAllowed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationPropertyPlacementRule {
    InsideWorld,
    ClearTerrain,
    NoStructureOverlap,
    OutsideBeaconCommons,
    OutsideProtectedRoute,
    EntranceClear,
    EscapePathOpen,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyFootprint {
    pub width: u8,
    pub height: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyMaterialCost {
    pub kind: FoundationResourceKind,
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyStageDefinition {
    pub stage: FoundationPropertyStage,
    pub title: String,
    pub footprint: FoundationPropertyFootprint,
    pub storage_capacity: u32,
    pub material_costs: Vec<FoundationPropertyMaterialCost>,
    pub builder_gold_cost: u32,
    pub visible_result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyPlacementPolicy {
    pub rules: Vec<FoundationPropertyPlacementRule>,
    pub beacon_commons_radius: u8,
    pub entrance_clearance_tiles: u8,
    pub minimum_escape_routes: u8,
    pub maximum_properties_per_owner: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyUpkeepPolicy {
    pub interval_real_days: u16,
    pub condition_loss_per_interval: u8,
    pub minimum_condition: u8,
    pub maintenance_restores_condition: u8,
    pub ownership_changes_from_upkeep: bool,
    pub stored_goods_lost_from_upkeep: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyAccessPolicy {
    pub default_access: FoundationPropertyAccess,
    pub owner_may_change_access: bool,
    pub guests_may_inspect: bool,
    pub guests_may_store: bool,
    pub guests_may_collect: bool,
    pub guests_may_upgrade: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyBuilderPolicy {
    pub builder_landmark_id: String,
    pub timber_gold_per_unit: u32,
    pub stone_gold_per_unit: u32,
    pub substitutes_only_missing_materials: bool,
    pub player_building_remains_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyContract {
    pub contract_id: String,
    pub schema_version: u32,
    pub stages: Vec<FoundationPropertyStageDefinition>,
    pub placement: FoundationPropertyPlacementPolicy,
    pub upkeep: FoundationPropertyUpkeepPolicy,
    pub access: FoundationPropertyAccessPolicy,
    pub builder: FoundationPropertyBuilderPolicy,
}

impl Default for FoundationPropertyContract {
    fn default() -> Self {
        Self {
            contract_id: "first-beacon-personal-property-v1".to_owned(),
            schema_version: 1,
            stages: vec![
                FoundationPropertyStageDefinition {
                    stage: FoundationPropertyStage::Tent,
                    title: "Personal tent".to_owned(),
                    footprint: FoundationPropertyFootprint {
                        width: 1,
                        height: 1,
                    },
                    storage_capacity: 8,
                    material_costs: Vec::new(),
                    builder_gold_cost: 0,
                    visible_result: "A bedroll shelter and a small private chest.".to_owned(),
                },
                FoundationPropertyStageDefinition {
                    stage: FoundationPropertyStage::Camp,
                    title: "Established camp".to_owned(),
                    footprint: FoundationPropertyFootprint {
                        width: 2,
                        height: 2,
                    },
                    storage_capacity: 24,
                    material_costs: vec![
                        FoundationPropertyMaterialCost {
                            kind: FoundationResourceKind::Timber,
                            amount: 4,
                        },
                        FoundationPropertyMaterialCost {
                            kind: FoundationResourceKind::Stone,
                            amount: 2,
                        },
                    ],
                    builder_gold_cost: 14,
                    visible_result: "A framed camp with a work awning and larger chest.".to_owned(),
                },
                FoundationPropertyStageDefinition {
                    stage: FoundationPropertyStage::House,
                    title: "First house".to_owned(),
                    footprint: FoundationPropertyFootprint {
                        width: 3,
                        height: 2,
                    },
                    storage_capacity: 48,
                    material_costs: vec![
                        FoundationPropertyMaterialCost {
                            kind: FoundationResourceKind::Timber,
                            amount: 8,
                        },
                        FoundationPropertyMaterialCost {
                            kind: FoundationResourceKind::Stone,
                            amount: 6,
                        },
                    ],
                    builder_gold_cost: 34,
                    visible_result: "A timber-and-stone house with a lockable storeroom."
                        .to_owned(),
                },
            ],
            placement: FoundationPropertyPlacementPolicy {
                rules: vec![
                    FoundationPropertyPlacementRule::InsideWorld,
                    FoundationPropertyPlacementRule::ClearTerrain,
                    FoundationPropertyPlacementRule::NoStructureOverlap,
                    FoundationPropertyPlacementRule::OutsideBeaconCommons,
                    FoundationPropertyPlacementRule::OutsideProtectedRoute,
                    FoundationPropertyPlacementRule::EntranceClear,
                    FoundationPropertyPlacementRule::EscapePathOpen,
                ],
                beacon_commons_radius: 3,
                entrance_clearance_tiles: 1,
                minimum_escape_routes: 1,
                maximum_properties_per_owner: 1,
            },
            upkeep: FoundationPropertyUpkeepPolicy {
                interval_real_days: 30,
                condition_loss_per_interval: 10,
                minimum_condition: 1,
                maintenance_restores_condition: 25,
                ownership_changes_from_upkeep: false,
                stored_goods_lost_from_upkeep: false,
            },
            access: FoundationPropertyAccessPolicy {
                default_access: FoundationPropertyAccess::OwnerOnly,
                owner_may_change_access: true,
                guests_may_inspect: true,
                guests_may_store: true,
                guests_may_collect: false,
                guests_may_upgrade: false,
            },
            builder: FoundationPropertyBuilderPolicy {
                builder_landmark_id: "builder-mara".to_owned(),
                timber_gold_per_unit: 2,
                stone_gold_per_unit: 3,
                substitutes_only_missing_materials: true,
                player_building_remains_available: true,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyState {
    pub property_id: String,
    pub owner_account_id: String,
    pub owner_name: String,
    pub stage: FoundationPropertyStage,
    pub anchor: Position,
    pub entrance: FoundationPropertyDirection,
    pub access: FoundationPropertyAccess,
    pub revision: u64,
    pub condition: u8,
    pub last_maintained_unix_millis: u64,
    pub storage: Inventory,
    pub storage_capacity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertySummary {
    pub property_id: String,
    pub owner_account_id: String,
    pub owner_name: String,
    pub stage: FoundationPropertyStage,
    pub anchor: Position,
    pub entrance: FoundationPropertyDirection,
    pub access: FoundationPropertyAccess,
    pub revision: u64,
    pub condition: u8,
    pub stored_units: u32,
    pub storage_capacity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyPlacementPreview {
    pub anchor: Position,
    pub entrance: FoundationPropertyDirection,
    pub footprint: FoundationPropertyFootprint,
    pub accepted: bool,
    #[serde(default)]
    pub rejected_rules: Vec<FoundationPropertyPlacementRule>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyProjection {
    pub contract: FoundationPropertyContract,
    #[serde(default)]
    pub properties: Vec<FoundationPropertySummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_property: Option<FoundationPropertyState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement_preview: Option<FoundationPropertyPlacementPreview>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationPropertyAction {
    PreviewPlacement,
    PlaceTent,
    Inspect,
    UpgradeWithMaterials,
    HireBuilder,
    SetAccess,
    Store,
    Collect,
    Maintain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyRequest {
    pub request_id: String,
    pub action: FoundationPropertyAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Position>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entrance: Option<FoundationPropertyDirection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<FoundationPropertyAccess>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<FoundationResourceKind>,
    #[serde(default)]
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationPropertyResponse {
    pub request_id: String,
    pub action: FoundationPropertyAction,
    pub accepted: bool,
    pub projection: FoundationPropertyProjection,
    pub player: PlayerProjection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
