use super::phase3::Phase3State;
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Clone)]
pub(super) struct Session {
    pub(super) client_key: String,
    pub(super) identity_key: String,
    pub(super) last_seen_tick: u64,
    pub(super) last_movement_tick: Option<u64>,
    pub(super) last_chat_tick: Option<u64>,
    pub(super) movement_results: HashMap<String, MovementResponse>,
    pub(super) chat_results: HashMap<String, ChatResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredState {
    pub(crate) storage_version: u32,
    pub(crate) tick: u64,
    pub(super) clock: WorldClock,
    pub(crate) cursor: u64,
    pub(super) next_guest: u64,
    pub(super) next_message: u64,
    pub(super) next_token: u64,
    pub(super) next_trade: u64,
    pub(super) next_notice: u64,
    pub(super) identities: HashMap<String, Identity>,
    pub(super) plots: Vec<FarmPlot>,
    pub(super) events: VecDeque<EventRecord>,
    pub(super) chat_history: VecDeque<ChatMessage>,
    pub(super) notices: VecDeque<TavernNotice>,
    pub(super) trades: HashMap<String, TradeOffer>,
    #[serde(default)]
    pub(super) phase3: Phase3State,
    #[serde(default)]
    pub(super) phase4: super::phase4::Phase4State,
    #[serde(default)]
    pub(super) phase5: super::phase5::Phase5State,
    #[serde(default)]
    pub(super) phase6: super::phase6::Phase6State,
}

#[derive(Debug)]
pub(crate) struct RepositoryState {
    pub(super) tick: u64,
    pub(super) clock: WorldClock,
    pub(super) cursor: u64,
    pub(super) next_guest: u64,
    pub(super) next_message: u64,
    pub(super) next_token: u64,
    pub(super) next_trade: u64,
    pub(super) next_notice: u64,
    pub(crate) identities: HashMap<String, Identity>,
    pub(super) sessions: HashMap<String, Session>,
    pub(super) plots: Vec<FarmPlot>,
    pub(super) events: VecDeque<EventRecord>,
    pub(super) chat_history: VecDeque<ChatMessage>,
    pub(super) notices: VecDeque<TavernNotice>,
    pub(super) trades: HashMap<String, TradeOffer>,
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
            identities: HashMap::new(),
            sessions: HashMap::new(),
            plots: farm_plots(),
            events: VecDeque::new(),
            chat_history: VecDeque::new(),
            notices: VecDeque::new(),
            trades: HashMap::new(),
            phase3: super::phase3::fresh(),
            phase4: super::phase4::fresh(config),
            phase5: super::phase5::fresh(config),
            phase6: super::phase6::fresh(config),
        }
    }

    pub(crate) fn from_stored(stored: StoredState, config: &ServerConfig) -> Self {
        let mut phase4 = stored.phase4;
        if phase4.governance.taxation.is_none() {
            phase4.governance.taxation = Some(super::phase4::default_tax_policy());
        }
        Self {
            tick: stored.tick,
            clock: WorldClock {
                day: stored.clock.day.max(1),
                seconds: stored.clock.seconds.max(0.0),
                day_length_seconds: config.day_length_seconds,
            },
            cursor: stored.cursor,
            next_guest: stored.next_guest.max(1),
            next_message: stored.next_message.max(1),
            next_token: stored.next_token.max(1),
            next_trade: stored.next_trade.max(1),
            next_notice: stored.next_notice.max(1),
            identities: stored.identities,
            sessions: HashMap::new(),
            plots: if stored.plots.is_empty() {
                farm_plots()
            } else {
                stored.plots
            },
            events: trim_queue(stored.events, MAX_EVENTS),
            chat_history: trim_queue(stored.chat_history, MAX_CHAT_HISTORY),
            notices: trim_queue(stored.notices, MAX_NOTICES),
            trades: stored.trades,
            phase3: stored.phase3,
            phase4,
            phase5: stored.phase5,
            phase6: stored.phase6,
        }
    }

    pub(crate) fn to_stored(&self) -> StoredState {
        StoredState {
            storage_version: STORAGE_VERSION,
            tick: self.tick,
            clock: self.clock.clone(),
            cursor: self.cursor,
            next_guest: self.next_guest,
            next_message: self.next_message,
            next_token: self.next_token,
            next_trade: self.next_trade,
            next_notice: self.next_notice,
            identities: self.identities.clone(),
            plots: self.plots.clone(),
            events: self.events.clone(),
            chat_history: self.chat_history.clone(),
            notices: self.notices.clone(),
            trades: self.trades.clone(),
            phase3: self.phase3.clone(),
            phase4: self.phase4.clone(),
            phase5: self.phase5.clone(),
            phase6: self.phase6.clone(),
        }
    }
}

fn default_weapon() -> WeaponKind {
    WeaponKind::IronSword
}
