use super::super::models::RepositoryState;
use tarrowyn_protocol::{
    ApiResponse, FarmingAction, FoundationJourneyContract, FoundationJourneyFutureGoalState,
    FoundationJourneyMilestoneCredit, FoundationJourneyMilestoneKind, FoundationJourneyProgress,
    FoundationJourneyProjection, FoundationResourceAction, FoundationStorehouseAction, TradeOffer,
};

pub(crate) const MAX_JOURNEY_CREDITS: usize = 12;

pub(crate) fn restore_progress(progress: &mut FoundationJourneyProgress) {
    if progress.journey_id.is_empty() {
        *progress = FoundationJourneyProgress::default();
    }
}

pub(crate) fn record_arrival(state: &mut RepositoryState, identity_key: &str) {
    credit(
        state,
        identity_key,
        FoundationJourneyMilestoneKind::ArriveAtBeacon,
        "session:arrive-first-beacon",
    );
}

pub(crate) fn record_interaction(
    state: &mut RepositoryState,
    identity_key: &str,
    interaction_id: &str,
) {
    if matches!(interaction_id, "speak-with-builder" | "read-local-needs") {
        credit(
            state,
            identity_key,
            FoundationJourneyMilestoneKind::ConsultLocalNeed,
            &format!("interaction:{interaction_id}"),
        );
    }
}

pub(crate) fn record_farming(
    state: &mut RepositoryState,
    identity_key: &str,
    action: FarmingAction,
    request_id: &str,
) {
    match action {
        FarmingAction::Plant => {
            if !has_credit(
                state,
                identity_key,
                FoundationJourneyMilestoneKind::PlantCommonField,
            ) {
                credit(
                    state,
                    identity_key,
                    FoundationJourneyMilestoneKind::PlantCommonField,
                    &format!("farming:{request_id}"),
                );
            } else if has_credit(
                state,
                identity_key,
                FoundationJourneyMilestoneKind::HarvestCommonField,
            ) {
                credit(
                    state,
                    identity_key,
                    FoundationJourneyMilestoneKind::ReplantCommonField,
                    &format!("farming:{request_id}"),
                );
            }
        }
        FarmingAction::Harvest => {
            let future_active = state.identities.get(identity_key).is_some_and(|identity| {
                identity.foundation_journey.future_goal_state
                    == FoundationJourneyFutureGoalState::Active
            });
            if future_active {
                complete_future_goal(state, identity_key);
            } else {
                credit(
                    state,
                    identity_key,
                    FoundationJourneyMilestoneKind::HarvestCommonField,
                    &format!("farming:{request_id}"),
                );
            }
        }
        FarmingAction::Tend | FarmingAction::TendAnimal => {}
    }
}

pub(super) fn record_resource(
    state: &mut RepositoryState,
    identity_key: &str,
    action: FoundationResourceAction,
    request_id: &str,
) {
    let (exploration, work) = match action {
        FoundationResourceAction::Log => (
            FoundationJourneyMilestoneKind::ExploreWoodland,
            FoundationJourneyMilestoneKind::GatherTimber,
        ),
        FoundationResourceAction::Mine => (
            FoundationJourneyMilestoneKind::ExploreStoneSeam,
            FoundationJourneyMilestoneKind::MineStone,
        ),
    };
    credit(
        state,
        identity_key,
        exploration,
        &format!("resource:{request_id}:arrival"),
    );
    credit(state, identity_key, work, &format!("resource:{request_id}"));
}

pub(super) fn record_forge(
    state: &mut RepositoryState,
    identity_key: &str,
    action: tarrowyn_protocol::FoundationForgeAction,
    request_id: &str,
) {
    if action == tarrowyn_protocol::FoundationForgeAction::ForgeFieldTool {
        credit(
            state,
            identity_key,
            FoundationJourneyMilestoneKind::ForgeFieldTool,
            &format!("forge:{request_id}"),
        );
    }
}

