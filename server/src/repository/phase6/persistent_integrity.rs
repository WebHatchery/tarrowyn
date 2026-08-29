use super::super::models::RepositoryState;
use crate::config::ServerConfig;
use std::collections::HashSet;
use tarrowyn_protocol::{ClaimStatus, ContractStatus, ExpeditionStatus, TradeStatus};

const DELETED_ACCOUNT: &str = "former-resident";
const MAX_EXPEDITION_SUPPLY: u32 = 99;
const MAX_PHASE3_NAME_CHARS: usize = 80;

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
    let identities_ok = state.identities.values().all(|identity| {
        !identity.display_name.trim().is_empty()
            && position_in_world(identity.position, config)
            && identity.field_tool_condition <= super::super::FIELD_TOOL_MAX_CONDITION
            && identity.injuries <= 3
            && (!identity.knocked_out || identity.injuries > 0)
            && identity.last_seen_tick <= state.tick
            && identity.last_tax_day <= state.clock.day
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
    let chat_ok = unique_nonzero(state.chat_history.iter().map(|message| message.message_id))
        && state.chat_history.iter().all(|message| {
            account_reference_ok(&message.account_id, account_ids)
                && !message.channel.trim().is_empty()
                && message.channel.chars().count() <= 24
                && !message.text.trim().is_empty()
                && message.text.chars().count() <= tarrowyn_protocol::MAX_CHAT_MESSAGE_LENGTH
                && !message.text.chars().any(char::is_control)
                && cursor_in_world(message.cursor, state.cursor)
        });
    let notices_ok = unique_nonzero(state.notices.iter().map(|notice| notice.notice_id))
        && state.notices.iter().all(|notice| {
            !notice.kind.trim().is_empty()
                && !notice.text.trim().is_empty()
                && cursor_in_world(notice.cursor, state.cursor)
        });
    let trades_ok = unique_non_empty(state.trades.values().map(|trade| trade.trade_id.as_str()))
        && state.trades.iter().all(|(key, trade)| {
            key == &trade.trade_id
                && account_reference_ok(&trade.creator_account_id, account_ids)
                && account_reference_ok(&trade.recipient_account_id, account_ids)
                && trade.creator_account_id != trade.recipient_account_id
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

    sequence_ok
        && clock_ok
        && identities_ok
        && plots_ok
        && events_ok
        && chat_ok
        && notices_ok
        && trades_ok
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
            || entry.kind.trim().is_empty()
            || entry.title.trim().is_empty()
            || entry.text.trim().is_empty()
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
