use super::super::models::RepositoryState;
use crate::config::ServerConfig;
use std::collections::HashSet;
use tarrowyn_protocol::{
    ClaimStatus, ContractStatus, ExpeditionStatus, FoundationResourceKind,
    FoundationStorehouseContributionInput, FoundationStorehouseStage, FoundationStorehouseState,
    TradeStatus,
};

const DELETED_ACCOUNT: &str = "former-resident";
const MAX_EXPEDITION_SUPPLY: u32 = 99;
const MAX_ACCOUNT_ID_CHARS: usize = 160;
const MAX_DISPLAY_NAME_CHARS: usize = 80;
const MAX_PHASE3_NAME_CHARS: usize = 80;
const MAX_CHRONICLE_ID_CHARS: usize = 160;
const MAX_CHRONICLE_KIND_CHARS: usize = 80;
const MAX_PHASE3_HOUSEHOLD_MEMBERS: usize = 20;

pub(super) fn ok(state: &RepositoryState, config: &ServerConfig) -> bool {
    let account_ids: HashSet<&str> = state
        .identities
        .values()
        .map(|identity| identity.account_id.as_str())
        .collect();
    core_ok(state, config, &account_ids) && phase3_ok(state, config, &account_ids)
}

fn core_ok(state: &RepositoryState, config: &ServerConfig, account_ids: &HashSet<&str>) -> bool {
    let sequence_ok = state.next_guest > 0
        && state.next_message > 0
        && state.next_token > 0
        && state.next_trade > 0
        && state.next_notice > 0;
    let clock_ok = state.clock.day > 0
        && state.clock.day_length_seconds.is_finite()
        && state.clock.day_length_seconds > 0.0
        && state.clock.seconds.is_finite()
        && state.clock.seconds >= 0.0
        && state.clock.seconds < state.clock.day_length_seconds;
    let identities_ok = state.identities.iter().all(|(identity_key, identity)| {
        bounded_text(identity_key, super::super::MAX_CLIENT_KEY_CHARS)
            && bounded_text(&identity.account_id, MAX_ACCOUNT_ID_CHARS)
            && bounded_text(&identity.character_id, MAX_ACCOUNT_ID_CHARS)
            && bounded_text(&identity.display_name, MAX_DISPLAY_NAME_CHARS)
            && position_in_world(identity.position, config)
            && identity.field_tool_condition <= identity.field_tool_kind.max_condition()
            && identity.injuries <= 3
            && (!identity.knocked_out || identity.injuries > 0)
            && identity.last_seen_tick <= state.tick
            && identity.last_tax_day <= state.clock.day
            && super::super::skills::skill_ledger_integrity_ok(&identity.skills)
    });
    let plots_ok = unique_positions(state.plots.iter().map(|plot| plot.position))
        && state.plots.iter().all(|plot| {
            position_in_world(plot.position, config)
                && plot.crop.is_none_or(|crop| {
                    crop.stage <= tarrowyn_protocol::CropState::MATURE_STAGE
                        && crop.quality <= 100
                        && crop.planted_tick <= state.tick
                        && crop.last_tended_tick.is_none_or(|tick| tick <= state.tick)
                })
        });
    let events_ok = ordered_event_cursors(state);
    let chat_ok = state.chat_history.len() <= super::super::MAX_CHAT_HISTORY
        && unique_nonzero(state.chat_history.iter().map(|message| message.message_id))
        && state.chat_history.iter().all(|message| {
            account_reference_ok(&message.account_id, account_ids)
                && bounded_text(&message.display_name, MAX_DISPLAY_NAME_CHARS)
                && bounded_text(&message.channel, 24)
                && !message.text.trim().is_empty()
                && message.text.chars().count() <= tarrowyn_protocol::MAX_CHAT_MESSAGE_LENGTH
                && !message.text.chars().any(char::is_control)
                && cursor_in_world(message.cursor, state.cursor)
        });
    let notices_ok = state.notices.len() <= super::super::MAX_NOTICES
        && unique_nonzero(state.notices.iter().map(|notice| notice.notice_id))
        && state.notices.iter().all(|notice| {
            bounded_text(&notice.kind, 80)
                && bounded_text(&notice.text, 512)
                && notice.created_tick <= state.tick
                && cursor_in_world(notice.cursor, state.cursor)
        });
    let trades_ok = state.trades.len() <= super::super::MAX_TRADES
        && unique_non_empty(state.trades.values().map(|trade| trade.trade_id.as_str()))
        && state.trades.iter().all(|(key, trade)| {
            key == &trade.trade_id
                && account_reference_ok(&trade.creator_account_id, account_ids)
                && account_reference_ok(&trade.recipient_account_id, account_ids)
                && trade.creator_account_id != trade.recipient_account_id
                && !(trade.offer.is_empty() && trade.request.is_empty())
                && trade.offer.item_count() <= tarrowyn_protocol::MAX_TRADE_ITEMS
                && trade.request.item_count() <= tarrowyn_protocol::MAX_TRADE_ITEMS
                && trade.offer.gold <= 10_000
                && trade.request.gold <= 10_000
                && trade.expires_tick >= trade.created_tick
                && matches!(
                    trade.status,
                    TradeStatus::Pending
                        | TradeStatus::Accepted
                        | TradeStatus::Cancelled
                        | TradeStatus::Expired
                )
        });
    let cooperation = &state.foundation_activity.cooperation;
    let recent_work_ok = cooperation.recent_work.len() <= 64
        && cooperation.recent_work.iter().all(|credit| {
            account_reference_ok(&credit.account_id, account_ids)
                && credit.tick <= state.tick
                && credit.materials.iter().all(|material| material.amount > 0)
        });
    let active_attempts_ok = cooperation.active_attempts.len() <= 8
        && unique_non_empty(
            cooperation
                .active_attempts
                .iter()
                .map(|attempt| attempt.coordinator_account_id.as_str()),
        )
        && cooperation.active_attempts.iter().all(|attempt| {
            account_reference_ok(&attempt.coordinator_account_id, account_ids)
                && attempt.participant_account_ids.len() == 2
                && unique_non_empty(attempt.participant_account_ids.iter().map(String::as_str))
                && attempt
                    .participant_account_ids
                    .iter()
                    .all(|account_id| account_reference_ok(account_id, account_ids))
                && attempt
                    .participant_account_ids
                    .contains(&attempt.coordinator_account_id)
                && attempt.contributions.len() == attempt.participant_account_ids.len()
                && attempt.contributions.iter().all(|contribution| {
                    attempt
                        .participant_account_ids
                        .contains(&contribution.account_id)
                        && !contribution.materials.is_empty()
                        && contribution
                            .materials
                            .iter()
                            .all(|material| material.amount > 0)
                })
                && attempt
                    .contributions
                    .iter()
                    .map(|contribution| u16::from(contribution.work_actions))
                    .sum::<u16>()
                    == u16::from(attempt.work_actions)
                && !attempt.trade_id.trim().is_empty()
                && state.trades.get(&attempt.trade_id).is_none_or(|trade| {
                    trade.status == TradeStatus::Accepted
                        && trade.recipient_account_id == attempt.coordinator_account_id
                })
                && attempt.work_actions <= cooperation.goal.solo_work_actions
                && attempt.started_tick <= state.tick
        });
    let cooperation_ok = cooperation.goal == Default::default()
        && recent_work_ok
        && active_attempts_ok
        && cooperation.latest_result.as_ref().is_none_or(|result| {
            account_reference_ok(&result.coordinator_account_id, account_ids)
                && result.participant_account_ids.len() >= 2
                && result.participant_account_ids.len() <= 8
                && unique_non_empty(result.participant_account_ids.iter().map(String::as_str))
                && result
                    .participant_account_ids
                    .iter()
                    .all(|account_id| account_reference_ok(account_id, account_ids))
                && result
                    .participant_account_ids
                    .contains(&result.coordinator_account_id)
                && result.contributions.len() == result.participant_account_ids.len()
                && unique_non_empty(
                    result
                        .contributions
                        .iter()
                        .map(|contribution| contribution.account_id.as_str()),
                )
                && result.contributions.iter().all(|contribution| {
                    result
                        .participant_account_ids
                        .contains(&contribution.account_id)
                        && !contribution.materials.is_empty()
                        && contribution
                            .materials
                            .iter()
                            .all(|material| material.amount > 0)
                })
                && result
                    .contributions
                    .iter()
                    .map(|contribution| u16::from(contribution.work_actions))
                    .sum::<u16>()
                    == u16::from(result.work_actions)
                && !result.trade_id.trim().is_empty()
                && result.work_actions <= cooperation.goal.solo_work_actions
                && result.saved_work_actions
                    == cooperation
                        .goal
                        .solo_work_actions
                        .saturating_sub(result.work_actions)
                && result.completed_tick <= state.tick
        });
    let storehouse_ok = storehouse_ok(state, account_ids);

    sequence_ok
        && clock_ok
        && identities_ok
        && plots_ok
        && events_ok
        && chat_ok
        && notices_ok
        && trades_ok
        && cooperation_ok
        && storehouse_ok
}

