//! Production identity, operations, support, and long-lived world protocol records.

use crate::{ChronicleEntry, ChronicleSummary, ClaimRecord, PlayerProjection, TradeOffer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthLinkRequest {
    pub request_id: String,
    pub provider: String,
    pub subject: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthSession {
    pub account_token: String,
    pub refresh_token: String,
    pub expires_in_seconds: u32,
    pub expires_at_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthLinkResponse {
    pub request_id: String,
    pub provider: String,
    pub account_id: String,
    pub character_id: String,
    pub display_name: String,
    pub session: AuthSession,
    pub linked_guest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthRefreshRequest {
    pub request_id: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthRefreshResponse {
    pub request_id: String,
    pub session: AuthSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthRevokeRequest {
    pub request_id: String,
    pub revoke_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthRevokeResponse {
    pub request_id: String,
    pub revoked_sessions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountResponse {
    pub account_id: String,
    pub provider: String,
    pub character_id: String,
    pub display_name: String,
    pub guest_fixture: bool,
    pub privacy_policy_version: String,
    pub retention_note: String,
    pub session_expires_at_tick: u64,
    pub character: PlayerProjection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountDeletionRequest {
    pub request_id: String,
    pub account_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountDeletionResponse {
    pub request_id: String,
    pub account_id: String,
    pub character_id: String,
    pub accepted: bool,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupportAccountResponse {
    pub account: AccountResponse,
    pub claims: Vec<ClaimRecord>,
    pub trades: Vec<TradeOffer>,
    pub chronicle: Vec<ChronicleEntry>,
    pub event_cursor: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SupportRepairAction {
    ClearStuckTravel,
    NormalizeInventory,
    ReconcileTrade,
    RestoreClaim,
    MergeHousehold,
    ResolveModeration,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupportRepairRequest {
    pub request_id: String,
    pub action: SupportRepairAction,
    pub account_id: Option<String>,
    pub target_id: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupportRepairResponse {
    pub request_id: String,
    pub audit_id: String,
    pub accepted: bool,
    pub summary: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditRecord {
    pub audit_id: String,
    pub actor_account_id: String,
    pub action: String,
    pub target: String,
    pub outcome: String,
    pub tick: u64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModerationReportRequest {
    pub request_id: String,
    pub target_account_id: Option<String>,
    pub message_id: Option<u64>,
    pub category: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModerationReportResponse {
    pub request_id: String,
    pub accepted: bool,
    pub report_id: String,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpsHealthResponse {
    pub status: String,
    pub ready: bool,
    pub storage_version: u32,
    pub protocol_version: String,
    pub last_backup_tick: Option<u64>,
    pub last_backup_path: Option<String>,
    pub integrity_ok: bool,
    pub persistence_error: Option<String>,
    pub backup_error: Option<String>,
    pub maintenance_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpsMetricsResponse {
    pub server_tick: u64,
    pub connected_sessions: u32,
    pub accounts: u32,
    pub region_entities_visible: u32,
    pub event_cursor: u64,
    pub regional_event_backlog: u32,
    pub open_market_orders: u32,
    pub travelling_players: u32,
    pub rejected_commands: u64,
    pub completed_commands: u64,
    pub average_tick_ms: u32,
    pub last_tick_ms: u32,
    pub tick_drift_count: u64,
    pub average_price_index_percent: u32,
    pub scarce_goods_count: u32,
    pub npc_fallback_households: u32,
    pub abandoned_claims: u32,
    pub declining_settlements: u32,
    pub newcomer_access: bool,
    pub alert_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronicleSearchResponse {
    pub query: String,
    pub entries: Vec<ChronicleEntry>,
    #[serde(default)]
    pub summary: Option<ChronicleSummary>,
    pub next_cursor: Option<u64>,
    pub cursor: u64,
}
