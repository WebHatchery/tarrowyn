use super::phase3::Phase3State;
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use tarrowyn_protocol::{
    FarmPlot, FoundationActivityState, FoundationJourneyProgress, FoundationPropertyResponse,
    FoundationPropertyState, FoundationStorehouseResponse, FrontierEvent,
};

pub(super) const MAX_REPLAY_CACHE: usize = 512;

pub(super) fn trim_replay_cache<K, T>(cache: &mut HashMap<K, T>)
where
    K: Clone + Eq + Hash,
{
    while cache.len() > MAX_REPLAY_CACHE {
        let Some(key) = cache.keys().next().cloned() else {
            break;
        };
        cache.remove(&key);
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct SkillLedger {
    pub(super) practice: HashMap<String, u32>,
    pub(super) known: Vec<String>,
    pub(super) qualifying_events: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Identity {
    pub(crate) account_id: String,
    pub(crate) character_id: String,
    pub(super) display_name: String,
    pub(super) position: Position,
    pub(super) gold: u32,
    #[serde(default = "default_field_tool_condition")]
    pub(super) field_tool_condition: u8,
    #[serde(default)]
    pub(super) field_tool_kind: FoundationFieldToolKind,
    pub(super) skill: u32,
    pub(super) reputation: u32,
    pub(super) inventory: Inventory,
    pub(super) seeds_planted: u32,
    #[serde(default)]
    pub(super) last_seen_tick: u64,
    #[serde(default)]
    pub(super) last_tax_day: u32,
    #[serde(default)]
    pub(super) farming_results: HashMap<String, FarmingResponse>,
    #[serde(default)]
    pub(super) trade_results: HashMap<String, TradeResponse>,
    #[serde(default)]
    pub(super) movement_results: HashMap<String, MovementResponse>,
    #[serde(default)]
    pub(super) chat_results: HashMap<String, ChatResponse>,
    #[serde(default)]
    pub(super) foundation_resource_results: HashMap<String, FoundationResourceResponse>,
    #[serde(default)]
    pub(super) foundation_cache_results: HashMap<String, FoundationCacheResponse>,
    #[serde(default)]
    pub(super) foundation_forge_results: HashMap<String, FoundationForgeResponse>,
    #[serde(default)]
    pub(super) foundation_storehouse_results: HashMap<String, FoundationStorehouseResponse>,
    #[serde(default)]
    pub(super) foundation_property_results: HashMap<String, FoundationPropertyResponse>,
    #[serde(default)]
    pub(super) foundation_journey: FoundationJourneyProgress,
    #[serde(default = "default_weapon")]
    pub(super) weapon: WeaponKind,
    #[serde(default)]
    pub(super) knocked_out: bool,
    #[serde(default)]
    pub(super) injuries: u8,
    #[serde(default)]
    pub(super) recovery_cost: u32,
    #[serde(default)]
    pub(super) skills: SkillLedger,
}

fn default_field_tool_condition() -> u8 {
    super::FIELD_TOOL_MAX_CONDITION
}

#[derive(Debug, Clone)]
pub(super) struct Session {
    pub(super) client_key: String,
    pub(super) identity_key: String,
    pub(super) last_seen_tick: u64,
    pub(super) last_movement_tick: Option<u64>,
    pub(super) last_chat_tick: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredState {
    pub(crate) storage_version: u32,
    #[serde(default)]
    pub(crate) persisted_at_unix_millis: u64,
    pub(crate) tick: u64,
    pub(super) clock: WorldClock,
    pub(crate) cursor: u64,
    pub(super) next_guest: u64,
    pub(super) next_message: u64,
    pub(super) next_token: u64,
    pub(super) next_trade: u64,
    pub(super) next_notice: u64,
    #[serde(default = "default_next_property")]
    pub(super) next_property: u64,
    pub(super) identities: HashMap<String, Identity>,
    pub(super) plots: Vec<FarmPlot>,
    pub(super) events: VecDeque<EventRecord>,
    pub(super) chat_history: VecDeque<ChatMessage>,
    pub(super) notices: VecDeque<TavernNotice>,
    pub(super) trades: HashMap<String, TradeOffer>,
    #[serde(default = "super::foundation::fresh")]
    pub(super) foundation_activity: FoundationActivityState,
    #[serde(default)]
    pub(super) foundation_properties: Vec<FoundationPropertyState>,
    #[serde(default)]
    pub(super) phase3: Phase3State,
    #[serde(default)]
    pub(super) phase4: super::phase4::Phase4State,
    #[serde(default)]
    pub(super) phase5: super::phase5::Phase5State,
    #[serde(default)]
    pub(super) phase6: super::phase6::Phase6State,
}

#[derive(Debug, Clone)]
pub(crate) struct RepositoryState {
    pub(super) tick: u64,
    pub(super) clock: WorldClock,
    pub(super) cursor: u64,
    pub(super) next_guest: u64,
    pub(super) next_message: u64,
    pub(super) next_token: u64,
    pub(super) next_trade: u64,
    pub(super) next_notice: u64,
    pub(super) next_property: u64,
    pub(crate) identities: HashMap<String, Identity>,
    pub(super) sessions: HashMap<String, Session>,
    pub(super) plots: Vec<FarmPlot>,
    pub(super) events: VecDeque<EventRecord>,
    pub(super) chat_history: VecDeque<ChatMessage>,
    pub(super) notices: VecDeque<TavernNotice>,
    pub(super) trades: HashMap<String, TradeOffer>,
    pub(super) foundation_activity: FoundationActivityState,
    pub(super) foundation_properties: Vec<FoundationPropertyState>,
    pub(super) phase3: Phase3State,
    pub(super) phase4: super::phase4::Phase4State,
    pub(super) phase5: super::phase5::Phase5State,
    pub(super) phase6: super::phase6::Phase6State,
}

impl RepositoryState {
    pub(super) fn fresh(config: &ServerConfig) -> Self {
        Self {
            tick: 0,
            clock: WorldClock {
                day: 1,
                seconds: 0.0,
                day_length_seconds: config.day_length_seconds,
            },
            cursor: 0,
            next_guest: 1,
            next_message: 1,
            next_token: 1,
            next_trade: 1,
            next_notice: 1,
            next_property: 1,
            identities: HashMap::new(),
            sessions: HashMap::new(),
            plots: super::world::farm_plots(),
            events: VecDeque::new(),
            chat_history: VecDeque::new(),
            notices: VecDeque::new(),
            trades: HashMap::new(),
            foundation_activity: super::foundation::fresh(),
            foundation_properties: Vec::new(),
            phase3: super::phase3::fresh(),
            phase4: super::phase4::fresh(config),
            phase5: super::phase5::fresh(config),
            phase6: super::phase6::fresh(config),
        }
    }

    pub(crate) fn from_stored(stored: StoredState, config: &ServerConfig) -> Self {
        Self::from_stored_at(stored, config, unix_time_millis())
    }

    pub(crate) fn from_stored_at(
        stored: StoredState,
        config: &ServerConfig,
        now_unix_millis: u64,
    ) -> Self {
        let persisted_at_unix_millis = stored.persisted_at_unix_millis;
        let migrate_legacy_crop_growth = stored.storage_version < 22;
        let day_length_seconds = config.day_length_seconds.max(1.0);
        let clock_seconds = if stored.clock.seconds.is_finite() {
            stored.clock.seconds.max(0.0) % day_length_seconds
        } else {
            0.0
        };
        let mut identities = stored.identities;
        for identity in identities.values_mut() {
            trim_replay_cache(&mut identity.farming_results);
            trim_replay_cache(&mut identity.trade_results);
            trim_replay_cache(&mut identity.movement_results);
            trim_replay_cache(&mut identity.chat_results);
            trim_replay_cache(&mut identity.foundation_forge_results);
            trim_replay_cache(&mut identity.foundation_resource_results);
            trim_replay_cache(&mut identity.foundation_cache_results);
            trim_replay_cache(&mut identity.foundation_storehouse_results);
            trim_replay_cache(&mut identity.foundation_property_results);
            super::foundation::journey::restore_progress(&mut identity.foundation_journey);
        }
        let mut foundation_activity = stored.foundation_activity;
        super::foundation::restore(&mut foundation_activity);
        let mut phase3 = stored.phase3;
        super::phase3::archive_excess(&mut phase3);
        super::phase3::trim_expedition_members(&mut phase3);
        super::phase3_frontier::backfill_expedition_credentials(&mut phase3);
        for household in &mut phase3.households {
            super::phase3::normalize_opportunity_score(&mut household.opportunity_score);
        }
        trim_replay_cache(&mut phase3.request_results);
        let mut phase4 = stored.phase4;
        super::phase4::trim_proposals(&mut phase4.governance);
        super::phase4::trim_service_orders(&mut phase4);
        super::phase4::trim_claim_history(&mut phase4.claims);
        super::phase4::trim_school_lessons(&mut phase4, stored.tick);
        super::phase4::retain_recent(
            &mut phase4.governance.decisions,
            super::phase4::MAX_GOVERNANCE_DECISIONS,
        );
        super::phase4::retain_recent(
            &mut phase4.governance.tax_ledger,
            super::phase4::MAX_TAX_COLLECTIONS,
        );
        super::phase4::retain_recent(
            &mut phase4.infrastructure,
            super::phase4::MAX_INFRASTRUCTURE_RECORDS,
        );
        trim_replay_cache(&mut phase4.request_results);
        let mut phase5 = stored.phase5;
        if phase5.fallback_day == 0 {
            // Snapshots written before fallback-day tracking omitted this field.
            // Treat the restored clock day as the start of the current fallback
            // window so a legacy world can pass readiness without losing its
            // accumulated regional state.
            phase5.fallback_day = stored.clock.day.max(1);
        }
        phase5
            .route_action_available_at_tick
            .retain(|_, available_at_tick| *available_at_tick > stored.tick);
        super::phase5::trim_market_orders(&mut phase5);
        super::phase5::trim_event_history(&mut phase5);
        super::phase5::trim_settlement_chronicles(&mut phase5);
        super::phase5::trim_household_histories(&mut phase5);
        trim_replay_cache(&mut phase5.request_results);
        let mut phase6 = stored.phase6;
        super::phase6::trim_moderation_reports(&mut phase6, super::phase4::unix_time_seconds());
        trim_replay_cache(&mut phase6.auth_link_results);
        super::phase6::trim_auth_link_tokens(&mut phase6);
        trim_replay_cache(&mut phase6.auth_refresh_results);
        super::phase6::backfill_auth_refresh_accounts(&mut phase6, stored.tick);
        trim_replay_cache(&mut phase6.auth_revoke_results);
        trim_replay_cache(&mut phase6.auth_revoke_guest_tokens);
        trim_replay_cache(&mut phase6.moderation_results);
        phase6
            .moderation_last_report_ticks
            .retain(|identity_key, _| identities.contains_key(identity_key));
        trim_replay_cache(&mut phase6.request_results);
        trim_replay_cache(&mut phase6.deletion_results);
        super::phase6::trim_audits(&mut phase6.audits);
        phase4.animals = super::phase4::restore_animals(phase4.animals);
        if phase4.governance.taxation.is_none() {
            phase4.governance.taxation = Some(super::phase4::default_tax_policy());
        }
        let mut trades = stored.trades;
        super::trades::trim_trade_history(&mut trades);
        let mut events = trim_queue(stored.events, MAX_EVENTS);
        for record in &mut events {
            if let WorldEvent::Frontier(FrontierEvent::Opportunity(opportunity)) = &mut record.event
            {
                super::phase3::normalize_opportunity_score(&mut opportunity.opportunity_score);
            }
        }
        let lease_days = super::phase4::lease_duration_days(config);
        let now = super::phase4::unix_time_seconds();
        for claim in &mut phase4.claims {
            claim.lease_days = lease_days;
            if claim.expires_at_unix_seconds == 0
                && matches!(
                    claim.status,
                    tarrowyn_protocol::ClaimLifecycleStatus::Active
                        | tarrowyn_protocol::ClaimLifecycleStatus::Renewed
                        | tarrowyn_protocol::ClaimLifecycleStatus::Transferred
                        | tarrowyn_protocol::ClaimLifecycleStatus::Inherited
                )
            {
                claim.started_at_unix_seconds = now;
                claim.expires_at_unix_seconds =
                    now.saturating_add(config.lease_duration_seconds.max(1));
            }
        }
        let sessions = phase6
            .sessions
            .iter()
            .filter(|(_, session)| !session.revoked && session.expires_at_tick > stored.tick)
            .map(|(token, session)| {
                (
                    token.clone(),
                    Session {
                        client_key: session.identity_key.clone(),
                        identity_key: session.identity_key.clone(),
                        last_seen_tick: stored.tick,
                        last_movement_tick: None,
                        last_chat_tick: None,
                    },
                )
            })
            .collect();
        let mut state = Self {
            tick: stored.tick,
            clock: WorldClock {
                day: stored.clock.day.max(1),
                seconds: clock_seconds,
                day_length_seconds: config.day_length_seconds,
            },
            cursor: stored.cursor,
            next_guest: stored.next_guest.max(1),
            next_message: stored.next_message.max(1),
            next_token: stored.next_token.max(1),
            next_trade: stored.next_trade.max(1),
            next_notice: stored.next_notice.max(1),
            next_property: stored.next_property.max(1),
            identities,
            sessions,
            plots: super::world::restore_plots(
                stored.plots,
                stored.tick,
                migrate_legacy_crop_growth,
            ),
            events,
            chat_history: trim_queue(stored.chat_history, MAX_CHAT_HISTORY),
            notices: trim_queue(stored.notices, MAX_NOTICES),
            trades,
            foundation_activity,
            foundation_properties: super::foundation::property::restore_properties(
                stored.foundation_properties,
            ),
            phase3,
            phase4,
            phase5,
            phase6,
        };
        super::world::apply_offline_crop_growth(
            &mut state,
            config,
            persisted_at_unix_millis,
            now_unix_millis,
        );
        super::reset::anonymize_orphaned_public_history(&mut state);
        state
    }

    pub(crate) fn to_stored(&self) -> StoredState {
        let mut stored = StoredState {
            storage_version: STORAGE_VERSION,
            persisted_at_unix_millis: unix_time_millis(),
            tick: self.tick,
            clock: self.clock.clone(),
            cursor: self.cursor,
            next_guest: self.next_guest,
            next_message: self.next_message,
            next_token: self.next_token,
            next_trade: self.next_trade,
            next_notice: self.next_notice,
            next_property: self.next_property,
            identities: self.identities.clone(),
            plots: self.plots.clone(),
            events: self.events.clone(),
            chat_history: self.chat_history.clone(),
            notices: self.notices.clone(),
            trades: self.trades.clone(),
            foundation_activity: self.foundation_activity.clone(),
            foundation_properties: self.foundation_properties.clone(),
            phase3: self.phase3.clone(),
            phase4: self.phase4.clone(),
            phase5: self.phase5.clone(),
            phase6: self.phase6.clone(),
        };
        super::phase5::refresh_stored_settlement_facilities(&mut stored);
        stored
    }
}

pub(super) fn unix_time_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn default_next_property() -> u64 {
    1
}

fn default_weapon() -> WeaponKind {
    WeaponKind::IronSword
}
