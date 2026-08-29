use super::models::RepositoryState;
use crate::config::ServerConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tarrowyn_protocol::{
    Capability, ClaimLifecycleResponse, ClaimRecord, FarmAnimal, FarmAnimalKind,
    GovernanceResponse, GovernanceState, HouseholdRecord, InfrastructureRecord, KnowledgeItem,
    KnowledgeResponse, LocalCombatResponse, LocalCombatState, MaterialStock, OfficeKind,
    OfficeRecord, ProfessionProfile, ProfessionResponse, ServiceOrder, ServiceOrderStatus,
    SkillLesson, SkillResponse, TaxPolicy,
};

mod claims;
mod combat;
mod governance;
mod households;
mod knowledge;

pub(super) use super::{validate_optional_identifier, validate_request_id};
mod professions;
pub(super) use claims::trim_claim_history;

const DEFAULT_TREASURY: u32 = 48;
pub(super) const MAX_PROPOSALS: usize = 64;
pub(super) const MAX_SERVICE_ORDERS: usize = 64;
pub(super) const MAX_GOVERNANCE_DECISIONS: usize = 64;
pub(super) const MAX_TAX_COLLECTIONS: usize = 64;
pub(super) const MAX_INFRASTRUCTURE_RECORDS: usize = 32;
pub(super) const MAX_SCHOOL_LESSONS: usize = 128;
pub(super) const MAX_CLAIMS: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) enum Phase4Response {
    Governance(GovernanceResponse),
    Claim(ClaimLifecycleResponse),
    Profession(ProfessionResponse),
    Knowledge(KnowledgeResponse),
    Combat(LocalCombatResponse),
    Skill(SkillResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Phase4State {
    #[serde(default = "default_next_lesson_id")]
    pub(super) next_lesson_id: u64,
    #[serde(default = "default_next_tax_id")]
    pub(super) next_tax_id: u64,
    pub(super) next_proposal_id: u64,
    pub(super) next_decision_id: u64,
    pub(super) next_order_id: u64,
    pub(super) next_claim_id: u64,
    pub(super) next_knowledge_id: u64,
    pub(super) governance: GovernanceState,
    pub(super) infrastructure: Vec<InfrastructureRecord>,
    pub(super) claims: Vec<ClaimRecord>,
    pub(super) available_plots: Vec<tarrowyn_protocol::Position>,
    pub(super) households: Vec<HouseholdRecord>,
    pub(super) profiles: HashMap<String, Vec<ProfessionProfile>>,
    pub(super) materials: HashMap<String, MaterialStock>,
    pub(super) credentials: HashMap<String, Vec<String>>,
    pub(super) orders: Vec<ServiceOrder>,
    pub(super) knowledge: Vec<KnowledgeItem>,
    pub(super) known_by: HashMap<String, Vec<String>>,
    pub(super) combat: HashMap<String, LocalCombatState>,
    #[serde(default)]
    pub(super) animals: Vec<FarmAnimal>,
    #[serde(default)]
    pub(super) lessons: Vec<SkillLesson>,
    pub(super) request_results: HashMap<String, Phase4Response>,
}

impl Default for Phase4State {
    fn default() -> Self {
        fresh(&ServerConfig::default())
    }
}

pub(super) fn fresh(_config: &ServerConfig) -> Phase4State {
    Phase4State {
        next_lesson_id: 1,
        next_tax_id: 1,
        next_proposal_id: 1,
        next_decision_id: 1,
        next_order_id: 1,
        next_claim_id: 1,
        next_knowledge_id: 1,
        governance: GovernanceState {
            settlement_id: "hearth-settlement".to_owned(),
            offices: vec![
                office(
                    "steward",
                    OfficeKind::Steward,
                    "Settlement Steward",
                    "May approve and complete all bounded public actions.",
                ),
                office(
                    "works-warden",
                    OfficeKind::WorksWarden,
                    "Works Warden",
                    "May propose and complete road, bridge, and public-work repairs.",
                ),
                office(
                    "registrar",
                    OfficeKind::Registrar,
                    "Settlement Registrar",
                    "May maintain the contract board and public records.",
                ),
            ],
            proposals: Vec::new(),
            decisions: Vec::new(),
            public_treasury: DEFAULT_TREASURY,
            administration_quality: 80,
            service_funding_until_tick: 0,
            taxation: Some(default_tax_policy()),
            tax_ledger: Vec::new(),
            cursor: 0,
        },
        infrastructure: crate::content::infrastructure_profiles()
            .into_iter()
            .map(infrastructure_from_profile)
            .collect(),
        claims: Vec::new(),
        available_plots: crate::content::farm_plot_positions(),
        households: vec![crate::content::npc_household("bellweather")],
        profiles: HashMap::new(),
        materials: HashMap::new(),
        credentials: HashMap::new(),
        orders: Vec::new(),
        knowledge: vec![KnowledgeItem {
            knowledge_id: "moonberry-tending".to_owned(),
            title: "Moonberry trellis method".to_owned(),
            kind: tarrowyn_protocol::KnowledgeKind::CropTechnique,
            description: "A low trellis keeps moonberries dry when the road is wet.".to_owned(),
            effect: "Improves moonberry quality by one when applied to a harvest.".to_owned(),
            teachable: true,
            writable: true,
            discovered_by: Vec::new(),
            stored_in: "A discoverer's private field notes".to_owned(),
        }],
        known_by: HashMap::new(),
        combat: HashMap::new(),
        animals: fresh_animals(),
        lessons: Vec::new(),
        request_results: HashMap::new(),
    }
}

pub(super) fn trim_proposals(governance: &mut GovernanceState) {
    while governance.proposals.len() > MAX_PROPOSALS {
        let Some(index) = governance.proposals.iter().position(|proposal| {
            matches!(
                proposal.status,
                tarrowyn_protocol::ProposalStatus::Completed
                    | tarrowyn_protocol::ProposalStatus::Rejected
            )
        }) else {
            break;
        };
        governance.proposals.remove(index);
    }
}

pub(super) fn proposal_room(governance: &mut GovernanceState) -> bool {
    trim_proposals(governance);
    if governance.proposals.len() < MAX_PROPOSALS {
        return true;
    }
    let Some(index) = governance.proposals.iter().position(|proposal| {
        matches!(
            proposal.status,
            tarrowyn_protocol::ProposalStatus::Completed
                | tarrowyn_protocol::ProposalStatus::Rejected
        )
    }) else {
        return false;
    };
    governance.proposals.remove(index);
    true
}

pub(super) fn trim_service_orders(phase4: &mut Phase4State) {
    while phase4.orders.len() > MAX_SERVICE_ORDERS {
        let Some(index) = phase4.orders.iter().position(|order| {
            matches!(
                order.status,
                ServiceOrderStatus::Completed | ServiceOrderStatus::Cancelled
            )
        }) else {
            break;
        };
        phase4.orders.remove(index);
    }
}

pub(super) fn service_order_room(phase4: &mut Phase4State) -> bool {
    trim_service_orders(phase4);
    if phase4.orders.len() < MAX_SERVICE_ORDERS {
        return true;
    }
    let Some(index) = phase4.orders.iter().position(|order| {
        matches!(
            order.status,
            ServiceOrderStatus::Completed | ServiceOrderStatus::Cancelled
        )
    }) else {
        return false;
    };
    phase4.orders.remove(index);
    true
}

pub(super) fn retain_recent<T>(records: &mut Vec<T>, max: usize) {
    if records.len() > max {
        let excess = records.len() - max;
        records.drain(..excess);
    }
}

pub(super) const FARM_ANIMAL_POSITION: tarrowyn_protocol::Position =
    tarrowyn_protocol::Position { x: 3, y: 5 };

pub(super) fn fresh_animals() -> Vec<FarmAnimal> {
    vec![FarmAnimal {
        animal_id: "bellweather-goat".to_owned(),
        name: "Bellweather".to_owned(),
        kind: FarmAnimalKind::Goat,
        position: FARM_ANIMAL_POSITION,
        condition: 2,
        max_condition: 3,
        last_cared_tick: 0,
        last_cared_day: 1,
    }]
}

fn default_next_lesson_id() -> u64 {
    1
}

fn default_next_tax_id() -> u64 {
    1
}

pub(super) fn default_tax_policy() -> TaxPolicy {
    TaxPolicy {
        payer: "Players within four tiles of the Hearth".to_owned(),
        recipient: "Hearth public treasury".to_owned(),
        rate_percent: 5,
        exemptions: vec![
            "Players outside Hearth territory".to_owned(),
            "Knocked-out players".to_owned(),
        ],
        accounting_note: "A small daily charge on nearby carried gold; no items are taken."
            .to_owned(),
        recovery_path:
            "A mayor may set a rate from 0% through 10%; the public ledger keeps receipts."
                .to_owned(),
    }
}

pub(super) fn unix_time_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub(super) fn lease_duration_days(config: &ServerConfig) -> u32 {
    (config.lease_duration_seconds / (24 * 60 * 60)).max(1) as u32
}

fn office(id: &str, kind: OfficeKind, title: &str, authority: &str) -> OfficeRecord {
    OfficeRecord {
        office_id: id.to_owned(),
        kind,
        title: title.to_owned(),
        authority: authority.to_owned(),
        holder_account_id: None,
        holder_name: None,
        last_active_tick: 0,
        vacant: true,
        vacancy_reason: Some("The first office-holder has not yet been chosen.".to_owned()),
    }
}

fn infrastructure_from_profile(
    profile: crate::content::InfrastructureProfile,
) -> InfrastructureRecord {
    InfrastructureRecord {
        infrastructure_id: profile.id,
        name: profile.name,
        kind: profile.kind,
        position: profile.position,
        condition: profile.condition,
        upkeep_per_day: profile.upkeep_per_day,
        service_quality: profile.service_quality,
        status: infrastructure_status(profile.condition),
        last_maintained_tick: 0,
        failure_note: Some(profile.note),
    }
}

#[allow(clippy::too_many_arguments)]
fn infrastructure(
    id: &str,
    name: &str,
    kind: tarrowyn_protocol::InfrastructureKind,
    position: tarrowyn_protocol::Position,
    condition: u8,
    upkeep_per_day: u32,
    service_quality: u8,
    note: &str,
) -> InfrastructureRecord {
    InfrastructureRecord {
        infrastructure_id: id.to_owned(),
        name: name.to_owned(),
        kind,
        position,
        condition,
        upkeep_per_day,
        service_quality,
        status: infrastructure_status(condition),
        last_maintained_tick: 0,
        failure_note: Some(note.to_owned()),
    }
}

pub(super) fn infrastructure_status(condition: u8) -> tarrowyn_protocol::InfrastructureStatus {
    use tarrowyn_protocol::InfrastructureStatus;
    match condition {
        0..=24 => InfrastructureStatus::Failed,
        25..=59 => InfrastructureStatus::NeedsRepair,
        60..=79 => InfrastructureStatus::Recovering,
        _ => InfrastructureStatus::Operational,
    }
}

pub(super) fn cache_key(account: &str, request_id: &str) -> String {
    format!("phase4:{account}:{request_id}")
}

pub(super) fn account_id(state: &RepositoryState, key: &str) -> String {
    state
        .identities
        .get(key)
        .expect("identity exists")
        .account_id
        .clone()
}

pub(super) fn account_name(state: &RepositoryState, key: &str) -> String {
    state
        .identities
        .get(key)
        .expect("identity exists")
        .display_name
        .clone()
}

pub(super) fn key_for_account(state: &RepositoryState, account: &str) -> Option<String> {
    state
        .identities
        .iter()
        .find(|(_, identity)| identity.account_id == account)
        .map(|(key, _)| key.clone())
}

pub(super) fn record(state: &mut RepositoryState, kind: &str, title: &str, text: &str) {
    super::phase3::record(state, kind, title, text);
    state.phase4.governance.cursor = state.cursor;
}

pub(super) fn phase4_tick(state: &mut RepositoryState, config: &ServerConfig) -> Option<bool> {
    prune_school_lessons(state);
    governance::tick(state, config);
    claims::tick(state, config);
    households::tick(state, config);
    super::phase5::phase5_tick(state, config);
    super::phase6::phase6_tick(state, config)
}

pub(super) fn day_rollover(state: &mut RepositoryState) {
    for animal in &mut state.phase4.animals {
        if animal.last_cared_day < state.clock.day {
            animal.condition = animal.condition.saturating_sub(1);
        }
    }
}

pub(super) fn prune_school_lessons(state: &mut RepositoryState) {
    trim_school_lessons(&mut state.phase4, state.tick);
}

pub(super) fn school_lesson_room(state: &mut RepositoryState) -> bool {
    prune_school_lessons(state);
    state.phase4.lessons.len() < MAX_SCHOOL_LESSONS
}

pub(super) fn trim_school_lessons(phase4: &mut Phase4State, tick: u64) {
    phase4.lessons.retain(|lesson| lesson.expires_tick > tick);
    phase4.lessons.sort_by_key(|lesson| lesson.started_tick);
    let excess = phase4.lessons.len().saturating_sub(MAX_SCHOOL_LESSONS);
    if excess > 0 {
        phase4.lessons.drain(..excess);
    }
}

pub(super) fn default_capability(profession: tarrowyn_protocol::ProfessionKind) -> Capability {
    let (id, name, description, effect) = match profession {
        tarrowyn_protocol::ProfessionKind::Farmer => (
            "field-tending",
            "Field tending",
            "Keeps a crop reliable without wasting seed.",
            "Adds one quality to a tended crop.",
        ),
        tarrowyn_protocol::ProfessionKind::Smith => (
            "iron-fitting",
            "Iron fitting",
            "Turns a small iron stock into a dependable repair.",
            "Improves repair quality by one.",
        ),
        tarrowyn_protocol::ProfessionKind::Carpenter => (
            "joinery",
            "Joinery",
            "Makes wood and tools serve a public repair order.",
            "Improves public-work reliability by one.",
        ),
        tarrowyn_protocol::ProfessionKind::Healer => (
            "field-dressing",
            "Field dressing",
            "Limits an injury before it becomes a lasting burden.",
            "Reduces recovery cost by one.",
        ),
        tarrowyn_protocol::ProfessionKind::Scout => (
            "road-reading",
            "Road reading",
            "Finds a safe route and records the clue for others.",
            "Improves route knowledge.",
        ),
        tarrowyn_protocol::ProfessionKind::Steward => (
            "public-accounting",
            "Public accounting",
            "Keeps a proposal's cost and service legible.",
            "Adds one administration quality to a completed action.",
        ),
    };
    Capability {
        capability_id: id.to_owned(),
        name: name.to_owned(),
        profession,
        level: 1,
        description: description.to_owned(),
        effect: effect.to_owned(),
    }
}

pub(super) fn restore_service_order_escrow(state: &mut RepositoryState, order: &ServiceOrder) {
    let Some(requester_key) = key_for_account(state, &order.requester_account_id) else {
        return;
    };
    let Some(stock) = state.phase4.materials.get_mut(&requester_key) else {
        return;
    };
    stock.wood = stock.wood.saturating_add(order.materials.wood);
    stock.iron = stock.iron.saturating_add(order.materials.iron);
    stock.cloth = stock.cloth.saturating_add(order.materials.cloth);
    stock.bandages = stock.bandages.saturating_add(order.materials.bandages);
    stock.tools = stock
        .tools
        .saturating_add(order.materials.tools)
        .saturating_add(order.tools_required);
}

#[cfg(test)]
mod tests;