fn storehouse_ok(state: &RepositoryState, account_ids: &HashSet<&str>) -> bool {
    let project = &state.foundation_activity.storehouse;
    let expected = FoundationStorehouseState::default();
    let fixed_contract_ok = project.project_id == expected.project_id
        && project.title == expected.title
        && project.builder_landmark_id == expected.builder_landmark_id
        && project.noticeboard_landmark_id == expected.noticeboard_landmark_id
        && project.site_landmark_id == expected.site_landmark_id
        && project.operational_infrastructure_id == expected.operational_infrastructure_id
        && project.requirements == expected.requirements
        && project.stages == expected.stages
        && project.revision > 0;
    let contributions_ok = project.contributions.len()
        <= super::super::foundation::storehouse::MAX_CONTRIBUTIONS
        && unique_non_empty(
            project
                .contributions
                .iter()
                .map(|contribution| contribution.contribution_id.as_str()),
        )
        && project.contributions.iter().all(|contribution| {
            account_reference_ok(&contribution.account_id, account_ids)
                && contribution.credited_units > 0
                && contribution.contributed_tick <= state.tick
                && match contribution.input {
                    FoundationStorehouseContributionInput::Material { kind, amount } => {
                        kind == contribution.credited_kind
                            && amount == contribution.credited_units
                            && matches!(
                                kind,
                                FoundationResourceKind::Timber | FoundationResourceKind::Stone
                            )
                    }
                    FoundationStorehouseContributionInput::Gold { toward, amount } => project
                        .requirements
                        .iter()
                        .find(|requirement| requirement.kind == toward)
                        .is_some_and(|requirement| {
                            toward == contribution.credited_kind
                                && contribution
                                    .credited_units
                                    .checked_mul(requirement.gold_per_unit)
                                    == Some(amount)
                        }),
                }
        });
    let totals_ok = project
        .requirements
        .iter()
        .all(|requirement| credited_units(project, requirement.kind) <= requirement.units_required);
    let stage_ok =
        project.current_stage == super::super::foundation::storehouse::stage_for(project);
    let infrastructure_count = state
        .phase4
        .infrastructure
        .iter()
        .filter(|record| record.infrastructure_id == project.operational_infrastructure_id)
        .count();
    let completion_ok = match project.completion.as_ref() {
        None => {
            project.current_stage != FoundationStorehouseStage::Operational
                && infrastructure_count == 0
        }
        Some(completion) => {
            let mut contributors = Vec::new();
            for contribution in &project.contributions {
                if !contributors.contains(&contribution.account_id) {
                    contributors.push(contribution.account_id.clone());
                }
            }
            project.current_stage == FoundationStorehouseStage::Operational
                && completion.completed_tick <= state.tick
                && completion.operational_infrastructure_id == project.operational_infrastructure_id
                && !completion.contributor_account_ids.is_empty()
                && completion.contributor_account_ids == contributors
                && unique_non_empty(
                    completion
                        .contributor_account_ids
                        .iter()
                        .map(String::as_str),
                )
                && completion
                    .contributor_account_ids
                    .iter()
                    .all(|account_id| account_reference_ok(account_id, account_ids))
                && infrastructure_count == 1
        }
    };
    fixed_contract_ok && contributions_ok && totals_ok && stage_ok && completion_ok
}

