//! Durable Phase 3 state and compatibility normalization.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tarrowyn_protocol::{
    ChronicleEntry, ClaimResponse, CombatResponse, ContractResponse, ContractStatus, Expedition,
    ExpeditionMember, ExpeditionResponse, LandClaim, OpportunitySignal, Position, RecoveryResponse,
    WildernessZone,
};

pub(crate) const MAX_CHRONICLE: usize = 64;
pub(crate) const MAX_EXPEDITION_MEMBERS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ContractProgress {
    pub(crate) progress: u8,
    pub(crate) status: ContractStatus,
    pub(crate) completion_count: u32,
    pub(crate) available_at_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Phase3Response {
    Contract(ContractResponse),
    Combat(CombatResponse),
    Recovery(RecoveryResponse),
    Claim(ClaimResponse),
    Expedition(ExpeditionResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Phase3State {
    pub(crate) next_event_id: u64,
    pub(crate) zone: WildernessZone,
    pub(crate) contracts: HashMap<String, ContractProgress>,
    pub(crate) households: Vec<OpportunitySignal>,
    pub(crate) unmet_demand_ticks: u64,
    pub(crate) poor_condition_ticks: u64,
    pub(crate) chronicle: VecDeque<ChronicleEntry>,
    #[serde(default)]
    pub(crate) chronicle_archive: Vec<ChronicleEntry>,
    pub(crate) claim: Option<LandClaim>,
    pub(crate) expedition: Option<Expedition>,
    #[serde(default)]
    pub(crate) expedition_credentials: Vec<String>,
    pub(crate) outpost: Option<Position>,
    pub(crate) request_results: HashMap<String, Phase3Response>,
}

impl Default for Phase3State {
    fn default() -> Self {
        let threat = crate::content::threat_template("whisperwood-edge");
        let household = crate::content::opportunity_template("household-maren");
        Self {
            next_event_id: 1,
            zone: WildernessZone {
                zone_id: threat.id,
                name: threat.name,
                monster: threat.monster,
                monster_health: threat.monster_health,
                threat_active: true,
                road_open: false,
                position: threat.position,
                price_modifier_percent: threat.price_modifier_percent,
                resource_demand: threat.resource_demand,
                rumour: threat.rumour,
            },
            contracts: HashMap::new(),
            households: vec![OpportunitySignal {
                household_id: household.household_id,
                household_name: household.household_name,
                members: household.members,
                occupation: household.occupation,
                home_settlement: household.home_settlement,
                opportunity_score: household.opportunity_score,
                status: household.status,
                service: household.service,
                clue: household.clue,
            }],
            unmet_demand_ticks: 0,
            poor_condition_ticks: 0,
            chronicle: VecDeque::new(),
            chronicle_archive: Vec::new(),
            claim: None,
            expedition: None,
            expedition_credentials: Vec::new(),
            outpost: None,
            request_results: HashMap::new(),
        }
    }
}

pub(crate) fn fresh() -> Phase3State {
    Phase3State::default()
}

pub(crate) fn trim_expedition_members(phase: &mut Phase3State) {
    let Some(expedition) = phase.expedition.as_mut() else {
        return;
    };
    expedition.members.truncate(MAX_EXPEDITION_MEMBERS);
    if !expedition
        .members
        .iter()
        .any(|member: &ExpeditionMember| member.account_id == expedition.leader_account_id)
    {
        if let Some(member) = expedition.members.first() {
            expedition.leader_account_id = member.account_id.clone();
        }
    }
}

pub(crate) fn archive_excess(phase: &mut Phase3State) {
    while phase.chronicle.len() > MAX_CHRONICLE {
        if let Some(entry) = phase.chronicle.pop_front() {
            phase.chronicle_archive.push(entry);
        }
    }
}

pub(crate) fn normalize_opportunity_score(score: &mut i16) {
    *score = (*score).clamp(0, 100);
}
