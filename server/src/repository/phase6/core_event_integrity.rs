use super::super::models::RepositoryState;
use crate::config::ServerConfig;
use std::collections::HashSet;
use tarrowyn_protocol::{
    ChatMessage, ChronicleEntry, EventRecord, Expedition, FarmPlot, FrontierEvent, LandClaim,
    OpportunitySignal, PlayerPresence, TavernNotice, TradeOffer, WorldClock, WorldEvent,
};

const DELETED_ACCOUNT: &str = "former-resident";
const MAX_ACCOUNT_ID_CHARS: usize = 160;
const MAX_DISPLAY_NAME_CHARS: usize = 80;
const MAX_EVENT_ID_CHARS: usize = 160;
const MAX_EVENT_TEXT_CHARS: usize = 512;
const MAX_FRONTIER_NAME_CHARS: usize = 80;
const MAX_EXPEDITION_SUPPLY: u32 = 99;

pub(super) fn ok(state: &RepositoryState, config: &ServerConfig) -> bool {
    if state.events.len() > super::super::MAX_EVENTS {
        return false;
    }
    let account_ids: HashSet<&str> = state
        .identities
        .values()
        .map(|identity| identity.account_id.as_str())
        .collect();
    state.events.iter().all(|record| {
        record.cursor > 0
            && record.cursor <= state.cursor
            && event_ok(record, state, config, &account_ids)
    })
}

fn event_ok(
    record: &EventRecord,
    state: &RepositoryState,
    config: &ServerConfig,
    account_ids: &HashSet<&str>,
) -> bool {
    match &record.event {
        WorldEvent::Presence(presence) => presence_ok(presence, state, config, account_ids),
        WorldEvent::Clock(clock) => clock_ok(clock, &state.clock),
        WorldEvent::Chat(message) => chat_ok(message, record.cursor, state, account_ids),
        WorldEvent::Farming(plot) => plot_ok(plot, state, config),
        WorldEvent::Trade(trade) => trade_ok(trade, state, account_ids),
        WorldEvent::TavernNotice(notice) => notice_ok(notice, record.cursor, state),
        WorldEvent::Chronicle(entry) => chronicle_ok(entry, record.cursor, state),
        WorldEvent::Frontier(frontier) => frontier_ok(frontier, state, config, account_ids),
    }
}

fn presence_ok(
    presence: &PlayerPresence,
    state: &RepositoryState,
    config: &ServerConfig,
    account_ids: &HashSet<&str>,
) -> bool {
    account_reference_ok(&presence.account_id, account_ids)
        && bounded(&presence.character_id, MAX_ACCOUNT_ID_CHARS)
        && bounded(&presence.display_name, MAX_DISPLAY_NAME_CHARS)
        && position_in_world(presence.position, config)
        && presence.last_seen_tick <= state.tick
}

fn clock_ok(clock: &WorldClock, current: &WorldClock) -> bool {
    clock.day > 0
        && clock.day_length_seconds.is_finite()
        && clock.day_length_seconds > 0.0
        && clock.seconds.is_finite()
        && clock.seconds >= 0.0
        && clock.seconds < clock.day_length_seconds
        && (clock.day < current.day
            || (clock.day == current.day && clock.seconds <= current.seconds))
}

fn chat_ok(
    message: &ChatMessage,
    record_cursor: u64,
    state: &RepositoryState,
    account_ids: &HashSet<&str>,
) -> bool {
    message.message_id > 0
        && account_reference_ok(&message.account_id, account_ids)
        && bounded(&message.display_name, MAX_DISPLAY_NAME_CHARS)
        && bounded(&message.channel, 24)
        && bounded(&message.text, tarrowyn_protocol::MAX_CHAT_MESSAGE_LENGTH)
        && message.cursor == record_cursor
        && message.cursor <= state.cursor
}

fn plot_ok(plot: &FarmPlot, state: &RepositoryState, config: &ServerConfig) -> bool {
    position_in_world(plot.position, config)
        && plot.crop.is_none_or(|crop| {
            crop.stage <= tarrowyn_protocol::CropState::MATURE_STAGE
                && crop.quality <= 100
                && crop.planted_tick <= state.tick
                && crop.last_tended_tick.is_none_or(|tick| tick <= state.tick)
        })
}

fn trade_ok(trade: &TradeOffer, state: &RepositoryState, account_ids: &HashSet<&str>) -> bool {
    bounded(&trade.trade_id, MAX_EVENT_ID_CHARS)
        && account_reference_ok(&trade.creator_account_id, account_ids)
        && bounded(&trade.creator_name, MAX_DISPLAY_NAME_CHARS)
        && account_reference_ok(&trade.recipient_account_id, account_ids)
        && bounded(&trade.recipient_name, MAX_DISPLAY_NAME_CHARS)
        && trade.creator_account_id != trade.recipient_account_id
        && !(trade.offer.is_empty() && trade.request.is_empty())
        && trade.offer.item_count() <= tarrowyn_protocol::MAX_TRADE_ITEMS
        && trade.request.item_count() <= tarrowyn_protocol::MAX_TRADE_ITEMS
        && trade.offer.gold <= 10_000
        && trade.request.gold <= 10_000
        && trade.created_tick <= state.tick
        && trade.expires_tick >= trade.created_tick
}

fn notice_ok(notice: &TavernNotice, record_cursor: u64, state: &RepositoryState) -> bool {
    notice.notice_id > 0
        && bounded(&notice.kind, 80)
        && bounded(&notice.text, MAX_EVENT_TEXT_CHARS)
        && notice.created_tick <= state.tick
        && notice.cursor == record_cursor
        && notice.cursor <= state.cursor
}

