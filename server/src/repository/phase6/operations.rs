use super::super::{
    authenticate, meta, models::RepositoryState, validate_bounded_text, RepositoryError,
    WorldRepository, PROTOCOL_VERSION,
};
use super::is_support_operator;
use tarrowyn_protocol::{
    AccountResponse, ApiResponse, ChronicleSearchResponse, OpsHealthResponse, OpsMetricsResponse,
    SupportAccountResponse,
};

const MAX_CHRONICLE_SEARCH_RESULTS: usize = 128;
const MAX_SUPPORT_CHRONICLE_ENTRIES: usize = 128;

mod integrity;
use integrity::{alert_flags, integrity_failures};

pub(super) fn mysql_pool_max_metric(config: &crate::config::ServerConfig) -> u32 {
    if config.db_driver.trim().eq_ignore_ascii_case("mysql") {
        u32::try_from(config.mysql_pool_max_connections).unwrap_or(u32::MAX)
    } else {
        0
    }
}

fn validate_chronicle_search_cursor(
    state: &RepositoryState,
    since: u64,
) -> Result<(), RepositoryError> {
    if since > state.cursor {
        Err(RepositoryError::new(
            409,
            "cursor_ahead",
            "The chronicle search cursor is ahead of the settlement.",
        ))
    } else {
        Ok(())
    }
}

