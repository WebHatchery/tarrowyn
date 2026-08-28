//! Versioned wire types shared by the Tarrowyn client and development server.

use serde::{Deserialize, Serialize};

mod phase3;
pub use phase3::*;
mod phase4;
pub use phase4::*;
mod phase5;
pub use phase5::*;
mod phase6;
pub use phase6::*;
mod skills;
pub use skills::*;

pub const PROTOCOL_VERSION: &str = "6";
pub const MAX_CHAT_MESSAGE_LENGTH: usize = 160;
pub const MAX_TRADE_ITEMS: u32 = 99;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiMeta {
    pub protocol_version: String,
    pub server_tick: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<u64>,
}

impl ApiMeta {
    pub fn at(server_tick: u64) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION.to_owned(),
            server_tick,
            request_id: None,
            cursor: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiResponse<T> {
    pub meta: ApiMeta,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiErrorResponse {
    pub meta: ApiMeta,
    pub error: ApiError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub protocol_version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuestSessionRequest {
    #[serde(default)]
    pub client_key: Option<String>,
    #[serde(default)]
    pub reset: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuestSessionResponse {
    pub client_key: String,
    pub account_id: String,
    pub character_id: String,
    pub display_name: String,
    pub account_token: String,
    pub expires_in_seconds: u32,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn manhattan_distance(self, other: Self) -> u32 {
        self.x.abs_diff(other.x) + self.y.abs_diff(other.y)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TileKind {
    Meadow,
    Path,
    Field,
    Forest,
    Water,
    Stone,
}

impl TileKind {
    pub fn is_walkable(self) -> bool {
        !matches!(self, Self::Water)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldTile {
    pub position: Position,
    pub kind: TileKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldClock {
    pub day: u32,
    pub seconds: f32,
    pub day_length_seconds: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerPresence {
    pub account_id: String,
    pub character_id: String,
    pub display_name: String,
    pub position: Position,
    pub last_seen_tick: u64,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldSnapshot {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<WorldTile>,
    pub clock: WorldClock,
    pub players: Vec<PlayerPresence>,
    #[serde(default)]
    pub plots: Vec<FarmPlot>,
    #[serde(default)]
    pub tavern_position: Position,
    pub cursor: u64,
    #[serde(default)]
    pub wilderness: Option<WildernessZone>,
    #[serde(default)]
    pub outpost: Option<Position>,
    #[serde(default)]
    pub claim: Option<LandClaim>,
    #[serde(default)]
    pub expedition: Option<Expedition>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CropKind {
    Wheat,
    Turnip,
    Moonberry,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldWeather {
    #[default]
    Clear,
    DryWind,
    HeavyRain,
}

impl FieldWeather {
    pub fn label(self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::DryWind => "dry wind",
            Self::HeavyRain => "heavy rain",
        }
    }

    pub fn pressure(self) -> u8 {
        match self {
            Self::Clear => 0,
            Self::DryWind => 1,
            Self::HeavyRain => 2,
        }
    }
}

impl CropKind {
    pub fn value(self) -> u32 {
        match self {
            Self::Wheat => 3,
            Self::Turnip => 4,
            Self::Moonberry => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Inventory {
    pub wheat: u32,
    pub turnips: u32,
    pub moonberries: u32,
    pub seeds: u32,
    #[serde(default)]
    pub bandages: u32,
}

impl Inventory {
    pub fn total_items(self) -> u32 {
        self.wheat + self.turnips + self.moonberries + self.seeds + self.bandages
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CropState {
    pub kind: CropKind,
    pub stage: u8,
    pub quality: u8,
    pub planted_tick: u64,
    pub last_tended_tick: Option<u64>,
}

impl CropState {
    pub const MATURE_STAGE: u8 = 3;

    pub fn mature(self) -> bool {
        self.stage >= Self::MATURE_STAGE
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FarmPlot {
    pub position: Position,
    pub crop: Option<CropState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerProjection {
    pub account_id: String,
    pub character_id: String,
    pub display_name: String,
    pub position: Position,
    pub gold: u32,
    #[serde(default = "default_field_tool_condition")]
    pub field_tool_condition: u8,
    #[serde(default)]
    pub field_weather: FieldWeather,
    #[serde(default)]
    pub field_pest_pressure: u8,
    pub skill: u32,
    pub reputation: u32,
    #[serde(default)]
    pub adventurer_rank: AdventurerRank,
    #[serde(default)]
    pub adventurer_credentials: Vec<String>,
    pub inventory: Inventory,
    #[serde(default = "default_weapon")]
    pub weapon: WeaponKind,
    #[serde(default)]
    pub knocked_out: bool,
    #[serde(default)]
    pub injuries: u8,
    #[serde(default)]
    pub recovery_cost: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateSnapshot {
    pub world: WorldSnapshot,
    pub player: PlayerProjection,
    pub feed: TavernFeedResponse,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MovementIntent {
    pub request_id: String,
    pub dx: i32,
    pub dy: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MovementResponse {
    pub request_id: String,
    pub accepted: bool,
    pub position: Position,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatRequest {
    pub request_id: String,
    pub channel: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub message_id: u64,
    pub account_id: String,
    pub display_name: String,
    pub channel: String,
    pub text: String,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatResponse {
    pub request_id: String,
    pub accepted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<ChatMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FarmingAction {
    Plant,
    Tend,
    Harvest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FarmingRequest {
    pub request_id: String,
    pub action: FarmingAction,
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FarmingResponse {
    pub request_id: String,
    pub accepted: bool,
    pub action: FarmingAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plot: Option<FarmPlot>,
    pub player: PlayerProjection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradeBundle {
    pub wheat: u32,
    pub turnips: u32,
    pub moonberries: u32,
    pub seeds: u32,
    pub gold: u32,
}

impl TradeBundle {
    pub fn item_count(self) -> u32 {
        self.wheat + self.turnips + self.moonberries + self.seeds
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TradeStatus {
    Pending,
    Accepted,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradeOffer {
    pub trade_id: String,
    pub creator_account_id: String,
    pub creator_name: String,
    pub recipient_account_id: String,
    pub recipient_name: String,
    pub offer: TradeBundle,
    pub request: TradeBundle,
    pub status: TradeStatus,
    pub created_tick: u64,
    pub expires_tick: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TradeAction {
    Create,
    Review,
    Accept,
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradeRequest {
    pub request_id: String,
    pub action: TradeAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient_account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offer: Option<TradeBundle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<TradeBundle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradeResponse {
    pub request_id: String,
    pub accepted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade: Option<TradeOffer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradesResponse {
    pub trades: Vec<TradeOffer>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TavernNotice {
    pub notice_id: u64,
    pub kind: String,
    pub text: String,
    pub created_tick: u64,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TavernFeedResponse {
    pub notices: Vec<TavernNotice>,
    pub rumours: Vec<String>,
    pub chat: Vec<ChatMessage>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum WorldEvent {
    Presence(PlayerPresence),
    Clock(WorldClock),
    Chat(ChatMessage),
    Farming(FarmPlot),
    Trade(TradeOffer),
    TavernNotice(TavernNotice),
    Chronicle(ChronicleEntry),
    Frontier(FrontierEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum FrontierEvent {
    Threat(WildernessZone),
    Opportunity(OpportunitySignal),
    Claim(LandClaim),
    Expedition(Expedition),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventRecord {
    pub cursor: u64,
    pub event: WorldEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventsResponse {
    pub cursor: u64,
    pub clock: WorldClock,
    pub events: Vec<EventRecord>,
}

fn default_weapon() -> WeaponKind {
    WeaponKind::IronSword
}

fn default_field_tool_condition() -> u8 {
    3
}

#[cfg(test)]
mod tests;
