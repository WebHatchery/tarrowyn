//! Versioned wire types shared by the Tarrowyn client and development server.

use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: &str = "1";
pub const MAX_CHAT_MESSAGE_LENGTH: usize = 160;

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum WorldEvent {
    Presence(PlayerPresence),
    Clock(WorldClock),
    Chat(ChatMessage),
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

#[cfg(test)]
mod tests;
