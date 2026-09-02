//! Durable work attribution for the fixed First Beacon cooperation goal.

use super::super::models::RepositoryState;
use tarrowyn_protocol::{
    FoundationCooperationAttempt, FoundationCooperationContribution, FoundationCooperationResult,
    FoundationCooperationWorkCredit, FoundationCooperationWorkKind, FoundationForgeAction,
    FoundationForgeMaterialAmount, FoundationForgeMaterialKind, FoundationResourceAction,
    FoundationResourceAmount, FoundationResourceKind, TradeOffer,
};

const MAX_RECENT_WORK: usize = 64;
const MAX_ACTIVE_ATTEMPTS: usize = 8;

pub(super) fn mining_is_efficient(state: &RepositoryState, identity_key: &str) -> bool {
    state
        .identities
        .get(identity_key)
        .is_some_and(|identity| super::super::skills::mastery(&identity.skills, "mining") >= 2)
}

pub(super) fn record_gather(
    state: &mut RepositoryState,
    identity_key: &str,
    action: FoundationResourceAction,
    yields: &[FoundationResourceAmount],
) {
    let kind = match action {
        FoundationResourceAction::Log => FoundationCooperationWorkKind::Log,
        FoundationResourceAction::Mine => FoundationCooperationWorkKind::Mine,
    };
    let materials = yields
        .iter()
        .filter_map(|yielded| {
            let kind = match yielded.kind {
                FoundationResourceKind::Timber => FoundationForgeMaterialKind::Timber,
                FoundationResourceKind::IronOre => FoundationForgeMaterialKind::IronOre,
                FoundationResourceKind::Stone => return None,
            };
            Some(FoundationForgeMaterialAmount {
                kind,
                amount: yielded.amount,
            })
        })
        .collect();
    record_work(state, identity_key, kind, materials);
}

pub(crate) fn record_trade(state: &mut RepositoryState, trade: &TradeOffer) {
    let required_ore = required_amount(state, FoundationForgeMaterialKind::IronOre);
    let required_timber = required_amount(state, FoundationForgeMaterialKind::Timber);
    if trade.offer.iron_ore < required_ore || required_ore == 0 || required_timber == 0 {
        return;
    }
    let coordinator_has_timber = state.identities.values().any(|identity| {
        identity.account_id == trade.recipient_account_id
            && identity.inventory.timber >= required_timber
    });
    if !coordinator_has_timber {
        return;
    }
    let Some(miner_tick) = recent_credit_tick(
        state,
        &trade.creator_account_id,
        FoundationCooperationWorkKind::Mine,
        FoundationForgeMaterialKind::IronOre,
        required_ore,
    ) else {
        return;
    };
    let Some(logger_tick) = recent_credit_tick(
        state,
        &trade.recipient_account_id,
        FoundationCooperationWorkKind::Log,
        FoundationForgeMaterialKind::Timber,
        required_timber,
    ) else {
        return;
    };
    let contributions = vec![
        FoundationCooperationContribution {
            account_id: trade.recipient_account_id.clone(),
            materials: vec![material(
                FoundationForgeMaterialKind::Timber,
                required_timber,
            )],
            work_actions: 1,
        },
        FoundationCooperationContribution {
            account_id: trade.creator_account_id.clone(),
            materials: vec![material(FoundationForgeMaterialKind::IronOre, required_ore)],
            work_actions: 1,
        },
    ];
    let attempt = FoundationCooperationAttempt {
        coordinator_account_id: trade.recipient_account_id.clone(),
        participant_account_ids: vec![
            trade.recipient_account_id.clone(),
            trade.creator_account_id.clone(),
        ],
        contributions,
        trade_id: trade.trade_id.clone(),
        work_actions: 2,
        started_tick: miner_tick.max(logger_tick).max(state.tick),
    };
    state
        .foundation_activity
        .cooperation
        .recent_work
        .retain(|credit| {
            !((credit.account_id == trade.creator_account_id
                && credit.kind == FoundationCooperationWorkKind::Mine)
                || (credit.account_id == trade.recipient_account_id
                    && credit.kind == FoundationCooperationWorkKind::Log))
        });
    let attempts = &mut state.foundation_activity.cooperation.active_attempts;
    attempts.retain(|existing| existing.coordinator_account_id != attempt.coordinator_account_id);
    attempts.push(attempt);
    if attempts.len() > MAX_ACTIVE_ATTEMPTS {
        attempts.remove(0);
    }
}

pub(crate) fn remove_account(state: &mut RepositoryState, account_id: &str) {
    let cooperation = &mut state.foundation_activity.cooperation;
    cooperation
        .recent_work
        .retain(|credit| credit.account_id != account_id);
    cooperation.active_attempts.retain(|attempt| {
        !attempt
            .participant_account_ids
            .iter()
            .any(|participant| participant == account_id)
    });
    if cooperation.latest_result.as_ref().is_some_and(|result| {
        result
            .participant_account_ids
            .iter()
            .any(|participant| participant == account_id)
    }) {
        cooperation.latest_result = None;
    }
}

