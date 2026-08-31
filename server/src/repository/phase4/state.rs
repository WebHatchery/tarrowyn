//! Durable Phase 4 state and response records.

use super::{
    default_tax_policy, fresh_animals, infrastructure_from_profile, office, DEFAULT_TREASURY,
};
use crate::config::ServerConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tarrowyn_protocol::{
    ClaimLifecycleResponse, ClaimRecord, FarmAnimal, GovernanceResponse, GovernanceState,
    HouseholdRecord, InfrastructureRecord, KnowledgeItem, KnowledgeResponse, LocalCombatResponse,
    LocalCombatState, MaterialStock, OfficeKind, ProfessionProfile, ProfessionResponse,
    ServiceOrder, SkillLesson, SkillResponse,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Phase4Response {
    Governance(GovernanceResponse),
    Claim(ClaimLifecycleResponse),
    Profession(ProfessionResponse),
    Knowledge(KnowledgeResponse),
    Combat(LocalCombatResponse),
    Skill(SkillResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Phase4State {
    #[serde(default = "default_next_lesson_id")]
    pub(crate) next_lesson_id: u64,
    #[serde(default = "default_next_tax_id")]
    pub(crate) next_tax_id: u64,
    pub(crate) next_proposal_id: u64,
    pub(crate) next_decision_id: u64,
    pub(crate) next_order_id: u64,
    pub(crate) next_claim_id: u64,
    pub(crate) next_knowledge_id: u64,
    pub(crate) governance: GovernanceState,
    pub(crate) infrastructure: Vec<InfrastructureRecord>,
    pub(crate) claims: Vec<ClaimRecord>,
    pub(crate) available_plots: Vec<tarrowyn_protocol::Position>,
    pub(crate) households: Vec<HouseholdRecord>,
    pub(crate) profiles: HashMap<String, Vec<ProfessionProfile>>,
    pub(crate) materials: HashMap<String, MaterialStock>,
    pub(crate) credentials: HashMap<String, Vec<String>>,
    pub(crate) orders: Vec<ServiceOrder>,
    pub(crate) knowledge: Vec<KnowledgeItem>,
    pub(crate) known_by: HashMap<String, Vec<String>>,
    pub(crate) combat: HashMap<String, LocalCombatState>,
    #[serde(default)]
    pub(crate) animals: Vec<FarmAnimal>,
    #[serde(default)]
    pub(crate) lessons: Vec<SkillLesson>,
    pub(crate) request_results: HashMap<String, Phase4Response>,
}

impl Default for Phase4State {
    fn default() -> Self {
        fresh(&ServerConfig::default())
    }
}

pub(crate) fn fresh(_config: &ServerConfig) -> Phase4State {
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

fn default_next_lesson_id() -> u64 {
    1
}

fn default_next_tax_id() -> u64 {
    1
}
