//! Production-readiness authority: identity linking, audit, recovery, and observability.

use super::models::{trim_replay_cache, RepositoryState, MAX_REPLAY_CACHE};
use super::*;
use crate::config::ServerConfig;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tarrowyn_protocol::{
    AccountDeletionRequest, AccountDeletionResponse, AccountResponse, ApiResponse, AuditRecord,
    AuthLinkRequest, AuthLinkResponse, AuthRefreshRequest, AuthRefreshResponse, AuthRevokeRequest,
    AuthRevokeResponse, AuthSession, ModerationReportResponse, SupportRepairAction,
    SupportRepairRequest, SupportRepairResponse,
};

mod backup;
mod deletion;
mod moderation;
mod operations;

use deletion::PendingAccountDeletion;

const IDENTITY_PROVIDER: &str = "webhatchery-identity-oidc";
const PRIVACY_POLICY_VERSION: &str = "2026-08-19";

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
    pub(super) auth_refresh_results: HashMap<String, AuthRefreshResponse>,
    #[serde(default)]
    pub(super) auth_revoke_results: HashMap<String, AuthRevokeResponse>,
    pub(super) audits: VecDeque<AuditRecord>,
    pub(super) reports: HashMap<String, ModerationReportResponse>,
    #[serde(default)]
    pub(super) moderation_results: HashMap<String, ModerationReportResponse>,
    #[serde(default)]
    pub(super) moderation_last_report_ticks: HashMap<String, u64>,
    pub(super) request_results: HashMap<String, SupportRepairResponse>,
    #[serde(default)]
    pub(super) deletion_requests: HashMap<String, PendingAccountDeletion>,
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
        auth_refresh_results: HashMap::new(),
        auth_revoke_results: HashMap::new(),
        audits: VecDeque::new(),
        reports: HashMap::new(),
        moderation_results: HashMap::new(),
        moderation_last_report_ticks: HashMap::new(),
        request_results: HashMap::new(),
        deletion_requests: HashMap::new(),
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
        expire_sessions(&mut state, &self.config);
        let guest_key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        if request.provider != IDENTITY_PROVIDER {
            return Err(RepositoryError::new(
                400,
                "unsupported_provider",
                format!("Use the configured {} provider.", IDENTITY_PROVIDER),
            ));
        }
        if request.subject.trim().is_empty() || request.subject.len() > 160 {
            return Err(RepositoryError::new(
                400,
                "invalid_subject",
                "The identity provider subject is required and bounded.",
            ));
        }
        let cache_key = format!("{}:{}", guest_key, request.request_id);
        if let Some(previous) = state.phase6.auth_link_results.get(&cache_key).cloned() {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous,
            });
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
        if state.phase6.accounts.values().any(|account| {
            account.provider == request.provider && account.subject == request.subject
        }) {
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
            .find(|account| {
                account.provider == request.provider && account.subject == request.subject
            })
            .map(|account| account.account_id.clone())
            .unwrap_or_else(|| {
                let id = format!("account-{}", state.phase6.next_account_id);
                state.phase6.next_account_id = state.phase6.next_account_id.saturating_add(1);
                id
            });
        let display_name = request
            .display_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| {
                state
                    .identities
                    .get(&guest_key)
                    .map(|identity| identity.display_name.clone())
                    .unwrap_or_else(|| "Tarrowyn traveller".to_owned())
            });
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
                subject: request.subject,
                identity_key: guest_key.clone(),
                guest_linked: true,
            },
        );
        state.sessions.remove(token);
        let session = issue_session(&mut state, &self.config, &guest_key, &account_id);
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
        let cache_key = format!(
            "{}:{:016x}",
            request.request_id,
            stable_fingerprint(&request.refresh_token)
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
            .find(|(_, session)| session.refresh_token == request.refresh_token && !session.revoked)
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
        if let Some(session) = state.phase6.sessions.get_mut(&old_token) {
            session.revoked = true;
        }
        state.sessions.remove(&old_token);
        let access = issue_session(
            &mut state,
            &self.config,
            &old_session.identity_key,
            &old_session.account_id,
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
        let key = authenticate(&mut state, token, &self.config)?;
        let account = state
            .identities
            .get(&key)
            .expect("identity exists")
            .account_id
            .clone();
        let tokens: Vec<String> = state
            .phase6
            .sessions
            .iter()
            .filter(|(session_token, session)| {
                session.account_id == account && (request.revoke_all || *session_token == token)
            })
            .map(|(session_token, _)| session_token.clone())
            .collect();
        for session_token in &tokens {
            if let Some(session) = state.phase6.sessions.get_mut(session_token) {
                session.revoked = true;
            }
            state.sessions.remove(session_token);
        }
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
        validate_request_id(&request.request_id)?;
        if request.account_id.trim().is_empty() || request.account_id.len() > 160 {
            return Err(RepositoryError::new(
                400,
                "invalid_account_id",
                "The account ID to delete is required and bounded.",
            ));
        }
        let key = authenticate(&mut state, token, &self.config)?;
        let account = state
            .identities
            .get(&key)
            .expect("identity exists")
            .account_id
            .clone();
        if account != request.account_id {
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
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: deletion::scheduled_response(pending),
            });
        }
        let character_id = state
            .identities
            .get(&key)
            .expect("identity exists")
            .character_id
            .clone();
        let pending = PendingAccountDeletion {
            request_id: request.request_id.clone(),
            account_id: account.clone(),
            identity_key: key,
            character_id: character_id.clone(),
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
        record_command_outcome(&mut state, true);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }

    pub fn support_repair(
        &self,
        token: &str,
        request: SupportRepairRequest,
    ) -> Result<ApiResponse<SupportRepairResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        let actor_key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let actor = state
            .identities
            .get(&actor_key)
            .expect("identity exists")
            .account_id
            .clone();
        if !is_support_operator(&self.config, &actor) {
            return Err(RepositoryError::new(
                403,
                "support_operator_required",
                "A configured support operator account is required for repair actions.",
            ));
        }
        if request.note.trim().is_empty() {
            return Err(RepositoryError::new(
                400,
                "repair_note_required",
                "Every support repair needs an operator note.",
            ));
        }
        let cache = format!(
            "repair:{}:{}",
            state
                .identities
                .get(&actor_key)
                .expect("identity exists")
                .account_id,
            request.request_id
        );
        if let Some(previous) = state.phase6.request_results.get(&cache) {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous.clone(),
            });
        }
        let target_account = request.account_id.clone().unwrap_or_else(|| actor.clone());
        let target_key = state
            .identities
            .iter()
            .find(|(_, identity)| identity.account_id == target_account)
            .map(|(key, _)| key.clone());
        let (accepted, summary, reason) = match request.action {
            SupportRepairAction::ClearStuckTravel => { if let Some(target_key) = target_key { state.phase5.travel.remove(&target_key); if let Some(identity) = state.identities.get_mut(&target_key) { identity.position = crate::content::region_location_profile("hearth").position; } (true, "Stuck travel cleared at the origin with cargo and rewards preserved.".to_owned(), None) } else { (false, String::new(), Some("The target account is not present.".to_owned())) } }
            SupportRepairAction::NormalizeInventory => { if let Some(target_key) = target_key { if let Some(identity) = state.identities.get_mut(&target_key) { identity.inventory.wheat = identity.inventory.wheat.min(9_999); identity.inventory.turnips = identity.inventory.turnips.min(9_999); identity.inventory.moonberries = identity.inventory.moonberries.min(9_999); identity.inventory.seeds = identity.inventory.seeds.min(9_999); } (true, "Inventory values were normalised to the documented support ceiling.".to_owned(), None) } else { (false, String::new(), Some("The target account is not present.".to_owned())) } }
            SupportRepairAction::ReconcileTrade => match request.target_id.as_deref() {
                None => (false, String::new(), Some("A market order ID is required.".to_owned())),
                Some(order_id) => match state.phase5.market_orders.iter_mut().find(|order| order.order_id == order_id && matches!(order.status, tarrowyn_protocol::MarketOrderStatus::Open)) {
                    None => (false, String::new(), Some("No open market order needs reconciliation.".to_owned())),
                    Some(order) => { order.status = tarrowyn_protocol::MarketOrderStatus::Cancelled; (true, "The open order was cancelled; its audit trail remains available for refund review.".to_owned(), None) }
                },
            },
            SupportRepairAction::RestoreClaim => (true, "Claim repair is recorded for the claim service to reconcile on its next bounded tick.".to_owned(), None),
            SupportRepairAction::MergeHousehold => (true, "Household repair is recorded without deleting either household history.".to_owned(), None),
            SupportRepairAction::ResolveModeration => { let report = request.target_id.as_deref().and_then(|id| state.phase6.reports.get_mut(id)); if let Some(report) = report { report.status = "resolved".to_owned(); (true, "Moderation report marked resolved and retained in the audit record.".to_owned(), None) } else { (false, String::new(), Some("That moderation report is not recorded.".to_owned())) } }
        };
        let audit_id = audit(
            &mut state,
            &actor,
            "support.repair",
            &target_account,
            if accepted { "accepted" } else { "rejected" },
            &request.note,
        );
        let response = SupportRepairResponse {
            request_id: request.request_id.clone(),
            audit_id,
            accepted,
            summary,
            reason,
        };
        state.phase6.request_results.insert(cache, response.clone());
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }
}

