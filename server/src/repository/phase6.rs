//! Production-readiness authority: identity linking, audit, recovery, and observability.

use super::models::RepositoryState;
use super::*;
use crate::config::ServerConfig;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use tarrowyn_protocol::{
    AccountResponse, ApiResponse, AuditRecord, AuthLinkRequest, AuthLinkResponse,
    AuthRefreshRequest, AuthRefreshResponse, AuthRevokeRequest, AuthRevokeResponse, AuthSession,
    ChronicleSearchResponse, ModerationReportRequest, ModerationReportResponse, OpsHealthResponse,
    OpsMetricsResponse, SupportRepairAction, SupportRepairRequest, SupportRepairResponse,
};

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
    pub(super) audits: VecDeque<AuditRecord>,
    pub(super) reports: HashMap<String, ModerationReportResponse>,
    pub(super) request_results: HashMap<String, SupportRepairResponse>,
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
        audits: VecDeque::new(),
        reports: HashMap::new(),
        request_results: HashMap::new(),
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
        let session = issue_session(&mut state, &self.config, &guest_key, &account_id);
        audit(
            &mut state,
            &account_id,
            "auth.link",
            &account_id,
            "accepted",
            "A guest character was linked to the configured OIDC subject.",
        );
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(
                state.tick,
                Some(request.request_id.clone()),
                Some(state.cursor),
            ),
            data: AuthLinkResponse {
                request_id: request.request_id,
                provider: request.provider,
                account_id,
                character_id,
                display_name,
                session,
                linked_guest: true,
            },
        })
    }

    pub fn auth_refresh(
        &self,
        request: AuthRefreshRequest,
    ) -> Result<ApiResponse<AuthRefreshResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        validate_request_id(&request.request_id)?;
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
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(
                state.tick,
                Some(request.request_id.clone()),
                Some(state.cursor),
            ),
            data: AuthRefreshResponse {
                request_id: request.request_id,
                session: access,
            },
        })
    }

    pub fn auth_revoke(
        &self,
        token: &str,
        request: AuthRevokeRequest,
    ) -> Result<ApiResponse<AuthRevokeResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
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
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(
                state.tick,
                Some(request.request_id.clone()),
                Some(state.cursor),
            ),
            data: AuthRevokeResponse {
                request_id: request.request_id,
                revoked_sessions: tokens.len() as u32,
            },
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
        Ok(ApiResponse { meta: meta(state.tick, None, Some(state.cursor)), data: AccountResponse { account_id: identity.account_id.clone(), provider: production.map(|account| account.provider.clone()).unwrap_or_else(|| "development-guest".to_owned()), character_id: identity.character_id.clone(), display_name: identity.display_name.clone(), guest_fixture: production.is_none(), privacy_policy_version: PRIVACY_POLICY_VERSION.to_owned(), retention_note: "Account identity is retained until deletion; chat reports are retained for 90 days; settlement history is retained as public world history with account identifiers minimised.".to_owned(), session_expires_at_tick: expires, character: player_projection(identity) } })
    }

    pub fn support_repair(
        &self,
        token: &str,
        request: SupportRepairRequest,
    ) -> Result<ApiResponse<SupportRepairResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        let actor_key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
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
        let actor = state
            .identities
            .get(&actor_key)
            .expect("identity exists")
            .account_id
            .clone();
        let target_account = request.account_id.clone().unwrap_or_else(|| actor.clone());
        let target_key = state
            .identities
            .iter()
            .find(|(_, identity)| identity.account_id == target_account)
            .map(|(key, _)| key.clone());
        let (accepted, summary, reason) = match request.action {
            SupportRepairAction::ClearStuckTravel => { if let Some(target_key) = target_key { state.phase5.travel.remove(&target_key); if let Some(identity) = state.identities.get_mut(&target_key) { identity.position = Position { x: 8, y: 6 }; } (true, "Stuck travel cleared at the origin with cargo and rewards preserved.".to_owned(), None) } else { (false, String::new(), Some("The target account is not present.".to_owned())) } }
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
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }

    pub fn moderation_report(
        &self,
        token: &str,
        request: ModerationReportRequest,
    ) -> Result<ApiResponse<ModerationReportResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        if request.category.trim().is_empty() || request.note.trim().is_empty() {
            return Err(RepositoryError::new(
                400,
                "invalid_report",
                "A moderation category and note are required.",
            ));
        }
        let report_id = format!("report-{}", state.phase6.next_audit_id);
        state.phase6.next_audit_id = state.phase6.next_audit_id.saturating_add(1);
        let response = ModerationReportResponse {
            request_id: request.request_id,
            accepted: true,
            report_id: report_id.clone(),
            status: "queued".to_owned(),
            reason: None,
        };
        state
            .phase6
            .reports
            .insert(report_id.clone(), response.clone());
        let actor = state
            .identities
            .get(&key)
            .expect("identity exists")
            .account_id
            .clone();
        audit(
            &mut state,
            &actor,
            "moderation.report",
            request.target_account_id.as_deref().unwrap_or("message"),
            "accepted",
            &request.note,
        );
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: response,
        })
    }

    pub fn ops_health(&self) -> ApiResponse<OpsHealthResponse> {
        let state = self.state.lock().expect("world repository lock poisoned");
        ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: OpsHealthResponse {
                status: if integrity_ok(&state) {
                    "ok".to_owned()
                } else {
                    "degraded".to_owned()
                },
                ready: integrity_ok(&state),
                storage_version: super::STORAGE_VERSION,
                protocol_version: PROTOCOL_VERSION.to_owned(),
                last_backup_tick: state.phase6.last_backup_tick,
                last_backup_path: state.phase6.last_backup_path.clone(),
                integrity_ok: integrity_ok(&state),
                maintenance_message: self.config.maintenance_message.clone(),
            },
        }
    }

    pub fn ops_metrics(
        &self,
        token: &str,
    ) -> Result<ApiResponse<OpsMetricsResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: OpsMetricsResponse {
                server_tick: state.tick,
                connected_sessions: state.sessions.len() as u32,
                accounts: state.identities.len() as u32,
                region_entities_visible: (state.phase5.locations.len()
                    + state.phase5.routes.len()
                    + state.phase5.settlements.len())
                    as u32,
                event_cursor: state.cursor,
                regional_event_backlog: state
                    .phase5
                    .events
                    .iter()
                    .filter(|event| {
                        !matches!(
                            event.stage,
                            tarrowyn_protocol::RegionalEventStage::Aftermath
                        )
                    })
                    .count() as u32,
                open_market_orders: state
                    .phase5
                    .market_orders
                    .iter()
                    .filter(|order| order.status == tarrowyn_protocol::MarketOrderStatus::Open)
                    .count() as u32,
                travelling_players: state
                    .phase5
                    .travel
                    .values()
                    .filter(|travel| travel.status == tarrowyn_protocol::TravelStatus::Travelling)
                    .count() as u32,
                rejected_commands: state.phase6.rejected_commands,
                completed_commands: state.phase6.completed_commands,
                average_tick_ms: self.config.tick_interval.as_millis() as u32,
                alert_flags: alert_flags(&state),
            },
        })
    }

    pub fn chronicle_search(
        &self,
        token: &str,
        query: &str,
        since: u64,
    ) -> Result<ApiResponse<ChronicleSearchResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        authenticate(&mut state, token, &self.config)?;
        let query = query.trim().chars().take(80).collect::<String>();
        let needle = query.to_lowercase();
        let entries: Vec<_> = state
            .phase3
            .chronicle
            .iter()
            .filter(|entry| {
                entry.cursor > since
                    && (needle.is_empty()
                        || format!("{} {} {}", entry.title, entry.text, entry.kind)
                            .to_lowercase()
                            .contains(&needle))
            })
            .cloned()
            .collect();
        let next_cursor = entries.last().map(|entry| entry.cursor);
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: ChronicleSearchResponse {
                query,
                entries,
                next_cursor,
                cursor: state.cursor,
            },
        })
    }
}

