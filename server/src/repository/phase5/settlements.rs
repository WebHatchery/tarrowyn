//! Settlement condition, activity, and facility projections.

use super::super::models::RepositoryState;
use super::logic::{player_location, record_regional};
use std::collections::{HashMap, HashSet};
use tarrowyn_protocol::{
    ClaimLifecycleStatus, LocationRecord, MarketOrderStatus, Position, RouteRecord, RouteStatus,
    SettlementCondition,
};

pub(super) fn update_settlements(state: &mut RepositoryState) {
    let infrastructure_targets = infrastructure_targets(state);
    let safety_targets = safety_targets(&state.phase5.routes, &state.phase5.locations);
    let administration_quality = state.phase4.governance.administration_quality;
    let mut transitions = Vec::new();
    for index in 0..state.phase5.settlements.len() {
        let location_id = state.phase5.settlements[index].location_id.clone();
        let local_players = active_players_at(state, &location_id);
        let local_orders = open_orders_at(state, &location_id);
        let activity_pulse = local_players
            .saturating_mul(4)
            .saturating_add(local_orders.min(3).saturating_mul(2));
        let settlement = &mut state.phase5.settlements[index];
        let is_hearth = settlement.location_id == "hearth";
        let locally_supported = local_players > 0 || local_orders > 0;
        settlement.player_activity = settlement
            .player_activity
            .saturating_sub(1)
            .saturating_add(activity_pulse)
            .min(100);
        if is_hearth {
            settlement.food = settlement.food.saturating_add(2).min(100);
            settlement.infrastructure = settlement.infrastructure.saturating_add(1).min(100);
        } else {
            settlement.food = settlement.food.saturating_sub(1);
        }
        if settlement.safety < 35 {
            settlement.population = settlement.population.saturating_sub(1).max(4);
        }
        if settlement.food > 65 && settlement.safety > 65 {
            settlement.population = settlement.population.saturating_add(1).min(99);
        }
        let industry_target = if local_orders > 0 {
            70
        } else if local_players > 0 {
            60
        } else {
            30
        };
        settlement.industry = move_toward(settlement.industry, industry_target);
        let governance_target = if is_hearth {
            administration_quality
        } else if locally_supported {
            65
        } else {
            40
        };
        settlement.governance = move_toward(settlement.governance, governance_target);
        if let Some(target) = infrastructure_targets.get(&location_id) {
            settlement.infrastructure = move_toward(settlement.infrastructure, *target);
        }
        if let Some(target) = safety_targets.get(&location_id) {
            settlement.safety = move_toward(settlement.safety, *target);
        }
        let old = settlement.condition;
        let low_activity = settlement.player_activity < 15;
        settlement.condition = if settlement.food < 35
            || settlement.population <= 6
            || (low_activity && settlement.governance < 50)
        {
            SettlementCondition::Quiet
        } else if settlement.safety < 45
            || settlement.infrastructure < 45
            || settlement.governance < 45
            || (low_activity && settlement.industry < 45)
        {
            SettlementCondition::Strained
        } else if settlement.condition == SettlementCondition::Quiet
            && settlement.food > 50
            && settlement.safety > 50
        {
            SettlementCondition::Recovering
        } else if settlement.food > 72
            && settlement.safety > 72
            && settlement.infrastructure > 72
            && settlement.player_activity > 35
        {
            SettlementCondition::Flourishing
        } else {
            SettlementCondition::Stable
        };
        settlement.price_index_percent = (100
            + (settlement.scarce_goods.len() as u16 * 8)
            + u16::from(100 - settlement.infrastructure.min(100)))
        .min(190);
        if settlement.condition != old {
            transitions.push((settlement.location_id.clone(), old, settlement.condition));
        }
        settlement.recovery_opportunity = match settlement.condition {
            SettlementCondition::Quiet | SettlementCondition::Strained => Some(
                "Repair a route, fill a vacancy, or bring food to reopen this community."
                    .to_owned(),
            ),
            SettlementCondition::Recovering => Some(
                "The settlement is recovering; a steady supply chain can make the change durable."
                    .to_owned(),
            ),
            _ => None,
        };
    }
    for (location, old, new) in transitions {
        record_regional(
            state,
            &[location.as_str()],
            "settlement condition",
            &format!(
                "A settlement moved from {old:?} to {new:?}; its vacancies and recovery work remain visible."
            ),
        );
    }
}