pub(super) fn phase6_tick(state: &mut RepositoryState, config: &ServerConfig) -> Option<bool> {
    deletion::process(state);
    trim_replay_cache(&mut state.phase6.auth_link_results);
    trim_replay_cache(&mut state.phase6.auth_refresh_results);
    trim_replay_cache(&mut state.phase6.auth_revoke_results);
    trim_replay_cache(&mut state.phase6.moderation_results);
    trim_replay_cache(&mut state.phase6.moderation_last_report_ticks);
    trim_replay_cache(&mut state.phase3.request_results);
    trim_replay_cache(&mut state.phase4.request_results);
    trim_replay_cache(&mut state.phase5.request_results);
    trim_replay_cache(&mut state.phase6.request_results);
    trim_replay_cache(&mut state.phase6.deletion_requests);
    for identity in state.identities.values_mut() {
        trim_replay_cache(&mut identity.farming_results);
        trim_replay_cache(&mut identity.trade_results);
        trim_replay_cache(&mut identity.movement_results);
        trim_replay_cache(&mut identity.chat_results);
    }
    state.phase6.audits.truncate(MAX_REPLAY_CACHE);
    if config.backup_interval_ticks > 0 && state.tick.is_multiple_of(config.backup_interval_ticks) {
        Some(backup::write(state, config))
    } else {
        None
    }
}

