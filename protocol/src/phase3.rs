use super::{PlayerProjection, Position};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MonsterKind {
    Brambleback,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WeaponKind {
    IronSword,
    Spear,
    Axe,
    Bow,
    Shield,
    ImprovisedClub,
}

impl WeaponKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::IronSword => "iron sword",
            Self::Spear => "spear",
            Self::Axe => "axe",
            Self::Bow => "bow",
            Self::Shield => "shield",
            Self::ImprovisedClub => "improvised club",
        }
    }

    pub fn damage(self) -> u8 {
        match self {
            Self::IronSword => 3,
            Self::Spear | Self::Axe | Self::Bow => 2,
            Self::Shield | Self::ImprovisedClub => 1,
        }
    }

    pub fn skill_id(self) -> &'static str {
        match self {
            Self::IronSword => "sword-fighting",
            Self::Spear => "spear-fighting",
            Self::Axe => "axe-fighting",
            Self::Bow => "bow-fighting",
            Self::Shield => "shield-use",
            Self::ImprovisedClub => "unarmed-fighting",
        }
    }

    pub fn weapon_fighting_family(self) -> Option<&'static str> {
        match self {
            Self::IronSword => Some("sword"),
            Self::Spear => Some("spear"),
            Self::Axe => Some("axe"),
            Self::Bow | Self::Shield | Self::ImprovisedClub => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WildernessZone {
    pub zone_id: String,
    pub name: String,
    pub monster: MonsterKind,
    pub monster_health: u8,
    pub threat_active: bool,
    pub road_open: bool,
    pub position: Position,
    pub price_modifier_percent: i16,
    pub resource_demand: String,
    pub rumour: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContractStatus {
    Available,
    Accepted,
    Completed,
    Cooldown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdventurerContract {
    pub contract_id: String,
    pub title: String,
    pub description: String,
    pub target: MonsterKind,
    pub progress: u8,
    pub required_progress: u8,
    pub reward_gold: u32,
    pub status: ContractStatus,
    pub completion_count: u32,
    pub available_at_tick: u64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdventurerRank {
    #[default]
    Unproven,
    Trailhand,
    Pathfinder,
    RoadWarden,
}

impl AdventurerRank {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unproven => "Unproven",
            Self::Trailhand => "Trailhand",
            Self::Pathfinder => "Pathfinder",
            Self::RoadWarden => "Road Warden",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContractAction {
    Accept,
    Progress,
    Report,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractRequest {
    pub request_id: String,
    pub action: ContractAction,
    pub contract_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractResponse {
    pub request_id: String,
    pub accepted: bool,
    pub contract: AdventurerContract,
    pub player: PlayerProjection,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractsResponse {
    pub contracts: Vec<AdventurerContract>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CombatAction {
    Strike,
    Retreat,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CombatOutcome {
    Victory,
    KnockedOut,
    Retreated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CombatRequest {
    pub request_id: String,
    pub action: CombatAction,
    pub weapon: WeaponKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CombatResponse {
    pub request_id: String,
    pub accepted: bool,
    pub outcome: Option<CombatOutcome>,
    pub monster: MonsterKind,
    pub player: PlayerProjection,
    pub zone: WildernessZone,
    pub recovery_prompt: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryChoice {
    SelfRecover,
    AskRescuer,
    PayHealer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryRequest {
    pub request_id: String,
    pub choice: RecoveryChoice,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryResponse {
    pub request_id: String,
    pub accepted: bool,
    pub choice: RecoveryChoice,
    pub player: PlayerProjection,
    pub consequence: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronicleEntry {
    pub event_id: String,
    pub kind: String,
    pub title: String,
    pub text: String,
    pub created_tick: u64,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronicleSummary {
    pub from_tick: u64,
    pub to_tick: u64,
    pub from_cursor: u64,
    pub to_cursor: u64,
    pub entry_count: u32,
    pub kinds: Vec<String>,
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronicleResponse {
    pub entries: Vec<ChronicleEntry>,
    #[serde(default)]
    pub summary: Option<ChronicleSummary>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HouseholdMember {
    pub name: String,
    pub occupation: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HouseholdStatus {
    Travelling,
    Candidate,
    Arrived,
    Departed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpportunitySignal {
    pub household_id: String,
    pub household_name: String,
    pub members: Vec<HouseholdMember>,
    pub occupation: String,
    pub home_settlement: String,
    pub opportunity_score: i16,
    pub status: HouseholdStatus,
    pub service: String,
    pub clue: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpportunitiesResponse {
    pub opportunities: Vec<OpportunitySignal>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Active,
    Reclaimed,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LandClaim {
    pub claim_id: String,
    pub owner_account_id: String,
    pub owner_name: String,
    pub position: Position,
    pub lease_days: u32,
    pub last_active_tick: u64,
    pub reclaim_after_ticks: u64,
    pub status: ClaimStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimAction {
    Request,
    Renew,
    Abandon,
    Inspect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimRequest {
    pub request_id: String,
    pub action: ClaimAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimResponse {
    pub request_id: String,
    pub accepted: bool,
    pub claim: Option<LandClaim>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpeditionRole {
    Scout,
    Farmer,
    Builder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpeditionMember {
    pub account_id: String,
    pub display_name: String,
    pub role: ExpeditionRole,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpeditionStatus {
    Planning,
    Launched,
    Succeeded,
    Retreated,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpeditionRequirements {
    pub food: u32,
    pub tools: u32,
    pub materials: u32,
    pub safety: u32,
}

impl Default for ExpeditionRequirements {
    fn default() -> Self {
        Self {
            food: 6,
            tools: 3,
            materials: 8,
            safety: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Expedition {
    pub expedition_id: String,
    pub outpost_name: String,
    pub leader_account_id: String,
    pub members: Vec<ExpeditionMember>,
    pub food: u32,
    pub tools: u32,
    pub materials: u32,
    pub safety: u32,
    pub status: ExpeditionStatus,
    pub outcome: Option<String>,
    pub outpost_position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpeditionRequest {
    pub request_id: String,
    pub action: ExpeditionAction,
    pub expedition_id: Option<String>,
    pub role: Option<ExpeditionRole>,
    pub food: u32,
    pub tools: u32,
    pub materials: u32,
    pub safety: u32,
    pub outpost_name: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpeditionAction {
    Announce,
    Join,
    Supply,
    Launch,
    Resolve,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpeditionResponse {
    pub request_id: String,
    pub accepted: bool,
    pub expedition: Option<Expedition>,
    pub reason: Option<String>,
}
