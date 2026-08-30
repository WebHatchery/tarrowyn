//! Production-readiness authority: identity linking, audit, recovery, and observability.

use super::models::{RepositoryState, MAX_REPLAY_CACHE};
use super::*;
use crate::config::ServerConfig;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use tarrowyn_protocol::{
    AccountDeletionRequest, AccountDeletionResponse, AccountResponse, ApiResponse, AuditRecord,
    AuthLinkRequest, AuthLinkResponse, AuthRefreshRequest, AuthRefreshResponse, AuthRevokeRequest,
    AuthRevokeResponse, AuthSession, ModerationReportResponse, SupportRepairResponse,
};

mod account;
mod audit_helpers;
mod backup;
mod core_event_integrity;
mod core_replay_integrity;
mod core_session_integrity;
mod deletion;
mod maintenance;
mod moderation;
mod operations;
mod persistent_integrity;
mod phase3_replay_integrity;
mod phase4_integrity;
mod phase4_replay_integrity;
mod phase5_replay_integrity;
mod production_integrity;
mod regional_integrity;
mod repair;
mod retention;

pub(super) const MAX_AUDITS: usize = MAX_REPLAY_CACHE;

use account::migrate_guest_account_references;
pub(super) use audit_helpers::{
    audit, audit_command, issue_session, new_session_tokens, stable_fingerprint,
};
use deletion::PendingAccountDeletion;

const IDENTITY_PROVIDER: &str = "webhatchery-identity-oidc";
const PRIVACY_POLICY_VERSION: &str = "2026-08-19";
const MAX_DISPLAY_NAME_CHARS: usize = 80;
const MAX_REFRESH_TOKEN_CHARS: usize = 512;
pub(super) const MAX_MODERATION_REPORTS: usize = 512;
pub(super) const MODERATION_REPORT_RETENTION_SECONDS: u64 = 90 * 24 * 60 * 60;
pub(super) const MAX_PENDING_DELETIONS: usize = 128;

fn session_unavailable() -> RepositoryError {
    RepositoryError::new(
        503,
        "session_unavailable",
        "A secure session could not be issued; try again shortly.",
    )
}

pub(super) fn is_support_replay_key_for_account(
    key: &str,
    account_id: &str,
    response: &SupportRepairResponse,
) -> bool {
    key == format!("repair:{account_id}:{}", response.request_id)
}

pub(super) fn trim_auth_link_tokens(phase6: &mut Phase6State) {
    retention::trim_auth_link_tokens(phase6);
}

pub(super) fn trim_audits(audits: &mut VecDeque<AuditRecord>) {
    retention::trim_audits(audits);
}

pub(super) fn trim_moderation_reports(phase6: &mut Phase6State, now: u64) {
    retention::trim_moderation_reports(phase6, now);
}

pub(super) fn prune_moderation_cooldowns(state: &mut RepositoryState) {
    retention::prune_moderation_cooldowns(state);
}

