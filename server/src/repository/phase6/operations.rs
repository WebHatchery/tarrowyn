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

impl WorldRepository {
    pub fn support_account(
        &self,
        token: &str,
        target_account_id: &str,
    ) -> Result<ApiResponse<SupportAccountResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
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
            .values()
            .filter(|session| session.account_id == target_account_id)
            .map(|session| session.expires_at_tick)
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
        let state = self.state.lock().expect("world repository lock poisoned");
        let persistence_failed = *self
            .persistence_failed
            .lock()
            .expect("persistence status lock poisoned");
        let backup_failed = *self
            .backup_failed
            .lock()
            .expect("backup status lock poisoned");
        let integrity_ok = integrity_ok(&state);
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
                last_backup_path: state.phase6.last_backup_path.clone(),
                integrity_ok,
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
                    persistence_failed,
                    backup_failed,
                    telemetry.last_tick_drift,
                ),
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
            .take(MAX_CHRONICLE_SEARCH_RESULTS)
            .cloned()
            .collect();
        let archived_matches: Vec<_> = state
            .phase3
            .chronicle_archive
            .iter()
            .filter(|entry| {
                entry.cursor > since
                    && (needle.is_empty()
                        || format!("{} {} {}", entry.title, entry.text, entry.kind)
                            .to_lowercase()
                            .contains(&needle))
            })
            .take(MAX_CHRONICLE_SEARCH_RESULTS)
            .cloned()
            .collect();
        let next_cursor = entries.last().map(|entry| entry.cursor);
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: ChronicleSearchResponse {
                query,
                entries,
                summary: super::super::phase3::chronicle_summary(&archived_matches, since),
                next_cursor,
                cursor: state.cursor,
            },
        })
    }
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
        && state.phase5.routes.iter().all(|route| {
            route.length > 0
                && route.risk_percent <= 100
                && route.condition <= 100
                && route.capacity > 0
                && route.travel_ticks > 0
                && route.repair_cost > 0
        })
        && state.phase5.settlements.iter().all(|settlement| {
            settlement.population > 0
                && settlement.food <= 100
                && settlement.safety <= 100
                && settlement.infrastructure <= 100
                && settlement.industry <= 100
                && settlement.governance <= 100
                && settlement.player_activity <= 100
                && settlement.price_index_percent > 0
        })
}

fn alert_flags(
    state: &RepositoryState,
    persistence_failed: bool,
    backup_failed: bool,
    tick_drift: bool,
) -> Vec<String> {
    let mut flags = Vec::new();
    if persistence_failed {
        flags.push("persistence_write_failed".to_owned());
    }
    if backup_failed {
        flags.push("backup_write_failed".to_owned());
    }
    if tick_drift {
        flags.push("tick_drift".to_owned());
    }
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
    if state
        .phase5
        .events
        .iter()
        .filter(|event| {
            !matches!(
                event.stage,
                tarrowyn_protocol::RegionalEventStage::Aftermath
            )
        })
        .count()
        > 128
    {
        flags.push("regional_event_backlog".to_owned());
    }
    if state.phase5.market_orders.iter().any(|order| {
        order.quantity == 0
            || order.unit_price == 0
            || order.total_price != order.unit_price.saturating_mul(order.quantity)
    }) {
        flags.push("economy_anomaly".to_owned());
    }
    flags
}

#[cfg(test)]
mod tests;
