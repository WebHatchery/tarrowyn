use super::super::{
    authenticate, expire_sessions, meta, models::RepositoryState, validate_bounded_text,
    RepositoryError, WorldRepository, PROTOCOL_VERSION,
};
use super::is_support_operator;
use crate::config::ServerConfig;
use std::collections::HashSet;
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
        expire_sessions(&mut state, &self.config);
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
        expire_sessions(&mut state, &self.config);
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
        super::super::validate_event_cursor(&state, since, "chronicle search")?;
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

fn integrity_ok(state: &RepositoryState, config: &ServerConfig) -> bool {
    integrity_failures(state, config).is_empty()
}

fn integrity_failures(state: &RepositoryState, config: &ServerConfig) -> Vec<String> {
    let account_ids: HashSet<&str> = state
        .identities
        .values()
        .map(|identity| identity.account_id.as_str())
        .collect();
    let location_ids: HashSet<&str> = state
        .phase5
        .locations
        .iter()
        .map(|location| location.location_id.as_str())
        .collect();
    let regional_topology_ok = unique_non_empty(
        state
            .phase5
            .locations
            .iter()
            .map(|location| location.location_id.as_str()),
    ) && unique_non_empty(
        state
            .phase5
            .routes
            .iter()
            .map(|route| route.route_id.as_str()),
    ) && unique_non_empty(
        state
            .phase5
            .settlements
            .iter()
            .map(|settlement| settlement.settlement_id.as_str()),
    ) && unique_non_empty(
        state
            .phase5
            .settlements
            .iter()
            .map(|settlement| settlement.location_id.as_str()),
    ) && state.phase5.routes.iter().all(|route| {
        location_ids.contains(route.origin_location_id.as_str())
            && location_ids.contains(route.destination_location_id.as_str())
    }) && state
        .phase5
        .settlements
        .iter()
        .all(|settlement| location_ids.contains(settlement.location_id.as_str()));
    let market_orders_ok = state.phase5.market_orders.len()
        <= super::super::phase5::MAX_MARKET_ORDERS
        && unique_non_empty(
            state
                .phase5
                .market_orders
                .iter()
                .map(|order| order.order_id.as_str()),
        )
        && state.phase5.market_orders.iter().all(|order| {
            let Some(route) = state
                .phase5
                .routes
                .iter()
                .find(|route| route.route_id == order.route_id)
            else {
                return false;
            };
            bounded_market(&order.order_id, 160)
                && bounded_market(&order.owner_account_id, 160)
                && bounded_market(&order.owner_name, 160)
                && location_ids.contains(order.origin_location_id.as_str())
                && location_ids.contains(order.destination_location_id.as_str())
                && (account_ids.contains(order.owner_account_id.as_str())
                    || order.owner_account_id == "former-resident")
                && route.origin_location_id == order.origin_location_id
                && route.destination_location_id == order.destination_location_id
                && (1..=99).contains(&order.quantity)
                && order.unit_price > 0
                && order.total_price == order.unit_price.saturating_mul(order.quantity)
                && order.created_tick <= state.tick
                && match order.status {
                    tarrowyn_protocol::MarketOrderStatus::Open
                    | tarrowyn_protocol::MarketOrderStatus::Failed => order.settled_tick.is_none(),
                    tarrowyn_protocol::MarketOrderStatus::Fulfilled
                    | tarrowyn_protocol::MarketOrderStatus::Cancelled => order
                        .settled_tick
                        .is_some_and(|tick| tick >= order.created_tick && tick <= state.tick),
                }
        });
    let travel_ids_ok = unique_non_empty(
        state
            .phase5
            .travel
            .values()
            .map(|travel| travel.travel_id.as_str()),
    ) && state.phase5.travel.iter().all(|(identity_key, travel)| {
        let Some(route) = state
            .phase5
            .routes
            .iter()
            .find(|route| route.route_id == travel.route_id)
        else {
            return false;
        };
        state.identities.contains_key(identity_key)
            && location_ids.contains(travel.origin_location_id.as_str())
            && location_ids.contains(travel.destination_location_id.as_str())
            && ((route.origin_location_id == travel.origin_location_id
                && route.destination_location_id == travel.destination_location_id)
                || (route.origin_location_id == travel.destination_location_id
                    && route.destination_location_id == travel.origin_location_id))
            && travel.progress <= 100
            && travel.risk_percent <= 100
    });
    let events_ok = unique_non_empty(
        state
            .phase5
            .events
            .iter()
            .map(|event| event.event_id.as_str()),
    ) && state.phase5.events.iter().all(|event| {
        !event.affected_location_ids.is_empty()
            && event
                .affected_location_ids
                .iter()
                .all(|location_id| location_ids.contains(location_id.as_str()))
    });
    let households_ok = unique_non_empty(
        state
            .phase5
            .households
            .iter()
            .map(|household| household.household_id.as_str()),
    ) && state.phase5.households.iter().all(|household| {
        location_ids.contains(household.origin_location_id.as_str())
            && household
                .destination_location_id
                .as_deref()
                .is_none_or(|location_id| location_ids.contains(location_id))
    });
    let item_ids = crate::content::item_ids();
    let stock_ok = !state.phase5.stock.is_empty()
        && state.phase5.stock.keys().all(|key| {
            let Some((location_id, commodity)) = key.split_once(':') else {
                return false;
            };
            location_ids.contains(location_id) && item_ids.contains(commodity)
        });
    let phase5_metadata_ok = state.phase5.next_travel_id > 0
        && state.phase5.next_order_id > 0
        && state.phase5.next_event_id > 0
        && state.phase5.fallback_day > 0
        && state.phase5.fallback_day <= state.clock.day;
    let identity_ids_ok = unique_non_empty(
        state
            .identities
            .values()
            .map(|identity| identity.account_id.as_str()),
    ) && unique_non_empty(
        state
            .identities
            .values()
            .map(|identity| identity.character_id.as_str()),
    );
    let mut failures = Vec::new();
    if !identity_ids_ok {
        failures.push("identity_ids".to_owned());
    }
    if !super::phase4_integrity::ok(state, config) {
        failures.push("phase4".to_owned());
    }
    if !super::phase4_replay_integrity::ok(state) {
        failures.push("phase4_replay".to_owned());
    }
    if !super::phase3_replay_integrity::ok(state) {
        failures.push("phase3_replay".to_owned());
    }
    if !super::core_replay_integrity::ok(state) {
        failures.push("core_replay".to_owned());
    }
    if !super::core_event_integrity::ok(state, config) {
        failures.push("core_event".to_owned());
    }
    if !super::core_session_integrity::ok(state, config) {
        failures.push("core_session".to_owned());
    }
    if !super::persistent_integrity::ok(state, config) {
        failures.push("persistent".to_owned());
    }
    if !super::production_integrity::ok(state) {
        failures.push("production".to_owned());
    }
    if !super::regional_integrity::ok(state, config) {
        failures.push("regional".to_owned());
    }
    if !super::phase5_replay_integrity::ok(state) {
        failures.push("phase5_replay".to_owned());
    }
    if state.phase5.locations.is_empty()
        || state.phase5.routes.is_empty()
        || state.phase5.settlements.is_empty()
    {
        failures.push("regional_collections".to_owned());
    }
    if !regional_topology_ok {
        failures.push("regional_topology".to_owned());
    }
    if !market_orders_ok {
        failures.push("market_orders".to_owned());
    }
    if !travel_ids_ok {
        failures.push("travel".to_owned());
    }
    if !events_ok {
        failures.push("events".to_owned());
    }
    if !households_ok {
        failures.push("households".to_owned());
    }
    if !stock_ok {
        failures.push("stock".to_owned());
    }
    if !phase5_metadata_ok {
        failures.push("phase5_metadata".to_owned());
    }
    if !state.phase5.routes.iter().all(|route| {
        route.length > 0
            && route.risk_percent <= 100
            && route.condition <= 100
            && route.capacity > 0
            && route.travel_ticks > 0
            && route.repair_cost > 0
    }) {
        failures.push("route_bounds".to_owned());
    }
    if !state.phase5.settlements.iter().all(|settlement| {
        settlement.population > 0
            && settlement.food <= 100
            && settlement.safety <= 100
            && settlement.infrastructure <= 100
            && settlement.industry <= 100
            && settlement.governance <= 100
            && settlement.player_activity <= 100
            && settlement.price_index_percent > 0
    }) {
        failures.push("settlement_bounds".to_owned());
    }
    failures
}

fn unique_non_empty<'a>(mut values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = HashSet::new();
    values.all(|value| !value.trim().is_empty() && seen.insert(value))
}

fn bounded_market(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}

fn alert_flags(
    state: &RepositoryState,
    config: &ServerConfig,
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
    if !integrity_ok(state, config) {
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
