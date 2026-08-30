use super::deletion::PendingAccountDeletion;
use crate::config::ServerConfig;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tarrowyn_protocol::{
    AccountDeletionResponse, AuditRecord, AuthLinkResponse, AuthRefreshResponse,
    AuthRevokeResponse, ModerationReportResponse, SupportRepairResponse,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProductionAccount {
    pub(crate) account_id: String,
    pub(crate) provider: String,
    pub(crate) subject: String,
    pub(crate) identity_key: String,
    pub(crate) guest_linked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProductionSession {
    pub(crate) identity_key: String,
    pub(crate) account_id: String,
    pub(crate) refresh_token: String,
    pub(crate) expires_at_tick: u64,
    pub(crate) refresh_expires_at_tick: u64,
    pub(crate) revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Phase6State {
    pub(crate) next_account_id: u64,
    pub(crate) next_session_id: u64,
    pub(crate) next_audit_id: u64,
    pub(crate) accounts: HashMap<String, ProductionAccount>,
    pub(crate) sessions: HashMap<String, ProductionSession>,
    #[serde(default)]
    pub(crate) auth_link_results: HashMap<String, AuthLinkResponse>,
    #[serde(default)]
    pub(crate) auth_link_tokens: HashMap<String, String>,
    #[serde(default)]
    pub(crate) auth_refresh_results: HashMap<String, AuthRefreshResponse>,
    #[serde(default)]
    pub(crate) auth_refresh_accounts: HashMap<String, String>,
    #[serde(default)]
    pub(crate) auth_revoke_results: HashMap<String, AuthRevokeResponse>,
    #[serde(default)]
    pub(crate) auth_revoke_guest_tokens: HashMap<String, String>,
    pub(crate) audits: VecDeque<AuditRecord>,
    pub(crate) reports: HashMap<String, ModerationReportResponse>,
    #[serde(default)]
    pub(crate) report_created_at: HashMap<String, u64>,
    #[serde(default)]
    pub(crate) moderation_results: HashMap<String, ModerationReportResponse>,
    #[serde(default)]
    pub(crate) moderation_last_report_ticks: HashMap<String, u64>,
    pub(crate) request_results: HashMap<String, SupportRepairResponse>,
    #[serde(default)]
    pub(crate) deletion_requests: HashMap<String, PendingAccountDeletion>,
    #[serde(default)]
    pub(crate) deletion_results: HashMap<String, AccountDeletionResponse>,
    pub(crate) last_backup_tick: Option<u64>,
    pub(crate) last_backup_path: Option<String>,
    pub(crate) rejected_commands: u64,
    pub(crate) completed_commands: u64,
}

impl Default for Phase6State {
    fn default() -> Self {
        fresh(&ServerConfig::default())
    }
}

pub(crate) fn fresh(_config: &ServerConfig) -> Phase6State {
    Phase6State {
        next_account_id: 1,
        next_session_id: 1,
        next_audit_id: 1,
        accounts: HashMap::new(),
        sessions: HashMap::new(),
        auth_link_results: HashMap::new(),
        auth_link_tokens: HashMap::new(),
        auth_refresh_results: HashMap::new(),
        auth_refresh_accounts: HashMap::new(),
        auth_revoke_results: HashMap::new(),
        auth_revoke_guest_tokens: HashMap::new(),
        audits: VecDeque::new(),
        reports: HashMap::new(),
        report_created_at: HashMap::new(),
        moderation_results: HashMap::new(),
        moderation_last_report_ticks: HashMap::new(),
        request_results: HashMap::new(),
        deletion_requests: HashMap::new(),
        deletion_results: HashMap::new(),
        last_backup_tick: None,
        last_backup_path: None,
        rejected_commands: 0,
        completed_commands: 0,
    }
}