fn chronicle_ok(entry: &ChronicleEntry, record_cursor: u64, state: &RepositoryState) -> bool {
    bounded(&entry.event_id, MAX_EVENT_ID_CHARS)
        && bounded(&entry.kind, 80)
        && bounded(&entry.title, MAX_EVENT_TEXT_CHARS)
        && bounded(&entry.text, MAX_EVENT_TEXT_CHARS)
        && entry.created_tick <= state.tick
        && entry.cursor == record_cursor
        && entry.cursor <= state.cursor
}

fn frontier_ok(
    frontier: &FrontierEvent,
    state: &RepositoryState,
    config: &ServerConfig,
    account_ids: &HashSet<&str>,
) -> bool {
    match frontier {
        FrontierEvent::Threat(threat) => {
            let template = crate::content::threat_template("whisperwood-edge");
            bounded(&threat.zone_id, MAX_EVENT_ID_CHARS)
                && bounded(&threat.name, MAX_FRONTIER_NAME_CHARS)
                && threat.zone_id == template.id
                && threat.monster == template.monster
                && threat.position == template.position
                && threat.monster_health <= template.monster_health
                && threat.threat_active == (threat.monster_health > 0)
                && threat.road_open == !threat.threat_active
                && (0..=100).contains(&threat.price_modifier_percent)
                && bounded(&threat.resource_demand, MAX_EVENT_TEXT_CHARS)
                && bounded(&threat.rumour, MAX_EVENT_TEXT_CHARS)
                && position_in_world(threat.position, config)
        }
        FrontierEvent::Opportunity(opportunity) => opportunity_ok(opportunity),
        FrontierEvent::Claim(claim) => claim_ok(claim, state, config, account_ids),
        FrontierEvent::Expedition(expedition) => expedition_ok(expedition, config, account_ids),
    }
}

fn opportunity_ok(opportunity: &OpportunitySignal) -> bool {
    bounded(&opportunity.household_id, MAX_EVENT_ID_CHARS)
        && bounded(&opportunity.household_name, MAX_FRONTIER_NAME_CHARS)
        && !opportunity.members.is_empty()
        && opportunity.members.len() <= super::super::phase3::MAX_EXPEDITION_MEMBERS
        && opportunity.members.iter().all(|member| {
            bounded(&member.name, MAX_FRONTIER_NAME_CHARS)
                && bounded(&member.occupation, MAX_FRONTIER_NAME_CHARS)
        })
        && bounded(&opportunity.occupation, MAX_FRONTIER_NAME_CHARS)
        && bounded(&opportunity.home_settlement, MAX_FRONTIER_NAME_CHARS)
        && (0..=100).contains(&opportunity.opportunity_score)
        && bounded(&opportunity.service, MAX_FRONTIER_NAME_CHARS)
        && bounded(&opportunity.clue, MAX_EVENT_TEXT_CHARS)
}

fn claim_ok(
    claim: &LandClaim,
    state: &RepositoryState,
    config: &ServerConfig,
    account_ids: &HashSet<&str>,
) -> bool {
    bounded(&claim.claim_id, MAX_EVENT_ID_CHARS)
        && account_reference_ok(&claim.owner_account_id, account_ids)
        && bounded(&claim.owner_name, MAX_DISPLAY_NAME_CHARS)
        && position_in_world(claim.position, config)
        && claim.lease_days > 0
        && claim.last_active_tick <= state.tick
        && claim.reclaim_after_ticks > 0
}

fn expedition_ok(
    expedition: &Expedition,
    config: &ServerConfig,
    account_ids: &HashSet<&str>,
) -> bool {
    bounded(&expedition.expedition_id, MAX_EVENT_ID_CHARS)
        && bounded(&expedition.outpost_name, MAX_FRONTIER_NAME_CHARS)
        && account_reference_ok(&expedition.leader_account_id, account_ids)
        && position_in_world(expedition.outpost_position, config)
        && expedition.food <= MAX_EXPEDITION_SUPPLY
        && expedition.tools <= MAX_EXPEDITION_SUPPLY
        && expedition.materials <= MAX_EXPEDITION_SUPPLY
        && expedition.safety <= MAX_EXPEDITION_SUPPLY
        && expedition.members.len() <= super::super::phase3::MAX_EXPEDITION_MEMBERS
        && expedition.members.iter().all(|member| {
            account_reference_ok(&member.account_id, account_ids)
                && bounded(&member.display_name, MAX_FRONTIER_NAME_CHARS)
        })
        && (expedition
            .members
            .iter()
            .any(|member| member.account_id == expedition.leader_account_id)
            || expedition.leader_account_id == DELETED_ACCOUNT)
        && expedition
            .outcome
            .as_deref()
            .is_none_or(|outcome| bounded(outcome, MAX_EVENT_TEXT_CHARS))
}

fn account_reference_ok(account_id: &str, account_ids: &HashSet<&str>) -> bool {
    bounded(account_id, MAX_ACCOUNT_ID_CHARS)
        && (account_ids.contains(account_id) || account_id == DELETED_ACCOUNT)
}

fn bounded(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}

fn position_in_world(position: tarrowyn_protocol::Position, config: &ServerConfig) -> bool {
    position.x >= 0
        && position.y >= 0
        && (position.x as u32) < config.world_width
        && (position.y as u32) < config.world_height
}
