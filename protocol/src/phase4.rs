use super::{PlayerProjection, Position, WeaponKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OfficeKind {
    Steward,
    WorksWarden,
    Registrar,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficeRecord {
    pub office_id: String,
    pub kind: OfficeKind,
    pub title: String,
    pub authority: String,
    pub holder_account_id: Option<String>,
    pub holder_name: Option<String>,
    pub last_active_tick: u64,
    pub vacant: bool,
    pub vacancy_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PublicAction {
    RepairRoad,
    FundService,
    HostFestival,
    CommissionPublicWork,
    UpdateContractBoard,
}

impl PublicAction {
    pub fn default_cost(self) -> u32 {
        match self {
            Self::RepairRoad => 8,
            Self::FundService => 5,
            Self::HostFestival => 4,
            Self::CommissionPublicWork => 12,
            Self::UpdateContractBoard => 3,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::RepairRoad => "repair the north road",
            Self::FundService => "fund a settlement service",
            Self::HostFestival => "host a settlement festival",
            Self::CommissionPublicWork => "commission a public work",
            Self::UpdateContractBoard => "update the contract board",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Proposed,
    Approved,
    Completed,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicProposal {
    pub proposal_id: String,
    pub proposer_account_id: String,
    pub proposer_name: String,
    pub action: PublicAction,
    pub target: String,
    pub cost: u32,
    pub status: ProposalStatus,
    pub created_tick: u64,
    pub approved_by: Option<String>,
    pub completed_tick: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceDecision {
    pub decision_id: String,
    pub actor_account_id: String,
    pub actor_name: String,
    pub action: PublicAction,
    pub proposal_id: String,
    pub cost: u32,
    pub service_affected: String,
    pub created_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaxPolicy {
    pub payer: String,
    pub recipient: String,
    pub rate_percent: u8,
    pub exemptions: Vec<String>,
    pub accounting_note: String,
    pub recovery_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaxCollection {
    pub collection_id: String,
    pub payer_account_id: String,
    pub payer_name: String,
    pub amount: u32,
    pub rate_percent: u8,
    pub territory: String,
    pub day: u32,
    pub created_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceState {
    pub settlement_id: String,
    pub offices: Vec<OfficeRecord>,
    pub proposals: Vec<PublicProposal>,
    pub decisions: Vec<GovernanceDecision>,
    pub public_treasury: u32,
    pub administration_quality: u8,
    pub service_funding_until_tick: u64,
    pub taxation: Option<TaxPolicy>,
    #[serde(default)]
    pub tax_ledger: Vec<TaxCollection>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceAction {
    Inspect,
    ClaimOffice,
    Propose,
    Approve,
    Complete,
    SetTaxRate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceRequest {
    pub request_id: String,
    pub action: GovernanceAction,
    pub office_id: Option<String>,
    pub proposal_id: Option<String>,
    pub public_action: Option<PublicAction>,
    pub target: Option<String>,
    pub cost: Option<u32>,
    #[serde(default)]
    pub tax_rate_percent: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceResponse {
    pub request_id: String,
    pub accepted: bool,
    pub governance: GovernanceState,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InfrastructureKind {
    Road,
    Bridge,
    Plot,
    PublicBuilding,
    Service,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InfrastructureStatus {
    Operational,
    NeedsRepair,
    Failed,
    Recovering,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InfrastructureRecord {
    pub infrastructure_id: String,
    pub name: String,
    pub kind: InfrastructureKind,
    pub position: Position,
    pub condition: u8,
    pub upkeep_per_day: u32,
    pub service_quality: u8,
    pub status: InfrastructureStatus,
    pub last_maintained_tick: u64,
    pub failure_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InfrastructureResponse {
    pub records: Vec<InfrastructureRecord>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimLifecycleAction {
    Request,
    Approve,
    Renew,
    Transfer,
    Inherit,
    Abandon,
    Reclaim,
    Inspect,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimLifecycleStatus {
    Requested,
    Active,
    Renewed,
    Transferred,
    Inherited,
    Abandoned,
    Expired,
    Reclaimed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimRecord {
    pub claim_id: String,
    pub plot_id: String,
    pub owner_account_id: Option<String>,
    pub owner_name: Option<String>,
    pub position: Position,
    pub lease_days: u32,
    pub started_tick: u64,
    pub expires_tick: u64,
    #[serde(default)]
    pub started_at_unix_seconds: u64,
    #[serde(default)]
    pub expires_at_unix_seconds: u64,
    pub last_active_tick: u64,
    pub status: ClaimLifecycleStatus,
    pub approved_by: Option<String>,
    pub building_access: bool,
    pub protected_goods_policy: String,
    pub inspection_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimsResponse {
    pub claims: Vec<ClaimRecord>,
    pub available_plots: Vec<Position>,
    #[serde(default)]
    pub lease_duration_days: u32,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimLifecycleRequest {
    pub request_id: String,
    pub action: ClaimLifecycleAction,
    pub claim_id: Option<String>,
    pub target_account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimLifecycleResponse {
    pub request_id: String,
    pub accepted: bool,
    pub claim: Option<ClaimRecord>,
    pub claims: ClaimsResponse,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProfessionKind {
    Farmer,
    Smith,
    Carpenter,
    Healer,
    Scout,
    Steward,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capability {
    pub capability_id: String,
    pub name: String,
    pub profession: ProfessionKind,
    pub level: u8,
    pub description: String,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfessionProfile {
    pub profession: ProfessionKind,
    pub level: u8,
    pub reputation: u32,
    pub credential: Option<String>,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MaterialStock {
    pub wood: u32,
    pub iron: u32,
    pub cloth: u32,
    pub bandages: u32,
    pub tools: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceOrderStatus {
    Open,
    Accepted,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceOrder {
    pub order_id: String,
    pub requester_account_id: String,
    pub requester_name: String,
    pub provider_account_id: Option<String>,
    pub provider_name: Option<String>,
    pub service: String,
    pub required_profession: ProfessionKind,
    pub materials: MaterialStock,
    pub tools_required: u32,
    pub reward_gold: u32,
    pub benefit: String,
    pub status: ServiceOrderStatus,
    pub quality: u8,
    pub created_tick: u64,
    pub completed_tick: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProfessionAction {
    Inspect,
    CreateOrder,
    AcceptOrder,
    CompleteOrder,
    LearnCapability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfessionRequest {
    pub request_id: String,
    pub action: ProfessionAction,
    pub order_id: Option<String>,
    pub profession: Option<ProfessionKind>,
    pub capability_id: Option<String>,
    pub service: Option<String>,
    #[serde(default)]
    pub timing_score: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfessionsResponse {
    pub profiles: Vec<ProfessionProfile>,
    pub orders: Vec<ServiceOrder>,
    pub materials: MaterialStock,
    pub credentials: Vec<String>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfessionResponse {
    pub request_id: String,
    pub accepted: bool,
    pub professions: ProfessionsResponse,
    pub order: Option<ServiceOrder>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeKind {
    CropTechnique,
    MonsterClue,
    Route,
    Recipe,
    MaterialProperty,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeItem {
    pub knowledge_id: String,
    pub title: String,
    pub kind: KnowledgeKind,
    pub description: String,
    pub effect: String,
    pub teachable: bool,
    pub writable: bool,
    pub discovered_by: Vec<String>,
    pub stored_in: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeState {
    pub items: Vec<KnowledgeItem>,
    pub known_by_player: Vec<String>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeAction {
    Inspect,
    Discover,
    Teach,
    Record,
    Apply,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeRequest {
    pub request_id: String,
    pub action: KnowledgeAction,
    pub knowledge_id: Option<String>,
    pub target_account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeResponse {
    pub request_id: String,
    pub accepted: bool,
    pub knowledge: KnowledgeState,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HouseholdMemberRecord {
    pub name: String,
    pub role: String,
    pub service: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HouseholdLifeStatus {
    Arrived,
    ReducedService,
    ConsideringDeparture,
    Departed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HouseholdRecord {
    pub household_id: String,
    pub household_name: String,
    pub members: Vec<HouseholdMemberRecord>,
    pub home: String,
    pub needs: Vec<String>,
    pub work: String,
    pub service_quality: u8,
    pub demand: u8,
    pub housing: u8,
    pub safety: u8,
    pub food: u8,
    pub competition: u8,
    pub status: HouseholdLifeStatus,
    pub clue: String,
    pub last_decision_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HouseholdsResponse {
    pub households: Vec<HouseholdRecord>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalCombatAction {
    Prepare,
    Strike,
    Technique,
    Guard,
    Retreat,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalCombatStatus {
    Ready,
    Engaged,
    Victorious,
    KnockedOut,
    Retreated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalCombatState {
    pub encounter_id: String,
    pub enemy_name: String,
    pub enemy_health: u8,
    pub player_health: u8,
    pub turn: u32,
    pub status: LocalCombatStatus,
    pub weapon: WeaponKind,
    pub injury_limit: u8,
    pub stored_property_safe: bool,
    pub carried_risk: String,
    pub recovery_cost: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalCombatRequest {
    pub request_id: String,
    pub action: LocalCombatAction,
    pub weapon: WeaponKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalCombatResponse {
    pub request_id: String,
    pub accepted: bool,
    pub combat: LocalCombatState,
    pub player: PlayerProjection,
    pub prompt: String,
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests;
