//! Regional protocol records for travel, logistics, markets, events, and migration.

use crate::{ChronicleEntry, Position};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocationKind {
    Settlement,
    Outpost,
    Frontier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocationRecord {
    pub location_id: String,
    pub name: String,
    pub kind: LocationKind,
    pub position: Position,
    pub role: String,
    pub resources: Vec<String>,
    pub services: Vec<String>,
    pub condition: u8,
    pub access_note: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouteStatus {
    Operational,
    Delayed,
    Threatened,
    Repairing,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteRecord {
    pub route_id: String,
    pub name: String,
    pub origin_location_id: String,
    pub destination_location_id: String,
    pub transport: String,
    pub length: u32,
    pub risk_percent: u8,
    pub condition: u8,
    pub capacity: u32,
    pub travel_ticks: u64,
    pub repair_cost: u32,
    pub status: RouteStatus,
    pub last_action_tick: u64,
    pub note: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TravelStatus {
    Idle,
    Travelling,
    Interrupted,
    Arrived,
    Recovering,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TravelState {
    pub travel_id: String,
    pub route_id: String,
    pub origin_location_id: String,
    pub destination_location_id: String,
    pub departure_tick: u64,
    pub eta_tick: u64,
    pub progress: u8,
    pub risk_percent: u8,
    pub status: TravelStatus,
    pub interruption: Option<String>,
    pub recovery_note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TravelAction {
    Start,
    Interrupt,
    Resume,
    Recover,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TravelRequest {
    pub request_id: String,
    pub action: TravelAction,
    pub route_id: Option<String>,
    pub travel_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TravelResponse {
    pub request_id: String,
    pub accepted: bool,
    pub travel: Option<TravelState>,
    pub location_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SettlementCondition {
    Flourishing,
    Stable,
    Strained,
    Quiet,
    Recovering,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SettlementProjection {
    pub settlement_id: String,
    pub name: String,
    pub location_id: String,
    pub population: u32,
    pub food: u8,
    pub safety: u8,
    pub infrastructure: u8,
    pub industry: u8,
    pub governance: u8,
    pub player_activity: u8,
    #[serde(default)]
    pub claim_count: u32,
    #[serde(default)]
    pub available_plot_count: u32,
    #[serde(default)]
    pub public_works: Vec<String>,
    pub condition: SettlementCondition,
    pub milestones: Vec<String>,
    pub vacancies: Vec<String>,
    pub demand: Vec<String>,
    pub abundant_goods: Vec<String>,
    pub scarce_goods: Vec<String>,
    pub price_index_percent: u16,
    pub chronicle: Vec<ChronicleEntry>,
    pub recovery_opportunity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SettlementsResponse {
    pub settlements: Vec<SettlementProjection>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionSnapshot {
    pub region_id: String,
    pub season: String,
    pub calendar_day: u32,
    pub locations: Vec<LocationRecord>,
    pub routes: Vec<RouteRecord>,
    pub visible_settlements: Vec<SettlementProjection>,
    pub player_location_id: String,
    pub travel: Option<TravelState>,
    pub interest_radius: u32,
    pub cursor: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouteAction {
    Repair,
    Escort,
    Improve,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteRequest {
    pub request_id: String,
    pub route_id: String,
    pub action: RouteAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteResponse {
    pub request_id: String,
    pub accepted: bool,
    pub route: RouteRecord,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommodityKind {
    Wheat,
    Turnips,
    Moonberries,
    Seeds,
    Timber,
    Stone,
    Bandages,
}

impl CommodityKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Wheat => "wheat",
            Self::Turnips => "turnips",
            Self::Moonberries => "moonberries",
            Self::Seeds => "seeds",
            Self::Timber => "timber",
            Self::Stone => "stone",
            Self::Bandages => "bandages",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketOrderStatus {
    Open,
    Fulfilled,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketOrder {
    pub order_id: String,
    pub owner_account_id: String,
    pub owner_name: String,
    pub origin_location_id: String,
    pub destination_location_id: String,
    pub commodity: CommodityKind,
    pub quantity: u32,
    pub unit_price: u32,
    pub total_price: u32,
    pub status: MarketOrderStatus,
    pub created_tick: u64,
    pub settled_tick: Option<u64>,
    pub route_id: String,
    pub fallback_used: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketOrderAction {
    Create,
    Fulfil,
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketOrderRequest {
    pub request_id: String,
    pub action: MarketOrderAction,
    pub order_id: Option<String>,
    pub destination_location_id: Option<String>,
    pub commodity: Option<CommodityKind>,
    pub quantity: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketOrderResponse {
    pub request_id: String,
    pub accepted: bool,
    pub order: Option<MarketOrder>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketSnapshot {
    pub orders: Vec<MarketOrder>,
    pub stock_notes: Vec<String>,
    pub prices: Vec<String>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegionalEventStage {
    Signal,
    Escalation,
    Intervention,
    Resolution,
    Aftermath,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionalEvent {
    pub event_id: String,
    pub title: String,
    pub kind: String,
    pub stage: RegionalEventStage,
    pub affected_location_ids: Vec<String>,
    pub effects: Vec<String>,
    pub cause: String,
    pub intervention_options: Vec<String>,
    pub chosen_intervention: Option<String>,
    pub outcome: Option<String>,
    pub started_tick: u64,
    pub updated_tick: u64,
    pub cursor: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegionalEventAction {
    Seed,
    Intervene,
    Resolve,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionalEventRequest {
    pub request_id: String,
    pub action: RegionalEventAction,
    pub event_id: Option<String>,
    pub intervention: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionalEventResponse {
    pub request_id: String,
    pub accepted: bool,
    pub event: Option<RegionalEvent>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionalEventsResponse {
    pub events: Vec<RegionalEvent>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionalHousehold {
    pub household_id: String,
    pub household_name: String,
    pub origin_location_id: String,
    pub destination_location_id: Option<String>,
    pub status: String,
    pub reason: String,
    pub service: String,
    pub departure_tick: Option<u64>,
    pub arrival_tick: Option<u64>,
    pub history: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionalHouseholdsResponse {
    pub households: Vec<RegionalHousehold>,
    pub vacancies: Vec<String>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LawBoundaryResponse {
    pub pvp_enabled: bool,
    pub theft_enabled: bool,
    pub claims_protected: bool,
    pub trade_protected: bool,
    pub travel_protected: bool,
    pub recovery_path: String,
    pub policy_version: String,
}
