use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationJourneyMilestoneKind {
    ArriveAtBeacon,
    ConsultLocalNeed,
    PlantCommonField,
    ExploreWoodland,
    GatherTimber,
    ExploreStoneSeam,
    MineStone,
    ForgeFieldTool,
    CompleteBarter,
    ContributeStorehouse,
    HarvestCommonField,
    ReplantCommonField,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationJourneyEvidenceKind {
    Arrival,
    Interaction,
    Farming,
    Exploration,
    ResourceWork,
    ForgeWork,
    Trade,
    Storehouse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationJourneyMilestoneDefinition {
    pub milestone_id: String,
    pub kind: FoundationJourneyMilestoneKind,
    pub title: String,
    pub direction: String,
    pub evidence_kind: FoundationJourneyEvidenceKind,
    pub target_id: String,
    pub required_count: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationJourneyRhythmKind {
    UsefulShortVisit,
    CohesiveFirstHour,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationJourneyRhythmDefinition {
    pub kind: FoundationJourneyRhythmKind,
    pub target_minutes: u16,
    pub required_milestone_ids: Vec<String>,
    pub outcome: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationJourneyAccessPolicy {
    GuidedNotGated,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationJourneyFutureGoalKind {
    HarvestReplantedCrop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationJourneyFutureGoalDefinition {
    pub goal_id: String,
    pub kind: FoundationJourneyFutureGoalKind,
    pub title: String,
    pub direction: String,
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationJourneyContract {
    pub journey_id: String,
    pub title: String,
    pub access_policy: FoundationJourneyAccessPolicy,
    pub advanced_tools_secondary: bool,
    pub milestones: Vec<FoundationJourneyMilestoneDefinition>,
    pub rhythms: Vec<FoundationJourneyRhythmDefinition>,
    pub future_goal: FoundationJourneyFutureGoalDefinition,
}

impl Default for FoundationJourneyContract {
    fn default() -> Self {
        let milestones = vec![
            milestone(
                "arrive-first-beacon",
                FoundationJourneyMilestoneKind::ArriveAtBeacon,
                "Arrive at the First Beacon",
                "Take in the Beacon, the tents, Mara, and the nearby work.",
                FoundationJourneyEvidenceKind::Arrival,
                "first-beacon",
            ),
            milestone(
                "consult-first-need",
                FoundationJourneyMilestoneKind::ConsultLocalNeed,
                "Ask what the camp needs",
                "Talk to Mara or read the noticeboard beside the Beacon.",
                FoundationJourneyEvidenceKind::Interaction,
                "first-beacon-storehouse",
            ),
            milestone(
                "plant-common-field",
                FoundationJourneyMilestoneKind::PlantCommonField,
                "Plant a shared crop",
                "Plant one empty common plot so time can work while you explore.",
                FoundationJourneyEvidenceKind::Farming,
                "first-beacon-fields",
            ),
            milestone(
                "explore-whisperwood",
                FoundationJourneyMilestoneKind::ExploreWoodland,
                "Find the woodland edge",
                "Walk beyond the tents to the marked Whisperwood trees.",
                FoundationJourneyEvidenceKind::Exploration,
                "whisperwood-edge",
            ),
            milestone(
                "gather-first-timber",
                FoundationJourneyMilestoneKind::GatherTimber,
                "Gather useful timber",
                "Use the shared hand axe beside the woodland.",
                FoundationJourneyEvidenceKind::ResourceWork,
                "whisperwood-edge-node",
            ),
            milestone(
                "explore-stone-seam",
                FoundationJourneyMilestoneKind::ExploreStoneSeam,
                "Find the shallow stone seam",
                "Follow the ground east of the Beacon to the marked seam.",
                FoundationJourneyEvidenceKind::Exploration,
                "first-beacon-mine",
            ),
            milestone(
                "mine-first-stone",
                FoundationJourneyMilestoneKind::MineStone,
                "Mine stone and ore",
                "Use the shared stone pick beside the seam.",
                FoundationJourneyEvidenceKind::ResourceWork,
                "shallow-stone-seam-node",
            ),
            milestone(
                "forge-field-tool",
                FoundationJourneyMilestoneKind::ForgeFieldTool,
                "Forge a lasting field tool",
                "Prepare fuel and a handle at the rough forge, then forge the iron tool.",
                FoundationJourneyEvidenceKind::ForgeWork,
                "first-beacon-forge",
            ),
            milestone(
                "complete-first-barter",
                FoundationJourneyMilestoneKind::CompleteBarter,
                "Exchange useful goods",
                "Complete one direct barter; self-supply remains available when nobody answers.",
                FoundationJourneyEvidenceKind::Trade,
                "first-beacon-field-tool",
            ),
            milestone(
                "contribute-storehouse",
                FoundationJourneyMilestoneKind::ContributeStorehouse,
                "Help raise the storehouse",
                "Give Mara carried timber or stone, or fund an exact listed substitute.",
                FoundationJourneyEvidenceKind::Storehouse,
                "first-beacon-storehouse",
            ),
            milestone(
                "harvest-common-field",
                FoundationJourneyMilestoneKind::HarvestCommonField,
                "Return to the ripe field",
                "Harvest a mature shared crop after the settlement work has had time to grow.",
                FoundationJourneyEvidenceKind::Farming,
                "first-beacon-fields",
            ),
            milestone(
                "replant-common-field",
                FoundationJourneyMilestoneKind::ReplantCommonField,
                "Leave another crop growing",
                "Replant the harvested plot before leaving the Beacon.",
                FoundationJourneyEvidenceKind::Farming,
                "first-beacon-fields",
            ),
        ];
        Self {
            journey_id: "first-beacon-first-hour".to_owned(),
            title: "Make a place at the First Beacon".to_owned(),
            access_policy: FoundationJourneyAccessPolicy::GuidedNotGated,
            advanced_tools_secondary: true,
            rhythms: vec![
                FoundationJourneyRhythmDefinition {
                    kind: FoundationJourneyRhythmKind::UsefulShortVisit,
                    target_minutes: 15,
                    required_milestone_ids: vec![
                        "consult-first-need".to_owned(),
                        "plant-common-field".to_owned(),
                    ],
                    outcome: "A shared crop is growing and the player knows the camp's concrete need."
                        .to_owned(),
                },
                FoundationJourneyRhythmDefinition {
                    kind: FoundationJourneyRhythmKind::CohesiveFirstHour,
                    target_minutes: 60,
                    required_milestone_ids: milestones
                        .iter()
                        .map(|milestone| milestone.milestone_id.clone())
                        .collect(),
                    outcome: "The player has used every foundational activity and leaves another crop growing."
                        .to_owned(),
                },
            ],
            milestones,
            future_goal: FoundationJourneyFutureGoalDefinition {
                goal_id: "harvest-return-crop".to_owned(),
                kind: FoundationJourneyFutureGoalKind::HarvestReplantedCrop,
                title: "Return for the next harvest".to_owned(),
                direction: "Come back after the replanted common crop matures, then harvest it."
                    .to_owned(),
                target_id: "first-beacon-fields".to_owned(),
            },
        }
    }
}

fn milestone(
    milestone_id: &str,
    kind: FoundationJourneyMilestoneKind,
    title: &str,
    direction: &str,
    evidence_kind: FoundationJourneyEvidenceKind,
    target_id: &str,
) -> FoundationJourneyMilestoneDefinition {
    FoundationJourneyMilestoneDefinition {
        milestone_id: milestone_id.to_owned(),
        kind,
        title: title.to_owned(),
        direction: direction.to_owned(),
        evidence_kind,
        target_id: target_id.to_owned(),
        required_count: 1,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationJourneyMilestoneCredit {
    pub milestone_id: String,
    pub evidence_kind: FoundationJourneyEvidenceKind,
    pub evidence_ref: String,
    pub credited_tick: u64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationJourneyFutureGoalState {
    #[default]
    Locked,
    Active,
    Complete,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationJourneyProgress {
    pub journey_id: String,
    pub revision: u64,
    #[serde(default)]
    pub credits: Vec<FoundationJourneyMilestoneCredit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_tick: Option<u64>,
    #[serde(default)]
    pub future_goal_state: FoundationJourneyFutureGoalState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub future_goal_completed_tick: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationJourneyProjection {
    pub contract: FoundationJourneyContract,
    pub progress: FoundationJourneyProgress,
    pub completed_milestones: u16,
    pub total_milestones: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_milestone: Option<FoundationJourneyMilestoneDefinition>,
    pub next_action: String,
}