pub(super) fn phase6_tick(state: &mut RepositoryState, config: &ServerConfig) {
    if config.backup_interval_ticks > 0 && state.tick.is_multiple_of(config.backup_interval_ticks) {
        write_backup(state, config);
    }
    state.phase6.audits.truncate(512);
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
            movement_results: HashMap::new(),
            chat_results: HashMap::new(),
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

fn integrity_ok(state: &RepositoryState) -> bool {
    let unique_characters = state
        .identities
        .values()
        .map(|identity| identity.character_id.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len()
        == state.identities.len();
    unique_characters
        && state
            .phase5
            .routes
            .iter()
            .all(|route| route.condition <= 100)
        && state
            .phase5
            .settlements
            .iter()
            .all(|settlement| settlement.food <= 100 && settlement.safety <= 100)
}

fn alert_flags(state: &RepositoryState) -> Vec<String> {
    let mut flags = Vec::new();
    if !integrity_ok(state) {
        flags.push("integrity_check_failed".to_owned());
    }
    if state
        .phase5
        .market_orders
        .iter()
        .filter(|order| order.status == tarrowyn_protocol::MarketOrderStatus::Open)
        .count()
        > 32
    {
        flags.push("market_backlog".to_owned());
    }
    if state
        .phase5
        .travel
        .values()
        .filter(|travel| travel.status == tarrowyn_protocol::TravelStatus::Interrupted)
        .count()
        > 4
    {
        flags.push("travel_recovery_backlog".to_owned());
    }
    flags
}

fn write_backup(state: &mut RepositoryState, config: &ServerConfig) {
    let Some(path) = config
        .backup_path
        .as_deref()
        .or(config.persistence_path.as_deref())
    else {
        return;
    };
    let backup_path = if config.backup_path.is_some() {
        path.to_owned()
    } else {
        format!("{path}.backup")
    };
    let Ok(data) = serde_json::to_vec_pretty(&state.to_stored()) else {
        return;
    };
    if fs::write(&backup_path, data).is_ok() {
        state.phase6.last_backup_tick = Some(state.tick);
        state.phase6.last_backup_path = Some(backup_path);
    }
}