fn credited_units(project: &FoundationStorehouseState, kind: FoundationResourceKind) -> u32 {
    project
        .contributions
        .iter()
        .filter(|contribution| contribution.credited_kind == kind)
        .fold(0_u32, |total, contribution| {
            total.saturating_add(contribution.credited_units)
        })
}

fn phase3_ok(state: &RepositoryState, config: &ServerConfig, account_ids: &HashSet<&str>) -> bool {
    let sequence_ok = state.phase3.next_event_id > 0;
    let threat = crate::content::threat_template("whisperwood-edge");
    let zone = &state.phase3.zone;
    let zone_ok = zone.zone_id == threat.id
        && zone.monster == threat.monster
        && zone.position == threat.position
        && zone.monster_health <= threat.monster_health
        && zone.threat_active == (zone.monster_health > 0)
        && zone.road_open == !zone.threat_active
        && (0..=100).contains(&zone.price_modifier_percent)
        && !zone.resource_demand.trim().is_empty()
        && !zone.rumour.trim().is_empty()
        && position_in_world(zone.position, config);
    let contracts_ok = state
        .phase3
        .contracts
        .iter()
        .all(|(identity_key, progress)| {
            state.identities.contains_key(identity_key)
                && progress.progress
                    <= crate::content::contract_template("brambleback-watch").required_progress
                && matches!(
                    progress.status,
                    ContractStatus::Available
                        | ContractStatus::Accepted
                        | ContractStatus::Completed
                        | ContractStatus::Cooldown
                )
        });
    let households_ok = unique_non_empty(
        state
            .phase3
            .households
            .iter()
            .map(|household| household.household_id.as_str()),
    ) && state.phase3.households.iter().all(|household| {
        !household.household_name.trim().is_empty()
            && !household.members.is_empty()
            && household.members.len() <= MAX_PHASE3_HOUSEHOLD_MEMBERS
            && household.members.iter().all(|member| {
                bounded_text(&member.name, MAX_PHASE3_NAME_CHARS)
                    && bounded_text(&member.occupation, MAX_PHASE3_NAME_CHARS)
            })
            && !household.occupation.trim().is_empty()
            && !household.home_settlement.trim().is_empty()
            && (0..=100).contains(&household.opportunity_score)
            && !household.service.trim().is_empty()
            && !household.clue.trim().is_empty()
    });
    let claim_ok = state.phase3.claim.as_ref().is_none_or(|claim| {
        !claim.claim_id.trim().is_empty()
            && account_reference_ok(&claim.owner_account_id, account_ids)
            && position_in_world(claim.position, config)
            && claim.lease_days > 0
            && claim.last_active_tick <= state.tick
            && claim.reclaim_after_ticks > 0
            && matches!(
                claim.status,
                ClaimStatus::Active | ClaimStatus::Reclaimed | ClaimStatus::Abandoned
            )
    });
    let expedition_ok = state.phase3.expedition.as_ref().is_none_or(|expedition| {
        bounded_text(&expedition.expedition_id, MAX_PHASE3_NAME_CHARS)
            && bounded_text(&expedition.outpost_name, MAX_PHASE3_NAME_CHARS)
            && account_reference_ok(&expedition.leader_account_id, account_ids)
            && position_in_world(expedition.outpost_position, config)
            && expedition.food <= MAX_EXPEDITION_SUPPLY
            && expedition.tools <= MAX_EXPEDITION_SUPPLY
            && expedition.materials <= MAX_EXPEDITION_SUPPLY
            && expedition.safety <= MAX_EXPEDITION_SUPPLY
            && expedition.members.len() <= super::super::phase3::MAX_EXPEDITION_MEMBERS
            && unique_non_empty(
                expedition
                    .members
                    .iter()
                    .map(|member| member.account_id.as_str()),
            )
            && expedition.members.iter().all(|member| {
                account_reference_ok(&member.account_id, account_ids)
                    && bounded_text(&member.display_name, MAX_PHASE3_NAME_CHARS)
            })
            && (expedition
                .members
                .iter()
                .any(|member| member.account_id == expedition.leader_account_id)
                || (!matches!(
                    expedition.status,
                    ExpeditionStatus::Planning | ExpeditionStatus::Launched
                ) && expedition.leader_account_id == DELETED_ACCOUNT))
    });
    let credentials_ok = unique_non_empty(
        state
            .phase3
            .expedition_credentials
            .iter()
            .map(String::as_str),
    ) && state
        .phase3
        .expedition_credentials
        .iter()
        .all(|account_id| account_ids.contains(account_id.as_str()));
    let chronicle_ok = chronicle_entries_ok(state);
    let outpost_ok = state
        .phase3
        .outpost
        .is_none_or(|position| position_in_world(position, config));

    sequence_ok
        && zone_ok
        && contracts_ok
        && households_ok
        && claim_ok
        && expedition_ok
        && credentials_ok
        && chronicle_ok
        && outpost_ok
}