impl WorldRepository {
    pub fn support_account(
        &self,
        token: &str,
        target_account_id: &str,
    ) -> Result<ApiResponse<SupportAccountResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let operator_key = authenticate(&mut state, token, &self.config)?;
        let operator_account = state
            .identities
            .get(&operator_key)
            .expect("identity exists")
            .account_id
            .clone();
        if !is_support_operator(&self.config, &operator_account) {
            return Err(RepositoryError::new(
                403,
                "support_operator_required",
                "A configured support operator account is required for account views.",
            ));
        }
        let target_account_id = validate_bounded_text(
            target_account_id,
            160,
            "invalid_account_id",
            "A bounded target account ID without control characters is required for support views.",
        )?;
        let Some((target_key, identity)) = state
            .identities
            .iter()
            .find(|(_, identity)| identity.account_id == target_account_id)
        else {
            return Err(RepositoryError::new(
                404,
                "account_not_found",
                "That account is not present in the authoritative world.",
            ));
        };
        let target_key = target_key.clone();
        let identity = identity.clone();
        let production = state.phase6.accounts.get(&target_account_id);
        let session_expires_at_tick = state
            .phase6
            .sessions
            .iter()
            .filter(|(token, session)| {
                state.sessions.contains_key(*token)
                    && !session.revoked
                    && session.expires_at_tick > state.tick
                    && session.account_id == target_account_id
            })
            .map(|(_, session)| session.expires_at_tick)
            .max()
            .unwrap_or(0);
        let account = AccountResponse {
            account_id: identity.account_id.clone(),
            provider: production
                .map(|account| account.provider.clone())
                .unwrap_or_else(|| "development-guest".to_owned()),
            character_id: identity.character_id.clone(),
            display_name: identity.display_name.clone(),
            guest_fixture: production.is_none(),
            privacy_policy_version: super::PRIVACY_POLICY_VERSION.to_owned(),
            retention_note: "Account identity is retained until deletion; chat reports are retained for 90 days; settlement history is retained as public world history with account identifiers minimised.".to_owned(),
            session_expires_at_tick,
            character: super::super::player_projection(&state, &target_key),
        };
        let mut trades = state
            .trades
            .values()
            .filter(|trade| {
                trade.creator_account_id == target_account_id
                    || trade.recipient_account_id == target_account_id
            })
            .cloned()
            .collect::<Vec<_>>();
        trades.sort_by_key(|trade| std::cmp::Reverse(trade.created_tick));
        let chronicle_skip = state
            .phase3
            .chronicle_archive
            .len()
            .saturating_add(state.phase3.chronicle.len())
            .saturating_sub(MAX_SUPPORT_CHRONICLE_ENTRIES);
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: SupportAccountResponse {
                account,
                claims: state
                    .phase4
                    .claims
                    .iter()
                    .filter(|claim| claim.owner_account_id.as_deref() == Some(&target_account_id))
                    .cloned()
                    .collect(),
                trades,
                chronicle: super::super::phase3::chronicle_entries(&state.phase3)
                    .skip(chronicle_skip)
                    .take(MAX_SUPPORT_CHRONICLE_ENTRIES)
                    .cloned()
                    .collect(),
                event_cursor: state.cursor,
            },
        })
    }

    pub fn ops_health(&self) -> ApiResponse<OpsHealthResponse> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let persistence_failed = *self
            .persistence_failed
            .lock()
            .expect("persistence status lock poisoned");
        let backup_failed = *self
            .backup_failed
            .lock()
            .expect("backup status lock poisoned");
        let integrity_failures = integrity_failures(&state, &self.config);
        let integrity_ok = integrity_failures.is_empty();
        let ready = integrity_ok && !persistence_failed && !backup_failed;
        ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: OpsHealthResponse {
                status: if ready {
                    "ok".to_owned()
                } else {
                    "degraded".to_owned()
                },
                ready,
                storage_version: super::super::STORAGE_VERSION,
                protocol_version: PROTOCOL_VERSION.to_owned(),
                last_backup_tick: state.phase6.last_backup_tick,
                // Readiness is public so clients can detect maintenance; do not
                // disclose the deployment's configured filesystem path there.
                last_backup_path: None,
                integrity_ok,
                integrity_failures,
                persistence_error: persistence_failed.then(|| {
                    "The latest authoritative persistence write failed; inspect server logs before admitting traffic."
                        .to_owned()
                }),
                backup_error: backup_failed.then(|| {
                    "The latest scheduled backup failed; inspect server logs before admitting traffic."
                        .to_owned()
                }),
                maintenance_message: self.config.maintenance_message.clone(),
            },
        }
    }

    pub fn ops_metrics(
        &self,
        token: &str,
    ) -> Result<ApiResponse<OpsMetricsResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        let account = state
            .identities
            .get(&key)
            .expect("identity exists")
            .account_id
            .clone();
        if !is_support_operator(&self.config, &account) {
            return Err(RepositoryError::new(
                403,
                "support_operator_required",
                "A configured support operator account is required for operational metrics.",
            ));
        }
        let persistence_failed = *self
            .persistence_failed
            .lock()
            .expect("persistence status lock poisoned");
        let backup_failed = *self
            .backup_failed
            .lock()
            .expect("backup status lock poisoned");
        let telemetry = self
            .tick_telemetry
            .lock()
            .expect("tick telemetry lock poisoned");
        let settlement_count = state.phase5.settlements.len() as u32;
        let average_price_index_percent = if settlement_count == 0 {
            0
        } else {
            state
                .phase5
                .settlements
                .iter()
                .map(|settlement| u32::from(settlement.price_index_percent))
                .sum::<u32>()
                .checked_div(settlement_count)
                .unwrap_or(0)
        };
        let scarce_goods_count = state
            .phase5
            .settlements
            .iter()
            .flat_map(|settlement| settlement.scarce_goods.iter())
            .collect::<std::collections::HashSet<_>>()
            .len() as u32;
        let npc_fallback_households = state
            .phase4
            .households
            .iter()
            .filter(|household| {
                household.status != tarrowyn_protocol::HouseholdLifeStatus::Departed
            })
            .count() as u32;
        let abandoned_claims = state
            .phase4
            .claims
            .iter()
            .filter(|claim| {
                matches!(
                    claim.status,
                    tarrowyn_protocol::ClaimLifecycleStatus::Abandoned
                        | tarrowyn_protocol::ClaimLifecycleStatus::Expired
                )
            })
            .count() as u32;
        let declining_settlements = state
            .phase5
            .settlements
            .iter()
            .filter(|settlement| {
                matches!(
                    settlement.condition,
                    tarrowyn_protocol::SettlementCondition::Strained
                        | tarrowyn_protocol::SettlementCondition::Quiet
                )
            })
            .count() as u32;
        let newcomer_access = !state.phase4.available_plots.is_empty()
            || state
                .phase5
                .settlements
                .iter()
                .any(|settlement| !settlement.vacancies.is_empty());
        let open_market_fallback_orders = state
            .phase5
            .market_orders
            .iter()
            .filter(|order| {
                order.status == tarrowyn_protocol::MarketOrderStatus::Open && order.fallback_used
            })
            .count() as u32;
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
                open_market_fallback_orders,
                travelling_players: state
                    .phase5
                    .travel
                    .values()
                    .filter(|travel| travel.status == tarrowyn_protocol::TravelStatus::Travelling)
                    .count() as u32,
                rejected_commands: state.phase6.rejected_commands,
                completed_commands: state.phase6.completed_commands,
                average_tick_ms: telemetry.average_tick_ms,
                last_tick_ms: telemetry.last_tick_ms,
                tick_drift_count: telemetry.tick_drift_count,
                average_price_index_percent,
                scarce_goods_count,
                npc_fallback_households,
                abandoned_claims,
                declining_settlements,
                newcomer_access,
                alert_flags: alert_flags(
                    &state,
                    &self.config,
                    persistence_failed,
                    backup_failed,
                    telemetry.last_tick_drift,
                ),
                http_request_workers: 0,
                http_request_queue_capacity: 0,
                http_active_requests: 0,
                http_queue_depth: 0,
                http_queue_peak: 0,
                http_queue_full_events: 0,
                mysql_pool_max_connections: mysql_pool_max_metric(&self.config),
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
        self.expire_and_persist_sessions(&mut state);
        authenticate(&mut state, token, &self.config)?;
        validate_chronicle_search_cursor(&state, since)?;
        let trimmed_query = query.trim();
        if trimmed_query.chars().count() > 80 || query.chars().any(char::is_control) {
            return Err(RepositoryError::new(
                400,
                "invalid_chronicle_query",
                "A chronicle search query must be at most 80 characters and contain no control characters.",
            ));
        }
        let query = trimmed_query.to_owned();
        let needle = query.to_lowercase();
        let matches = |entry: &&tarrowyn_protocol::ChronicleEntry| {
            entry.cursor > since
                && (needle.is_empty()
                    || format!("{} {} {}", entry.title, entry.text, entry.kind)
                        .to_lowercase()
                        .contains(&needle))
        };
        let entries: Vec<_> = super::super::phase3::chronicle_entries(&state.phase3)
            .filter(matches)
            .take(MAX_CHRONICLE_SEARCH_RESULTS + 1)
            .cloned()
            .collect();
        let has_more = entries.len() > MAX_CHRONICLE_SEARCH_RESULTS;
        let mut entries = entries;
        if has_more {
            entries.truncate(MAX_CHRONICLE_SEARCH_RESULTS);
        }
        let summary = super::super::phase3::chronicle_summary(&entries, since);
        let next_cursor = has_more
            .then(|| entries.last())
            .flatten()
            .map(|entry| entry.cursor);
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: ChronicleSearchResponse {
                query,
                entries,
                summary,
                next_cursor,
                cursor: state.cursor,
            },
        })
    }
}

#[cfg(test)]
mod tests;
