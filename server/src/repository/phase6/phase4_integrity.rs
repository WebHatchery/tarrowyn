use super::super::models::RepositoryState;
use crate::config::ServerConfig;
use std::collections::HashSet;

const DELETED_ACCOUNT: &str = "former-resident";
const MAX_TAX_RATE_PERCENT: u8 = 10;
const MAX_HOUSEHOLD_TEXT_CHARS: usize = 80;
const MAX_HOUSEHOLD_MEMBERS: usize = 20;

pub(super) fn ok(state: &RepositoryState, config: &ServerConfig) -> bool {
    let account_ids: HashSet<&str> = state
        .identities
        .values()
        .map(|identity| identity.account_id.as_str())
        .collect();
    let identity_keys: HashSet<&str> = state.identities.keys().map(String::as_str).collect();
    let settlement_ids: HashSet<&str> = state
        .phase5
        .settlements
        .iter()
        .map(|settlement| settlement.settlement_id.as_str())
        .collect();
    let knowledge_ids: HashSet<&str> = state
        .phase4
        .knowledge
        .iter()
        .map(|item| item.knowledge_id.as_str())
        .collect();

    let sequence_ok = state.phase4.next_lesson_id > 0
        && state.phase4.next_tax_id > 0
        && state.phase4.next_proposal_id > 0
        && state.phase4.next_decision_id > 0
        && state.phase4.next_order_id > 0
        && state.phase4.next_claim_id > 0
        && state.phase4.next_knowledge_id > 0;

    let governance = &state.phase4.governance;
    let governance_ok = governance.cursor <= state.cursor
        && !governance.offices.is_empty()
        && settlement_ids.contains(governance.settlement_id.as_str())
        && unique_non_empty(
            governance
                .offices
                .iter()
                .map(|office| office.office_id.as_str()),
        )
        && governance.offices.iter().all(|office| {
            optional_account_reference_ok(office.holder_account_id.as_deref(), &account_ids)
                && office.last_active_tick <= state.tick
                && office.vacant == office.holder_account_id.is_none()
        })
        && unique_non_empty(
            governance
                .proposals
                .iter()
                .map(|proposal| proposal.proposal_id.as_str()),
        )
        && governance.proposals.iter().all(|proposal| {
            account_reference_ok(&proposal.proposer_account_id, &account_ids)
                && optional_account_reference_ok(proposal.approved_by.as_deref(), &account_ids)
                && !proposal.target.trim().is_empty()
                && proposal.cost > 0
                && proposal.created_tick <= state.tick
                && proposal
                    .completed_tick
                    .is_none_or(|tick| tick >= proposal.created_tick && tick <= state.tick)
        })
        && unique_non_empty(
            governance
                .decisions
                .iter()
                .map(|decision| decision.decision_id.as_str()),
        )
        && governance.decisions.iter().all(|decision| {
            account_reference_ok(&decision.actor_account_id, &account_ids)
                && !decision.proposal_id.trim().is_empty()
                && !decision.service_affected.trim().is_empty()
                && decision.cost > 0
        })
        && governance
            .taxation
            .as_ref()
            .is_none_or(|policy| policy.rate_percent <= MAX_TAX_RATE_PERCENT)
        && unique_non_empty(
            governance
                .tax_ledger
                .iter()
                .map(|receipt| receipt.collection_id.as_str()),
        )
        && governance.tax_ledger.iter().all(|receipt| {
            account_reference_ok(&receipt.payer_account_id, &account_ids)
                && receipt.amount > 0
                && receipt.rate_percent <= MAX_TAX_RATE_PERCENT
        })
        && governance.administration_quality <= 100;

    let infrastructure_ok = !state.phase4.infrastructure.is_empty()
        && unique_non_empty(
            state
                .phase4
                .infrastructure
                .iter()
                .map(|record| record.infrastructure_id.as_str()),
        )
        && state.phase4.infrastructure.iter().all(|record| {
            position_in_world(record.position, config)
                && record.condition <= 100
                && record.service_quality <= 100
                && record.status == super::super::phase4::infrastructure_status(record.condition)
                && record.last_maintained_tick <= state.tick
        });

    let claims_ok = unique_non_empty(
        state
            .phase4
            .claims
            .iter()
            .map(|claim| claim.claim_id.as_str()),
    ) && unique_non_empty(
        state
            .phase4
            .claims
            .iter()
            .map(|claim| claim.plot_id.as_str()),
    ) && state.phase4.claims.iter().all(|claim| {
        let active_status = matches!(
            claim.status,
            tarrowyn_protocol::ClaimLifecycleStatus::Active
                | tarrowyn_protocol::ClaimLifecycleStatus::Renewed
                | tarrowyn_protocol::ClaimLifecycleStatus::Transferred
                | tarrowyn_protocol::ClaimLifecycleStatus::Inherited
        );
        !claim.plot_id.trim().is_empty()
            && position_in_world(claim.position, config)
            && optional_account_reference_ok(claim.owner_account_id.as_deref(), &account_ids)
            && optional_account_reference_ok(claim.approved_by.as_deref(), &account_ids)
            && claim.lease_days > 0
            && claim.started_tick <= state.tick
            && claim.expires_tick >= claim.started_tick
            && claim.last_active_tick <= state.tick
            && claim.building_access == active_status
            && (claim.owner_account_id.is_some()
                || claim.status == tarrowyn_protocol::ClaimLifecycleStatus::Reclaimed)
            && (claim.status != tarrowyn_protocol::ClaimLifecycleStatus::Requested
                || claim.approved_by.is_none())
    });

    let households_ok = !state.phase4.households.is_empty()
        && unique_non_empty(
            state
                .phase4
                .households
                .iter()
                .map(|household| household.household_id.as_str()),
        )
        && state.phase4.households.iter().all(|household| {
            bounded_text(&household.household_id, MAX_HOUSEHOLD_TEXT_CHARS)
                && bounded_text(&household.household_name, MAX_HOUSEHOLD_TEXT_CHARS)
                && !household.members.is_empty()
                && household.members.len() <= MAX_HOUSEHOLD_MEMBERS
                && household.members.iter().all(|member| {
                    bounded_text(&member.name, MAX_HOUSEHOLD_TEXT_CHARS)
                        && bounded_text(&member.role, MAX_HOUSEHOLD_TEXT_CHARS)
                        && bounded_text(&member.service, MAX_HOUSEHOLD_TEXT_CHARS)
                })
                && !household.home.trim().is_empty()
                && household.service_quality <= 100
                && household.demand <= 100
                && household.housing <= 100
                && household.safety <= 100
                && household.food <= 100
                && household.competition <= 100
                && household.last_decision_tick <= state.tick
        });

    let orders_ok = unique_non_empty(
        state
            .phase4
            .orders
            .iter()
            .map(|order| order.order_id.as_str()),
    ) && state.phase4.orders.iter().all(|order| {
        account_reference_ok(&order.requester_account_id, &account_ids)
            && optional_account_reference_ok(order.provider_account_id.as_deref(), &account_ids)
            && !order.service.trim().is_empty()
            && order.quality <= 100
    });

    let lessons_ok = unique_non_empty(
        state
            .phase4
            .lessons
            .iter()
            .map(|lesson| lesson.lesson_id.as_str()),
    ) && state.phase4.lessons.iter().all(|lesson| {
        account_ids.contains(lesson.teacher_account_id.as_str())
            && account_ids.contains(lesson.learner_account_id.as_str())
            && lesson.teacher_account_id != lesson.learner_account_id
    });

    let knowledge_ok = !state.phase4.knowledge.is_empty()
        && unique_non_empty(
            state
                .phase4
                .knowledge
                .iter()
                .map(|item| item.knowledge_id.as_str()),
        )
        && state.phase4.knowledge.iter().all(|item| {
            unique_non_empty(item.discovered_by.iter().map(String::as_str))
                && item
                    .discovered_by
                    .iter()
                    .all(|account_id| account_ids.contains(account_id.as_str()))
                && !item.stored_in.trim().is_empty()
        })
        && state.phase4.known_by.iter().all(|(identity_key, known)| {
            identity_keys.contains(identity_key.as_str())
                && unique_non_empty(known.iter().map(String::as_str))
                && known
                    .iter()
                    .all(|knowledge_id| knowledge_ids.contains(knowledge_id.as_str()))
        });

    let identity_keyed_state_ok = state
        .phase4
        .profiles
        .keys()
        .chain(state.phase4.materials.keys())
        .chain(state.phase4.credentials.keys())
        .chain(state.phase4.known_by.keys())
        .chain(state.phase4.combat.keys())
        .all(|identity_key| identity_keys.contains(identity_key.as_str()));
    let profiles_ok = state.phase4.profiles.values().all(|profiles| {
        profiles.iter().all(|profile| {
            unique_non_empty(
                profile
                    .capabilities
                    .iter()
                    .map(|capability| capability.capability_id.as_str()),
            )
        })
    });
    let animals_ok = !state.phase4.animals.is_empty()
        && unique_non_empty(
            state
                .phase4
                .animals
                .iter()
                .map(|animal| animal.animal_id.as_str()),
        )
        && state
            .phase4
            .animals
            .iter()
            .all(|animal| animal.max_condition > 0 && animal.condition <= animal.max_condition);
    let available_plots_ok = unique_positions(
        state
            .phase4
            .available_plots
            .iter()
            .map(|position| (position.x, position.y)),
    ) && state
        .phase4
        .available_plots
        .iter()
        .all(|position| position_in_world(*position, config));

    sequence_ok
        && governance_ok
        && infrastructure_ok
        && claims_ok
        && households_ok
        && orders_ok
        && lessons_ok
        && knowledge_ok
        && identity_keyed_state_ok
        && profiles_ok
        && animals_ok
        && available_plots_ok
}

fn account_reference_ok(account_id: &str, account_ids: &HashSet<&str>) -> bool {
    !account_id.trim().is_empty()
        && (account_ids.contains(account_id) || account_id == DELETED_ACCOUNT)
}

fn optional_account_reference_ok(account_id: Option<&str>, account_ids: &HashSet<&str>) -> bool {
    account_id.is_none_or(|account_id| account_reference_ok(account_id, account_ids))
}

fn unique_non_empty<'a>(mut values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = HashSet::new();
    values.all(|value| !value.trim().is_empty() && seen.insert(value))
}

fn unique_positions(mut positions: impl Iterator<Item = (i32, i32)>) -> bool {
    let mut seen = HashSet::new();
    positions.all(|position| seen.insert(position))
}

fn position_in_world(position: tarrowyn_protocol::Position, config: &ServerConfig) -> bool {
    position.x >= 0
        && position.y >= 0
        && (position.x as u32) < config.world_width
        && (position.y as u32) < config.world_height
}

fn bounded_text(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}
