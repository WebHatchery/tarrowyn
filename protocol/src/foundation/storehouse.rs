use super::{FoundationResourceAmount, FoundationResourceKind};
use crate::PlayerProjection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationStorehouseStage {
    SiteMarked,
    FoundationLaid,
    FrameRaised,
    Operational,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationStorehouseStageGate {
    pub stage: FoundationStorehouseStage,
    pub credited_units_required: Vec<FoundationResourceAmount>,
    pub visible_label: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationStorehouseMaterialRequirement {
    pub kind: FoundationResourceKind,
    pub units_required: u32,
    pub gold_per_unit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum FoundationStorehouseContributionInput {
    Material {
        kind: FoundationResourceKind,
        amount: u32,
    },
    Gold {
        toward: FoundationResourceKind,
        amount: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationStorehouseContribution {
    pub contribution_id: String,
    pub account_id: String,
    pub input: FoundationStorehouseContributionInput,
    pub credited_kind: FoundationResourceKind,
    pub credited_units: u32,
    pub contributed_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationStorehouseCompletion {
    pub completed_tick: u64,
    pub contributor_account_ids: Vec<String>,
    pub operational_infrastructure_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationStorehouseState {
    pub project_id: String,
    pub title: String,
    pub builder_landmark_id: String,
    pub noticeboard_landmark_id: String,
    pub site_landmark_id: String,
    pub operational_infrastructure_id: String,
    pub revision: u64,
    pub requirements: Vec<FoundationStorehouseMaterialRequirement>,
    pub stages: Vec<FoundationStorehouseStageGate>,
    pub current_stage: FoundationStorehouseStage,
    #[serde(default)]
    pub contributions: Vec<FoundationStorehouseContribution>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion: Option<FoundationStorehouseCompletion>,
}

impl Default for FoundationStorehouseState {
    fn default() -> Self {
        Self {
            project_id: "first-beacon-storehouse".to_owned(),
            title: "Build the First Beacon storehouse".to_owned(),
            builder_landmark_id: "builder-mara".to_owned(),
            noticeboard_landmark_id: "first-beacon-noticeboard".to_owned(),
            site_landmark_id: "storehouse-site".to_owned(),
            operational_infrastructure_id: "first-beacon-storehouse".to_owned(),
            revision: 1,
            requirements: vec![
                FoundationStorehouseMaterialRequirement {
                    kind: FoundationResourceKind::Timber,
                    units_required: 8,
                    gold_per_unit: 2,
                },
                FoundationStorehouseMaterialRequirement {
                    kind: FoundationResourceKind::Stone,
                    units_required: 6,
                    gold_per_unit: 3,
                },
            ],
            stages: vec![
                FoundationStorehouseStageGate {
                    stage: FoundationStorehouseStage::SiteMarked,
                    credited_units_required: Vec::new(),
                    visible_label: "Marked storehouse site".to_owned(),
                },
                FoundationStorehouseStageGate {
                    stage: FoundationStorehouseStage::FoundationLaid,
                    credited_units_required: vec![
                        FoundationResourceAmount {
                            kind: FoundationResourceKind::Timber,
                            amount: 1,
                        },
                        FoundationResourceAmount {
                            kind: FoundationResourceKind::Stone,
                            amount: 3,
                        },
                    ],
                    visible_label: "Dry-stone foundation".to_owned(),
                },
                FoundationStorehouseStageGate {
                    stage: FoundationStorehouseStage::FrameRaised,
                    credited_units_required: vec![
                        FoundationResourceAmount {
                            kind: FoundationResourceKind::Timber,
                            amount: 6,
                        },
                        FoundationResourceAmount {
                            kind: FoundationResourceKind::Stone,
                            amount: 4,
                        },
                    ],
                    visible_label: "Raised timber frame".to_owned(),
                },
                FoundationStorehouseStageGate {
                    stage: FoundationStorehouseStage::Operational,
                    credited_units_required: vec![
                        FoundationResourceAmount {
                            kind: FoundationResourceKind::Timber,
                            amount: 8,
                        },
                        FoundationResourceAmount {
                            kind: FoundationResourceKind::Stone,
                            amount: 6,
                        },
                    ],
                    visible_label: "Operational storehouse".to_owned(),
                },
            ],
            current_stage: FoundationStorehouseStage::SiteMarked,
            contributions: Vec::new(),
            completion: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationStorehouseAction {
    Inspect,
    Contribute,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationStorehouseRequest {
    pub request_id: String,
    pub action: FoundationStorehouseAction,
    pub landmark_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contribution: Option<FoundationStorehouseContributionInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationStorehouseResponse {
    pub request_id: String,
    pub action: FoundationStorehouseAction,
    pub accepted: bool,
    pub storehouse: FoundationStorehouseState,
    pub player: PlayerProjection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
