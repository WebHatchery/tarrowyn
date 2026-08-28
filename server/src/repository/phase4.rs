use super::models::RepositoryState;
use crate::config::ServerConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tarrowyn_protocol::{
    Capability, ClaimLifecycleResponse, ClaimRecord, FarmAnimal, FarmAnimalKind,
    GovernanceResponse, GovernanceState, HouseholdRecord, InfrastructureRecord, KnowledgeItem,
    KnowledgeResponse, LocalCombatResponse, LocalCombatState, MaterialStock, OfficeKind,
    OfficeRecord, ProfessionProfile, ProfessionResponse, ServiceOrder, SkillLesson, SkillResponse,
    TaxPolicy,
};

mod claims;
mod combat;
mod governance;
mod households;
mod knowledge;
mod professions;

const DEFAULT_TREASURY: u32 = 48;

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
        infrastructure: vec![
            infrastructure(
                "north-road",
                "North road",
                tarrowyn_protocol::InfrastructureKind::Road,
                tarrowyn_protocol::Position { x: 11, y: 6 },
                72,
                2,
                58,
                "The Brambleback threat and unpaid upkeep make travel uncertain.",
            ),
            infrastructure(
                "stone-bridge",
                "Stone bridge",
                tarrowyn_protocol::InfrastructureKind::Bridge,
                tarrowyn_protocol::Position { x: 10, y: 6 },
                88,
                1,
                75,
                "The bridge is sound but still needs regular inspection.",
            ),
            infrastructure(
                "hearth-hall",
                "Town hall",
                tarrowyn_protocol::InfrastructureKind::PublicBuilding,
                tarrowyn_protocol::Position { x: 8, y: 5 },
                94,
                2,
                82,
                "The hall keeps its records even when an office is vacant.",
            ),
            infrastructure(
                "hearth-services",
                "Hearth services",
                tarrowyn_protocol::InfrastructureKind::Service,
                tarrowyn_protocol::Position { x: 8, y: 5 },
                86,
                3,
                70,
                "Menders and healers report whether shared funding reaches them.",
            ),
        ],
        claims: Vec::new(),
        available_plots: vec![
            tarrowyn_protocol::Position { x: 2, y: 8 },
            tarrowyn_protocol::Position { x: 2, y: 9 },
            tarrowyn_protocol::Position { x: 10, y: 8 },
        ],
        households: vec![HouseholdRecord {
            household_id: "household-bellweather".to_owned(),
            household_name: "The Bellweather household".to_owned(),
            members: vec![
                tarrowyn_protocol::HouseholdMemberRecord {
                    name: "Iven".to_owned(),
                    role: "miller".to_owned(),
                    service: "grain milling and field planning".to_owned(),
                },
                tarrowyn_protocol::HouseholdMemberRecord {
                    name: "Sella".to_owned(),
                    role: "herbal healer".to_owned(),
                    service: "bandages and recovery advice".to_owned(),
                },
            ],
            home: "The Hearth settlement".to_owned(),
            needs: vec!["steady grain demand".to_owned(), "safe roads".to_owned()],
            work: "The miller feeds the fields while the healer keeps workers safe.".to_owned(),
            service_quality: 72,
            demand: 60,
            housing: 70,
            safety: 62,
            food: 68,
            competition: 20,
            status: tarrowyn_protocol::HouseholdLifeStatus::Arrived,
            clue: "The miller and healer will keep both services open if demand and roads hold."
                .to_owned(),
            last_decision_tick: 0,
        }],
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
            stored_in: "The Hearth guild archive".to_owned(),
        }],
        known_by: HashMap::new(),
        combat: HashMap::new(),
        animals: fresh_animals(),
        lessons: Vec::new(),
        request_results: HashMap::new(),
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

pub(super) fn validate_request_id(request_id: &str) -> Result<(), super::RepositoryError> {
    if request_id.trim().is_empty() || request_id.len() > 64 {
        Err(super::RepositoryError::new(
            400,
            "invalid_request_id",
            "Phase 4 request IDs must contain 1 to 64 characters.",
        ))
    } else {
        Ok(())
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
    state
        .phase4
        .lessons
        .retain(|lesson| lesson.expires_tick > state.tick);
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

#[cfg(test)]
mod tests;