pub(crate) fn record_trade(state: &mut RepositoryState, trade: &TradeOffer) {
    let identity_keys = state
        .identities
        .iter()
        .filter(|(_, identity)| {
            identity.account_id == trade.creator_account_id
                || identity.account_id == trade.recipient_account_id
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    for identity_key in identity_keys {
        credit(
            state,
            &identity_key,
            FoundationJourneyMilestoneKind::CompleteBarter,
            &format!("trade:{}", trade.trade_id),
        );
    }
}

pub(super) fn record_storehouse(
    state: &mut RepositoryState,
    identity_key: &str,
    action: FoundationStorehouseAction,
    request_id: &str,
) {
    if action == FoundationStorehouseAction::Contribute {
        credit(
            state,
            identity_key,
            FoundationJourneyMilestoneKind::ContributeStorehouse,
            &format!("storehouse:{request_id}"),
        );
    }
}

pub(super) fn projection(progress: &FoundationJourneyProgress) -> FoundationJourneyProjection {
    let contract = FoundationJourneyContract::default();
    let next_milestone = contract
        .milestones
        .iter()
        .find(|milestone| {
            !progress
                .credits
                .iter()
                .any(|credit| credit.milestone_id == milestone.milestone_id)
        })
        .cloned();
    let next_action = if let Some(milestone) = &next_milestone {
        milestone.direction.clone()
    } else if progress.future_goal_state == FoundationJourneyFutureGoalState::Active {
        contract.future_goal.direction.clone()
    } else if progress.future_goal_state == FoundationJourneyFutureGoalState::Complete {
        "The return harvest is complete; choose your next work at the Beacon.".to_owned()
    } else {
        "Reassess the First Beacon journey state.".to_owned()
    };
    FoundationJourneyProjection {
        completed_milestones: progress.credits.len().min(u16::MAX as usize) as u16,
        total_milestones: contract.milestones.len().min(u16::MAX as usize) as u16,
        contract,
        progress: progress.clone(),
        next_milestone,
        next_action,
    }
}

impl super::super::WorldRepository {
    pub fn foundation_journey(
        &self,
        token: &str,
    ) -> Result<ApiResponse<FoundationJourneyProjection>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        let identity_key = super::super::authenticate(&mut state, token, &self.config)?;
        let progress = &state
            .identities
            .get(&identity_key)
            .expect("identity exists")
            .foundation_journey;
        Ok(ApiResponse {
            meta: super::super::meta(state.tick, None, Some(state.cursor)),
            data: projection(progress),
        })
    }
}

fn credit(
    state: &mut RepositoryState,
    identity_key: &str,
    kind: FoundationJourneyMilestoneKind,
    evidence_ref: &str,
) {
    let contract = FoundationJourneyContract::default();
    let Some(definition) = contract
        .milestones
        .iter()
        .find(|milestone| milestone.kind == kind)
    else {
        return;
    };
    let tick = state.tick;
    let progress = &mut state
        .identities
        .get_mut(identity_key)
        .expect("identity exists")
        .foundation_journey;
    restore_progress(progress);
    if progress
        .credits
        .iter()
        .any(|credit| credit.milestone_id == definition.milestone_id)
        || progress.credits.len() >= MAX_JOURNEY_CREDITS
    {
        return;
    }
    progress.credits.push(FoundationJourneyMilestoneCredit {
        milestone_id: definition.milestone_id.clone(),
        evidence_kind: definition.evidence_kind,
        evidence_ref: evidence_ref.to_owned(),
        credited_tick: tick,
    });
    progress.revision = progress.revision.saturating_add(1);
    if progress.credits.len() == contract.milestones.len() {
        progress.completed_tick = Some(tick);
        progress.future_goal_state = FoundationJourneyFutureGoalState::Active;
    }
}

fn has_credit(
    state: &RepositoryState,
    identity_key: &str,
    kind: FoundationJourneyMilestoneKind,
) -> bool {
    let contract = FoundationJourneyContract::default();
    let Some(id) = contract
        .milestones
        .iter()
        .find(|milestone| milestone.kind == kind)
        .map(|milestone| milestone.milestone_id.as_str())
    else {
        return false;
    };
    state.identities.get(identity_key).is_some_and(|identity| {
        identity
            .foundation_journey
            .credits
            .iter()
            .any(|credit| credit.milestone_id == id)
    })
}

fn complete_future_goal(state: &mut RepositoryState, identity_key: &str) {
    let tick = state.tick;
    let progress = &mut state
        .identities
        .get_mut(identity_key)
        .expect("identity exists")
        .foundation_journey;
    if progress.future_goal_state != FoundationJourneyFutureGoalState::Active {
        return;
    }
    progress.future_goal_state = FoundationJourneyFutureGoalState::Complete;
    progress.future_goal_completed_tick = Some(tick);
    progress.revision = progress.revision.saturating_add(1);
}