pub(crate) fn migrate_account(
    state: &mut RepositoryState,
    old_account_id: &str,
    new_account_id: &str,
) {
    let cooperation = &mut state.foundation_activity.cooperation;
    for credit in &mut cooperation.recent_work {
        replace_account(&mut credit.account_id, old_account_id, new_account_id);
    }
    for attempt in &mut cooperation.active_attempts {
        replace_account(
            &mut attempt.coordinator_account_id,
            old_account_id,
            new_account_id,
        );
        for participant in &mut attempt.participant_account_ids {
            replace_account(participant, old_account_id, new_account_id);
        }
        for contribution in &mut attempt.contributions {
            replace_account(&mut contribution.account_id, old_account_id, new_account_id);
        }
    }
    if let Some(result) = &mut cooperation.latest_result {
        replace_account(
            &mut result.coordinator_account_id,
            old_account_id,
            new_account_id,
        );
        for participant in &mut result.participant_account_ids {
            replace_account(participant, old_account_id, new_account_id);
        }
        for contribution in &mut result.contributions {
            replace_account(&mut contribution.account_id, old_account_id, new_account_id);
        }
    }
}

fn replace_account(value: &mut String, old_account_id: &str, new_account_id: &str) {
    if value == old_account_id {
        *value = new_account_id.to_owned();
    }
}

pub(super) fn record_forge(
    state: &mut RepositoryState,
    identity_key: &str,
    action: FoundationForgeAction,
) {
    let kind = match action {
        FoundationForgeAction::Inspect => return,
        FoundationForgeAction::BurnCharcoal => FoundationCooperationWorkKind::BurnCharcoal,
        FoundationForgeAction::ShapeHandle => FoundationCooperationWorkKind::ShapeHandle,
        FoundationForgeAction::ForgeFieldTool => FoundationCooperationWorkKind::ForgeFieldTool,
    };
    let account_id = state
        .identities
        .get(identity_key)
        .expect("identity exists")
        .account_id
        .clone();
    record_work(state, identity_key, kind, Vec::new());
    let Some(index) = state
        .foundation_activity
        .cooperation
        .active_attempts
        .iter()
        .position(|attempt| attempt.coordinator_account_id == account_id)
    else {
        return;
    };
    let attempt = &mut state.foundation_activity.cooperation.active_attempts[index];
    attempt.work_actions = attempt.work_actions.saturating_add(1);
    if let Some(contribution) = attempt
        .contributions
        .iter_mut()
        .find(|contribution| contribution.account_id == account_id)
    {
        contribution.work_actions = contribution.work_actions.saturating_add(1);
    }
    if action != FoundationForgeAction::ForgeFieldTool {
        return;
    }
    let attempt = state
        .foundation_activity
        .cooperation
        .active_attempts
        .remove(index);
    let solo = state.foundation_activity.cooperation.goal.solo_work_actions;
    if attempt.work_actions > solo {
        return;
    }
    state.foundation_activity.cooperation.latest_result = Some(FoundationCooperationResult {
        coordinator_account_id: attempt.coordinator_account_id,
        participant_account_ids: attempt.participant_account_ids,
        contributions: attempt.contributions,
        trade_id: attempt.trade_id,
        work_actions: attempt.work_actions,
        saved_work_actions: solo.saturating_sub(attempt.work_actions),
        completed_tick: state.tick,
    });
}

fn record_work(
    state: &mut RepositoryState,
    identity_key: &str,
    kind: FoundationCooperationWorkKind,
    materials: Vec<FoundationForgeMaterialAmount>,
) {
    let account_id = state
        .identities
        .get(identity_key)
        .expect("identity exists")
        .account_id
        .clone();
    let work = &mut state.foundation_activity.cooperation.recent_work;
    work.push(FoundationCooperationWorkCredit {
        account_id,
        kind,
        materials,
        tick: state.tick,
    });
    if work.len() > MAX_RECENT_WORK {
        work.remove(0);
    }
}

fn recent_credit_tick(
    state: &RepositoryState,
    account_id: &str,
    kind: FoundationCooperationWorkKind,
    material_kind: FoundationForgeMaterialKind,
    amount: u32,
) -> Option<u64> {
    let after_tick = state
        .foundation_activity
        .cooperation
        .latest_result
        .as_ref()
        .map_or(0, |result| result.completed_tick);
    state
        .foundation_activity
        .cooperation
        .recent_work
        .iter()
        .rev()
        .find(|credit| {
            credit.account_id == account_id
                && credit.kind == kind
                && credit.tick >= after_tick
                && credit
                    .materials
                    .iter()
                    .any(|material| material.kind == material_kind && material.amount >= amount)
        })
        .map(|credit| credit.tick)
}

fn required_amount(state: &RepositoryState, kind: FoundationForgeMaterialKind) -> u32 {
    state
        .foundation_activity
        .cooperation
        .goal
        .required_inputs
        .iter()
        .find(|material| material.kind == kind)
        .map_or(0, |material| material.amount)
}

fn material(kind: FoundationForgeMaterialKind, amount: u32) -> FoundationForgeMaterialAmount {
    FoundationForgeMaterialAmount { kind, amount }
}
