use super::*;

#[test]
fn first_hour_contract_orders_every_foundational_activity_without_gating_access() {
    let contract = FoundationJourneyContract::default();
    let kinds = contract
        .milestones
        .iter()
        .map(|milestone| milestone.kind)
        .collect::<Vec<_>>();

    assert_eq!(contract.journey_id, "first-beacon-first-hour");
    assert_eq!(
        contract.access_policy,
        FoundationJourneyAccessPolicy::GuidedNotGated
    );
    assert!(contract.advanced_tools_secondary);
    assert_eq!(contract.milestones.len(), 12);
    assert_eq!(
        kinds,
        vec![
            FoundationJourneyMilestoneKind::ArriveAtBeacon,
            FoundationJourneyMilestoneKind::ConsultLocalNeed,
            FoundationJourneyMilestoneKind::PlantCommonField,
            FoundationJourneyMilestoneKind::ExploreWoodland,
            FoundationJourneyMilestoneKind::GatherTimber,
            FoundationJourneyMilestoneKind::ExploreStoneSeam,
            FoundationJourneyMilestoneKind::MineStone,
            FoundationJourneyMilestoneKind::ForgeFieldTool,
            FoundationJourneyMilestoneKind::CompleteBarter,
            FoundationJourneyMilestoneKind::ContributeStorehouse,
            FoundationJourneyMilestoneKind::HarvestCommonField,
            FoundationJourneyMilestoneKind::ReplantCommonField,
        ]
    );
    assert!(contract
        .milestones
        .iter()
        .all(|milestone| milestone.required_count == 1));
}

#[test]
fn journey_contract_names_a_useful_short_visit_and_a_complete_first_hour() {
    let contract = FoundationJourneyContract::default();
    let short = contract
        .rhythms
        .iter()
        .find(|rhythm| rhythm.kind == FoundationJourneyRhythmKind::UsefulShortVisit)
        .unwrap();
    let first_hour = contract
        .rhythms
        .iter()
        .find(|rhythm| rhythm.kind == FoundationJourneyRhythmKind::CohesiveFirstHour)
        .unwrap();

    assert_eq!(short.target_minutes, 15);
    assert_eq!(
        short.required_milestone_ids,
        vec!["consult-first-need", "plant-common-field"]
    );
    assert_eq!(first_hour.target_minutes, 60);
    assert_eq!(
        first_hour.required_milestone_ids.len(),
        contract.milestones.len()
    );
    assert_eq!(
        contract.future_goal.kind,
        FoundationJourneyFutureGoalKind::HarvestReplantedCrop
    );
    assert_eq!(contract.future_goal.goal_id, "harvest-return-crop");
}

#[test]
fn journey_progress_round_trips_typed_evidence_and_defaults_new_state() {
    let projection = FoundationJourneyProjection {
        contract: FoundationJourneyContract::default(),
        progress: FoundationJourneyProgress {
            journey_id: "first-beacon-first-hour".to_owned(),
            revision: 2,
            credits: vec![FoundationJourneyMilestoneCredit {
                milestone_id: "arrive-first-beacon".to_owned(),
                evidence_kind: FoundationJourneyEvidenceKind::Arrival,
                evidence_ref: "guest-session:resident-1".to_owned(),
                credited_tick: 1,
            }],
            completed_tick: None,
            future_goal_state: FoundationJourneyFutureGoalState::Locked,
            future_goal_completed_tick: None,
        },
        completed_milestones: 1,
        total_milestones: 12,
        next_milestone: Some(FoundationJourneyContract::default().milestones[1].clone()),
        next_action: "Talk to Mara or read the noticeboard beside the Beacon.".to_owned(),
    };
    let encoded = serde_json::to_string(&projection).unwrap();
    let decoded: FoundationJourneyProjection = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, projection);
    assert!(encoded.contains("\"evidence_kind\":\"arrival\""));
    assert!(encoded.contains("\"access_policy\":\"guided_not_gated\""));

    let mut legacy = serde_json::to_value(&projection.progress).unwrap();
    let object = legacy.as_object_mut().unwrap();
    object.remove("credits");
    object.remove("future_goal_state");
    let decoded_legacy: FoundationJourneyProgress = serde_json::from_value(legacy).unwrap();
    assert!(decoded_legacy.credits.is_empty());
    assert_eq!(
        decoded_legacy.future_goal_state,
        FoundationJourneyFutureGoalState::Locked
    );
}
