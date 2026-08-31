use super::super::super::models::RepositoryState;
use crate::config::ServerConfig;
use std::collections::HashSet;

pub(super) fn integrity_ok(state: &RepositoryState, config: &ServerConfig) -> bool {
    integrity_failures(state, config).is_empty()
}

pub(super) fn integrity_failures(state: &RepositoryState, config: &ServerConfig) -> Vec<String> {
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
        <= super::super::super::phase5::MAX_MARKET_ORDERS
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
    if !super::super::phase4_integrity::ok(state, config) {
        failures.push("phase4".to_owned());
    }
    if !super::super::phase4_replay_integrity::ok(state) {
        failures.push("phase4_replay".to_owned());
    }
    if !super::super::phase3_replay_integrity::ok(state) {
        failures.push("phase3_replay".to_owned());
    }
    if !super::super::core_replay_integrity::ok(state) {
        failures.push("core_replay".to_owned());
    }
    if !super::super::core_event_integrity::ok(state, config) {
        failures.push("core_event".to_owned());
    }
    if !super::super::core_session_integrity::ok(state, config) {
        failures.push("core_session".to_owned());
    }
    if !super::super::persistent_integrity::ok(state, config) {
        failures.push("persistent".to_owned());
    }
    if !super::super::production_integrity::ok(state) {
        failures.push("production".to_owned());
    }
    if !super::super::regional_integrity::ok(state, config) {
        failures.push("regional".to_owned());
    }
    if !super::super::phase5_replay_integrity::ok(state) {
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

pub(super) fn alert_flags(
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