pub(super) fn scheduled_backup(state: &mut RepositoryState, config: &ServerConfig) -> Option<bool> {
    maintenance::backup_due(state, config).then(|| backup::write(state, config))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ProductionAccount {
    pub(super) account_id: String,
    pub(super) provider: String,
    pub(super) subject: String,
    pub(super) identity_key: String,
    pub(super) guest_linked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ProductionSession {
    pub(super) identity_key: String,
    pub(super) account_id: String,
    pub(super) refresh_token: String,
    pub(super) expires_at_tick: u64,
    pub(super) refresh_expires_at_tick: u64,
    pub(super) revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Phase6State {
    pub(super) next_account_id: u64,
    pub(super) next_session_id: u64,
    pub(super) next_audit_id: u64,
    pub(super) accounts: HashMap<String, ProductionAccount>,
    pub(super) sessions: HashMap<String, ProductionSession>,
    #[serde(default)]
    pub(super) auth_link_results: HashMap<String, AuthLinkResponse>,
    #[serde(default)]
    pub(super) auth_link_tokens: HashMap<String, String>,
    #[serde(default)]
    pub(super) auth_refresh_results: HashMap<String, AuthRefreshResponse>,
    #[serde(default)]
    pub(super) auth_refresh_accounts: HashMap<String, String>,
    #[serde(default)]
    pub(super) auth_revoke_results: HashMap<String, AuthRevokeResponse>,
    #[serde(default)]
    pub(super) auth_revoke_guest_tokens: HashMap<String, String>,
    pub(super) audits: VecDeque<AuditRecord>,
    pub(super) reports: HashMap<String, ModerationReportResponse>,
    #[serde(default)]
    pub(super) report_created_at: HashMap<String, u64>,
    #[serde(default)]
    pub(super) moderation_results: HashMap<String, ModerationReportResponse>,
    #[serde(default)]
    pub(super) moderation_last_report_ticks: HashMap<String, u64>,
    pub(super) request_results: HashMap<String, SupportRepairResponse>,
    #[serde(default)]
    pub(super) deletion_requests: HashMap<String, PendingAccountDeletion>,
    #[serde(default)]
    pub(super) deletion_results: HashMap<String, AccountDeletionResponse>,
    pub(super) last_backup_tick: Option<u64>,
    pub(super) last_backup_path: Option<String>,
    pub(super) rejected_commands: u64,
    pub(super) completed_commands: u64,
}

impl Default for Phase6State {
    fn default() -> Self {
        fresh(&ServerConfig::default())
    }
}

pub(super) fn fresh(_config: &ServerConfig) -> Phase6State {
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

impl WorldRepository {
    pub fn auth_link(
        &self,
        token: &str,
        request: AuthLinkRequest,
    ) -> Result<ApiResponse<AuthLinkResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let (guest_key, replay_only) = match authenticate(&mut state, token, &self.config) {
            Ok(guest_key) => (guest_key, false),
            Err(error) => {
                let Some(guest_key) = state.phase6.auth_link_tokens.get(token).cloned() else {
                    return Err(error);
                };
                (guest_key, true)
            }
        };
        validate_request_id(&request.request_id)?;
        if request.provider != IDENTITY_PROVIDER {
            return Err(RepositoryError::new(
                400,
                "unsupported_provider",
                format!("Use the configured {} provider.", IDENTITY_PROVIDER),
            ));
        }
        let subject = request.subject.trim().to_owned();
        if subject.is_empty()
            || subject.chars().count() > 160
            || subject.chars().any(char::is_control)
        {
            return Err(RepositoryError::new(
                400,
                "invalid_subject",
                "The identity provider subject is required and bounded.",
            ));
        }
        if request.display_name.as_deref().is_some_and(|name| {
            let trimmed = name.trim();
            !trimmed.is_empty()
                && (trimmed.chars().count() > MAX_DISPLAY_NAME_CHARS
                    || trimmed.chars().any(char::is_control))
        }) {
            return Err(RepositoryError::new(
                400,
                "invalid_display_name",
                "The linked display name must be at most 80 characters and contain no control characters.",
            ));
        }
        let cache_key = format!("{}:{}", guest_key, request.request_id);
        if let Some(previous) = state.phase6.auth_link_results.get(&cache_key).cloned() {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous,
            });
        }
        if replay_only {
            return Err(RepositoryError::unauthorized());
        }
        if state
            .phase6
            .accounts
            .values()
            .any(|account| account.identity_key == guest_key)
        {
            return Err(RepositoryError::new(
                409,
                "guest_already_linked",
                "This guest character is already linked; continue with its existing identity.",
            ));
        }
        if state
            .phase6
            .accounts
            .values()
            .any(|account| account.provider == request.provider && account.subject == subject)
        {
            return Err(RepositoryError::new(
                409,
                "identity_already_linked",
                "That identity provider subject is already linked to another character.",
            ));
        }
        let account_id = state
            .phase6
            .accounts
            .values()
            .find(|account| account.provider == request.provider && account.subject == subject)
            .map(|account| account.account_id.clone())
            .unwrap_or_else(|| {
                let id = format!("account-{}", state.phase6.next_account_id);
                state.phase6.next_account_id = state.phase6.next_account_id.saturating_add(1);
                id
            });
        let display_name = request
            .display_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| {
                state
                    .identities
                    .get(&guest_key)
                    .map(|identity| identity.display_name.clone())
                    .unwrap_or_else(|| "Tarrowyn traveller".to_owned())
            });
        let old_account_id = state
            .identities
            .get(&guest_key)
            .expect("identity exists")
            .account_id
            .clone();
        let old_display_name = state
            .identities
            .get(&guest_key)
            .expect("identity exists")
            .display_name
            .clone();
        let session_tokens = new_session_tokens().map_err(|_| session_unavailable())?;
        migrate_guest_account_references(
            &mut state,
            &old_account_id,
            &account_id,
            &old_display_name,
            &display_name,
        );
        let character_id = {
            let identity = state
                .identities
                .get_mut(&guest_key)
                .expect("identity exists");
            identity.account_id = account_id.clone();
            identity.display_name = display_name.clone();
            identity.character_id.clone()
        };
        state.phase6.accounts.insert(
            account_id.clone(),
            ProductionAccount {
                account_id: account_id.clone(),
                provider: request.provider.clone(),
                subject,
                identity_key: guest_key.clone(),
                guest_linked: true,
            },
        );
        state.sessions.remove(token);
        state
            .phase6
            .auth_link_tokens
            .insert(token.to_owned(), guest_key.clone());
        let session = issue_session(
            &mut state,
            &self.config,
            &guest_key,
            &account_id,
            session_tokens,
        );
        audit(
            &mut state,
            &account_id,
            "auth.link",
            &account_id,
            "accepted",
            "A guest character was linked to the configured OIDC subject.",
        );
        let response = AuthLinkResponse {
            request_id: request.request_id.clone(),
            provider: request.provider,
            account_id,
            character_id,
            display_name,
            session,
            linked_guest: true,
        };
        state
            .phase6
            .auth_link_results
            .insert(cache_key, response.clone());
        record_command_outcome(&mut state, true);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(
                state.tick,
                Some(request.request_id.clone()),
                Some(state.cursor),
            ),
            data: response,
        })
    }

    pub fn auth_refresh(
        &self,
        request: AuthRefreshRequest,
    ) -> Result<ApiResponse<AuthRefreshResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        validate_request_id(&request.request_id)?;
        let refresh_token = validate_bounded_text(
            &request.refresh_token,
            MAX_REFRESH_TOKEN_CHARS,
            "invalid_refresh_token",
            "The refresh token must be bounded and contain no control characters.",
        )?;
        let cache_key = format!(
            "{}:{}",
            request.request_id,
            stable_fingerprint(&refresh_token)
        );
        if let Some(previous) = state.phase6.auth_refresh_results.get(&cache_key).cloned() {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous,
            });
        }
        let Some((old_token, old_session)) = state
            .phase6
            .sessions
            .iter()
            .find(|(_, session)| session.refresh_token == refresh_token && !session.revoked)
            .map(|(token, session)| (token.clone(), session.clone()))
        else {
            return Err(RepositoryError::new(
                401,
                "invalid_refresh",
                "That refresh session is expired or revoked.",
            ));
        };
        if old_session.refresh_expires_at_tick <= state.tick {
            return Err(RepositoryError::new(
                401,
                "refresh_expired",
                "Sign in again; the refresh session has expired.",
            ));
        }
        let session_tokens = new_session_tokens().map_err(|_| session_unavailable())?;
        if let Some(session) = state.phase6.sessions.get_mut(&old_token) {
            session.revoked = true;
        }
        state.sessions.remove(&old_token);
        let access = issue_session(
            &mut state,
            &self.config,
            &old_session.identity_key,
            &old_session.account_id,
            session_tokens,
        );
        audit(
            &mut state,
            &old_session.account_id,
            "auth.refresh",
            &old_session.account_id,
            "accepted",
            "An expiring access session was rotated.",
        );
        let response = AuthRefreshResponse {
            request_id: request.request_id.clone(),
            session: access,
        };
        state
            .phase6
            .auth_refresh_results
            .insert(cache_key.clone(), response.clone());
        state
            .phase6
            .auth_refresh_accounts
            .insert(cache_key, old_session.account_id.clone());
        record_command_outcome(&mut state, true);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(
                state.tick,
                Some(request.request_id.clone()),
                Some(state.cursor),
            ),
            data: response,
        })
    }

    pub fn auth_revoke(
        &self,
        token: &str,
        request: AuthRevokeRequest,
    ) -> Result<ApiResponse<AuthRevokeResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        validate_request_id(&request.request_id)?;
        let identity_hint = state
            .sessions
            .get(token)
            .map(|session| session.identity_key.clone())
            .or_else(|| {
                state
                    .phase6
                    .sessions
                    .get(token)
                    .map(|session| session.identity_key.clone())
            })
            .or_else(|| {
                state
                    .phase6
                    .auth_revoke_guest_tokens
                    .get(&stable_fingerprint(token))
                    .cloned()
            });
        if let Some(identity_key) = identity_hint.as_deref() {
            let cache_key = format!("{}:{}", identity_key, request.request_id);
            if let Some(previous) = state.phase6.auth_revoke_results.get(&cache_key).cloned() {
                return Ok(ApiResponse {
                    meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                    data: previous,
                });
            }
        }
        self.expire_and_persist_sessions(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        let account = state
            .identities
            .get(&key)
            .expect("identity exists")
            .account_id
            .clone();
        let guest_tokens: Vec<String> = state
            .sessions
            .iter()
            .filter(|(session_token, session)| {
                session.identity_key == key
                    && !state.phase6.sessions.contains_key(*session_token)
                    && (request.revoke_all || *session_token == token)
            })
            .map(|(session_token, _)| session_token.clone())
            .collect();
        let mut tokens: HashSet<String> = state
            .phase6
            .sessions
            .iter()
            .filter(|(session_token, session)| {
                session.account_id == account
                    && !session.revoked
                    && (request.revoke_all || *session_token == token)
            })
            .map(|(session_token, _)| session_token.clone())
            .collect();
        for (session_token, session) in &state.sessions {
            if session.identity_key == key && (request.revoke_all || session_token == token) {
                tokens.insert(session_token.clone());
            }
        }
        for session_token in &tokens {
            if let Some(session) = state.phase6.sessions.get_mut(session_token) {
                session.revoked = true;
            }
            state.sessions.remove(session_token);
        }
        for guest_token in guest_tokens {
            state
                .phase6
                .auth_revoke_guest_tokens
                .insert(stable_fingerprint(&guest_token), key.clone());
        }
        let revoked_refresh_replays: HashSet<String> = state
            .phase6
            .auth_refresh_accounts
            .iter()
            .filter(|(_, account_id)| *account_id == &account)
            .map(|(cache_key, _)| cache_key.clone())
            .collect();
        state
            .phase6
            .auth_refresh_results
            .retain(|cache_key, _| !revoked_refresh_replays.contains(cache_key));
        state
            .phase6
            .auth_refresh_accounts
            .retain(|cache_key, _| !revoked_refresh_replays.contains(cache_key));
        audit(
            &mut state,
            &account,
            "auth.revoke",
            &account,
            "accepted",
            "An access session was revoked by the account boundary.",
        );
        let response = AuthRevokeResponse {
            request_id: request.request_id.clone(),
            revoked_sessions: tokens.len() as u32,
        };
        state
            .phase6
            .auth_revoke_results
            .insert(format!("{}:{}", key, request.request_id), response.clone());
        record_command_outcome(&mut state, true);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(
                state.tick,
                Some(request.request_id.clone()),
                Some(state.cursor),
            ),
            data: response,
        })
    }

    pub fn account(&self, token: &str) -> Result<ApiResponse<AccountResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        let identity = state.identities.get(&key).expect("identity exists");
        let production = state.phase6.accounts.get(&identity.account_id);
        let expires = state
            .phase6
            .sessions
            .get(token)
            .map(|session| session.expires_at_tick)
            .unwrap_or_else(|| state.tick.saturating_add(self.config.session_ttl_ticks()));
        Ok(ApiResponse { meta: meta(state.tick, None, Some(state.cursor)), data: AccountResponse { account_id: identity.account_id.clone(), provider: production.map(|account| account.provider.clone()).unwrap_or_else(|| "development-guest".to_owned()), character_id: identity.character_id.clone(), display_name: identity.display_name.clone(), guest_fixture: production.is_none(), privacy_policy_version: PRIVACY_POLICY_VERSION.to_owned(), retention_note: "Account identity is retained until deletion; chat reports are retained for 90 days; settlement history is retained as public world history with account identifiers minimised.".to_owned(), session_expires_at_tick: expires, character: super::player_projection(&state, &key) } })
    }

    pub fn account_delete(
        &self,
        token: &str,
        request: AccountDeletionRequest,
    ) -> Result<ApiResponse<AccountDeletionResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        validate_request_id(&request.request_id)?;
        let requested_account_id = validate_bounded_text(
            &request.account_id,
            160,
            "invalid_account_id",
            "The account ID to delete must be bounded and contain no control characters.",
        )?;
        let deletion_replay_key = deletion::replay_key(token, &request.request_id);
        if let Some(previous) = state
            .phase6
            .deletion_results
            .get(&deletion_replay_key)
            .filter(|response| response.account_id == requested_account_id)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous.clone(),
            });
        }
        let key = authenticate(&mut state, token, &self.config)?;
        let account = state
            .identities
            .get(&key)
            .expect("identity exists")
            .account_id
            .clone();
        if account != requested_account_id {
            return Err(RepositoryError::new(
                403,
                "account_boundary_violation",
                "An account may delete only its own character boundary.",
            ));
        }
        if !state.phase6.accounts.contains_key(&account) {
            return Err(RepositoryError::new(
                409,
                "guest_account_deletion_not_supported",
                "Link this development guest to the identity gateway before requesting deletion.",
            ));
        }
        let cache_key = format!("delete:{account}:{}", request.request_id);
        if let Some(pending) = state.phase6.deletion_requests.get(&cache_key) {
            let mut response = deletion::scheduled_response(pending);
            response.request_id = request.request_id.clone();
            state
                .phase6
                .deletion_results
                .insert(deletion_replay_key, response.clone());
            self.persist(&state);
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response,
            });
        }
        if let Some(pending) = state
            .phase6
            .deletion_requests
            .values()
            .find(|pending| pending.account_id == account)
            .cloned()
        {
            let mut response = deletion::scheduled_response(&pending);
            response.request_id = request.request_id;
            state
                .phase6
                .deletion_results
                .insert(deletion_replay_key, response.clone());
            self.persist(&state);
            return Ok(ApiResponse {
                meta: meta(
                    state.tick,
                    Some(response.request_id.clone()),
                    Some(state.cursor),
                ),
                data: response,
            });
        }
        let character_id = state
            .identities
            .get(&key)
            .expect("identity exists")
            .character_id
            .clone();
        if state.phase6.deletion_requests.len() >= MAX_PENDING_DELETIONS {
            let response = AccountDeletionResponse {
                request_id: request.request_id,
                account_id: account,
                character_id,
                accepted: false,
                status: "blocked".to_owned(),
                reason: Some(
                    "The account-deletion queue is full; wait for the next authoritative tick before trying again."
                        .to_owned(),
                ),
            };
            record_command_outcome(&mut state, false);
            self.persist(&state);
            return Ok(ApiResponse {
                meta: meta(
                    state.tick,
                    Some(response.request_id.clone()),
                    Some(state.cursor),
                ),
                data: response,
            });
        }
        let pending = PendingAccountDeletion {
            request_id: request.request_id.clone(),
            account_id: account.clone(),
            identity_key: key,
            character_id: character_id.clone(),
            replay_key: deletion_replay_key.clone(),
        };
        state.phase6.deletion_requests.insert(cache_key, pending);
        audit(
            &mut state,
            &account,
            "account.delete.requested",
            &account,
            "accepted",
            "Account deletion was queued for the next authoritative tick.",
        );
        let response = AccountDeletionResponse {
            request_id: request.request_id.clone(),
            account_id: account,
            character_id,
            accepted: true,
            status: "scheduled".to_owned(),
            reason: None,
        };
        state
            .phase6
            .deletion_results
            .insert(deletion_replay_key, response.clone());
        record_command_outcome(&mut state, true);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }
}

pub(super) fn phase6_tick(state: &mut RepositoryState) {
    maintenance::run(state);
}

fn is_support_operator(config: &ServerConfig, account_id: &str) -> bool {
    config
        .support_operator_accounts
        .iter()
        .any(|operator| operator == account_id)
}

#[cfg(test)]
mod tests;