fn chronicle_entries_ok(state: &RepositoryState) -> bool {
    let mut event_ids = HashSet::new();
    let mut previous_cursor = 0;
    for entry in state
        .phase3
        .chronicle_archive
        .iter()
        .chain(state.phase3.chronicle.iter())
    {
        if entry.event_id.trim().is_empty()
            || !event_ids.insert(entry.event_id.as_str())
            || !bounded_text(&entry.event_id, MAX_CHRONICLE_ID_CHARS)
            || !bounded_text(&entry.kind, MAX_CHRONICLE_KIND_CHARS)
            || !bounded_text(&entry.title, super::MAX_CHRONICLE_TEXT_CHARS)
            || !bounded_text(&entry.text, super::MAX_CHRONICLE_TEXT_CHARS)
            || entry.created_tick > state.tick
            || entry.cursor <= previous_cursor
            || !cursor_in_world(entry.cursor, state.cursor)
        {
            return false;
        }
        previous_cursor = entry.cursor;
    }
    true
}

fn ordered_event_cursors(state: &RepositoryState) -> bool {
    let mut previous_cursor = 0;
    for event in &state.events {
        if !cursor_in_world(event.cursor, state.cursor) || event.cursor <= previous_cursor {
            return false;
        }
        previous_cursor = event.cursor;
    }
    true
}

fn cursor_in_world(cursor: u64, current: u64) -> bool {
    cursor > 0 && cursor <= current
}

fn position_in_world(position: tarrowyn_protocol::Position, config: &ServerConfig) -> bool {
    position.x >= 0
        && position.y >= 0
        && (position.x as u32) < config.world_width
        && (position.y as u32) < config.world_height
}

fn account_reference_ok(account_id: &str, account_ids: &HashSet<&str>) -> bool {
    !account_id.trim().is_empty()
        && (account_ids.contains(account_id) || account_id == DELETED_ACCOUNT)
}

fn bounded_text(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}

fn unique_non_empty<'a>(mut values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = HashSet::new();
    values.all(|value| !value.trim().is_empty() && seen.insert(value))
}

fn unique_nonzero(mut values: impl Iterator<Item = u64>) -> bool {
    let mut seen = HashSet::new();
    values.all(|value| value > 0 && seen.insert(value))
}

fn unique_positions(mut positions: impl Iterator<Item = tarrowyn_protocol::Position>) -> bool {
    let mut seen = HashSet::new();
    positions.all(|position| seen.insert((position.x, position.y)))
}