fn infrastructure_targets(state: &RepositoryState) -> HashMap<String, u8> {
    let locations = state
        .phase5
        .locations
        .iter()
        .map(|location| (location.location_id.clone(), location.position))
        .collect::<Vec<_>>();
    let mut totals = HashMap::<String, (u32, u32)>::new();
    for record in &state.phase4.infrastructure {
        let Some(location_id) = nearest_location(&locations, record.position) else {
            continue;
        };
        let entry = totals.entry(location_id.to_owned()).or_default();
        entry.0 = entry.0.saturating_add(u32::from(record.condition));
        entry.1 = entry.1.saturating_add(1);
    }
    totals
        .into_iter()
        .filter_map(|(location_id, (total, count))| {
            (count > 0).then_some((location_id, (total / count) as u8))
        })
        .collect()
}

fn safety_targets(routes: &[RouteRecord], locations: &[LocationRecord]) -> HashMap<String, u8> {
    locations
        .iter()
        .map(|location| {
            let target = routes
                .iter()
                .filter(|route| {
                    route.origin_location_id == location.location_id
                        || route.destination_location_id == location.location_id
                })
                .map(|route| route_safety_target(route.status))
                .min()
                .unwrap_or(76);
            (location.location_id.clone(), target)
        })
        .collect()
}

fn route_safety_target(status: RouteStatus) -> u8 {
    match status {
        RouteStatus::Closed => 32,
        RouteStatus::Threatened => 42,
        RouteStatus::Delayed => 58,
        RouteStatus::Repairing => 64,
        RouteStatus::Operational => 76,
    }
}

fn move_toward(value: u8, target: u8) -> u8 {
    if value < target {
        value.saturating_add(1)
    } else if value > target {
        value.saturating_sub(1)
    } else {
        value
    }
}

pub(super) fn refresh_settlement_facilities(state: &mut RepositoryState) {
    let locations = state
        .phase5
        .locations
        .iter()
        .map(|location| (location.location_id.clone(), location.position))
        .collect::<Vec<_>>();
    let claims = state
        .phase4
        .claims
        .iter()
        .filter(|claim| claim.status != ClaimLifecycleStatus::Reclaimed)
        .map(|claim| claim.position)
        .collect::<Vec<_>>();
    let available_plots = state.phase4.available_plots.clone();
    let public_works = state
        .phase4
        .infrastructure
        .iter()
        .map(|record| (record.position, record.name.clone()))
        .collect::<Vec<_>>();

    for settlement in &mut state.phase5.settlements {
        let location_id = settlement.location_id.as_str();
        settlement.claim_count = claims
            .iter()
            .filter(|position| nearest_location(&locations, **position) == Some(location_id))
            .count() as u32;
        settlement.available_plot_count = available_plots
            .iter()
            .filter(|position| nearest_location(&locations, **position) == Some(location_id))
            .count() as u32;
        settlement.public_works = public_works
            .iter()
            .filter(|(position, _)| nearest_location(&locations, *position) == Some(location_id))
            .map(|(_, name)| name.clone())
            .collect();
    }
}

fn nearest_location(locations: &[(String, Position)], position: Position) -> Option<&str> {
    locations
        .iter()
        .min_by_key(|(_, candidate)| candidate.manhattan_distance(position))
        .map(|(location_id, _)| location_id.as_str())
}

fn active_players_at(state: &RepositoryState, location_id: &str) -> u8 {
    let mut identity_keys = HashSet::new();
    state
        .sessions
        .values()
        .filter(|session| identity_keys.insert(session.identity_key.as_str()))
        .filter(|session| player_location(state, &session.identity_key) == location_id)
        .count()
        .min(u8::MAX as usize) as u8
}

fn open_orders_at(state: &RepositoryState, location_id: &str) -> u8 {
    state
        .phase5
        .market_orders
        .iter()
        .filter(|order| {
            order.status == MarketOrderStatus::Open
                && (order.origin_location_id == location_id
                    || order.destination_location_id == location_id)
        })
        .count()
        .min(u8::MAX as usize) as u8
}

pub(super) fn update_households(state: &mut RepositoryState) {
    for household in &mut state.phase5.households {
        if household.status == "considering"
            && state.phase5.settlements.iter().any(|settlement| {
                settlement.location_id
                    == household
                        .destination_location_id
                        .clone()
                        .unwrap_or_default()
                    && settlement.condition != SettlementCondition::Quiet
            })
        {
            household.status = "travelling".to_owned();
            household.departure_tick = Some(state.tick);
            household
                .history
                .push("A visible service vacancy outweighed the departure cost.".to_owned());
        } else if household.status == "travelling"
            && state
                .tick
                .saturating_sub(household.departure_tick.unwrap_or(state.tick))
                >= 3
        {
            household.status = "arrived".to_owned();
            household.arrival_tick = Some(state.tick);
            household
                .history
                .push("The household arrived through a recoverable regional move.".to_owned());
        }
    }
}