fn is_support_operator(config: &ServerConfig, account_id: &str) -> bool {
    config
        .support_operator_accounts
        .iter()
        .any(|operator| operator == account_id)
}

pub(super) fn audit_command(
    state: &mut RepositoryState,
    actor: &str,
    action: &str,
    target: &str,
    accepted: bool,
    note: &str,
) {
    audit(
        state,
        actor,
        action,
        target,
        if accepted { "accepted" } else { "rejected" },
        note,
    );
}

fn stable_fingerprint(value: &str) -> u64 {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211_u64);
    }
    hash
}

fn validate_request_id(request_id: &str) -> Result<(), RepositoryError> {
    if request_id.trim().is_empty() || request_id.len() > 64 {
        Err(RepositoryError::new(
            400,
            "invalid_request_id",
            "Production request IDs must contain 1 to 64 characters.",
        ))
    } else {
        Ok(())
    }
}

fn issue_session(
    state: &mut RepositoryState,
    config: &ServerConfig,
    identity_key: &str,
    account_id: &str,
) -> AuthSession {
    let id = state.phase6.next_session_id;
    state.phase6.next_session_id = state.phase6.next_session_id.saturating_add(1);
    let access = format!("prod-session-{id}");
    let refresh = format!("prod-refresh-{id}");
    let expires = state
        .tick
        .saturating_add(config.production_session_ttl_ticks());
    let refresh_expires = state.tick.saturating_add(config.refresh_ttl_ticks());
    state.phase6.sessions.insert(
        access.clone(),
        ProductionSession {
            identity_key: identity_key.to_owned(),
            account_id: account_id.to_owned(),
            refresh_token: refresh.clone(),
            expires_at_tick: expires,
            refresh_expires_at_tick: refresh_expires,
            revoked: false,
        },
    );
    state.sessions.insert(
        access.clone(),
        Session {
            client_key: identity_key.to_owned(),
            identity_key: identity_key.to_owned(),
            last_seen_tick: state.tick,
            last_movement_tick: None,
            last_chat_tick: None,
        },
    );
    AuthSession {
        account_token: access,
        refresh_token: refresh,
        expires_in_seconds: config.production_session_ttl_seconds,
        expires_at_tick: expires,
    }
}

fn audit(
    state: &mut RepositoryState,
    actor: &str,
    action: &str,
    target: &str,
    outcome: &str,
    note: &str,
) -> String {
    let audit_id = format!("audit-{}", state.phase6.next_audit_id);
    state.phase6.next_audit_id = state.phase6.next_audit_id.saturating_add(1);
    state.phase6.audits.push_back(AuditRecord {
        audit_id: audit_id.clone(),
        actor_account_id: actor.to_owned(),
        action: action.to_owned(),
        target: target.to_owned(),
        outcome: outcome.to_owned(),
        tick: state.tick,
        note: note.chars().take(240).collect(),
    });
    audit_id
}

#[cfg(test)]
mod tests;
