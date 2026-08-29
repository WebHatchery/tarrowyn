//! Focused Phase 5 tick and commodity helpers.

use super::super::models::RepositoryState;
use super::super::*;
use super::*;
use tarrowyn_protocol::{CommodityKind, MarketOrderStatus, RegionalEventStage, TravelStatus};

pub(super) fn cache_key(account: &str, request_id: &str) -> String {
    format!("phase5:{account}:{request_id}")
}

pub(super) fn player_location(state: &RepositoryState, key: &str) -> String {
    let position = state.identities.get(key).expect("identity exists").position;
    state
        .phase5
        .locations
        .iter()
        .min_by_key(|location| location.position.manhattan_distance(position))
        .map(|location| location.location_id.clone())
        .unwrap_or_else(|| "hearth".to_owned())
}

pub(super) fn location_position(state: &RepositoryState, id: &str) -> Position {
    state
        .phase5
        .locations
        .iter()
        .find(|location| location.location_id == id)
        .map(|location| location.position)
        .unwrap_or_else(|| crate::content::region_location_profile("hearth").position)
}

pub(super) fn travel_response(
    state: &mut RepositoryState,
    key: &str,
    request: TravelRequest,
    travel: Option<TravelState>,
    accepted: bool,
    reason: Option<String>,
) -> Result<ApiResponse<TravelResponse>, RepositoryError> {
    let response = TravelResponse {
        request_id: request.request_id.clone(),
        accepted,
        travel,
        location_id: player_location(state, key),
        reason,
    };
    let cache = cache_key(key, &request.request_id);
    state
        .phase5
        .request_results
        .insert(cache, Phase5Response::Travel(response.clone()));
    Ok(ApiResponse {
        meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
        data: response,
    })
}

pub(super) fn seed_event(
    state: &mut RepositoryState,
) -> (bool, Option<RegionalEvent>, Option<String>) {
    if state
        .phase5
        .events
        .iter()
        .any(|event| !matches!(event.stage, RegionalEventStage::Aftermath))
    {
        return (
            false,
            None,
            Some("A regional event is already being resolved.".to_owned()),
        );
    }
    let template =
        crate::content::regional_event_template(state.phase5.next_event_id.saturating_sub(1));
    let event = RegionalEvent {
        event_id: format!("regional-event-{}", state.phase5.next_event_id),
        title: template.title,
        kind: template.kind,
        stage: RegionalEventStage::Signal,
        affected_location_ids: template.affected_locations,
        effects: template.effects,
        cause: template.cause,
        intervention_options: template.intervention_options,
        chosen_intervention: None,
        outcome: None,
        started_tick: state.tick,
        updated_tick: state.tick,
        cursor: 0,
    };
    state.phase5.next_event_id = state.phase5.next_event_id.saturating_add(1);
    state.phase5.events.push(event.clone());
    trim_event_history(&mut state.phase5);
    let affected_locations = event_location_refs(&event.affected_location_ids);
    record_regional(
        state,
        &affected_locations,
        "regional event signal",
        &event.cause,
    );
    let mut event = event;
    event.cursor = state.cursor;
    if let Some(stored) = state.phase5.events.last_mut() {
        stored.cursor = state.cursor;
    }
    (true, Some(event), None)
}

pub(super) fn intervene_event(
    state: &mut RepositoryState,
    event_id: Option<&str>,
    intervention: Option<&str>,
) -> (bool, Option<RegionalEvent>, Option<String>) {
    let Some(index) = state
        .phase5
        .events
        .iter()
        .position(|event| Some(event.event_id.as_str()) == event_id)
    else {
        return (
            false,
            None,
            Some("That regional event is not recorded.".to_owned()),
        );
    };
    let Some(intervention) = intervention.filter(|value| !value.is_empty()) else {
        return (
            false,
            None,
            Some("Name the visible intervention to commit.".to_owned()),
        );
    };
    if !state.phase5.events[index]
        .intervention_options
        .iter()
        .any(|option| option == intervention)
    {
        return (
            false,
            Some(state.phase5.events[index].clone()),
            Some("Choose one of the visible intervention options.".to_owned()),
        );
    }
    if !matches!(
        state.phase5.events[index].stage,
        RegionalEventStage::Signal | RegionalEventStage::Escalation
    ) {
        return (
            false,
            Some(state.phase5.events[index].clone()),
            Some("That event has already moved beyond intervention.".to_owned()),
        );
    }
    state.phase5.events[index].stage = RegionalEventStage::Intervention;
    state.phase5.events[index].chosen_intervention = Some(intervention.to_owned());
    state.phase5.events[index].updated_tick = state.tick;
    let affected_location_ids = state.phase5.events[index].affected_location_ids.clone();
    let consequence = apply_event_intervention(state, intervention, &affected_location_ids);
    let affected_locations = event_location_refs(&affected_location_ids);
    record_regional(
        state,
        &affected_locations,
        "regional intervention",
        &format!("Players chose {intervention}: {consequence}"),
    );
    state.phase5.events[index].cursor = state.cursor;
    (true, Some(state.phase5.events[index].clone()), None)
}

fn apply_event_intervention(
    state: &mut RepositoryState,
    intervention: &str,
    affected_location_ids: &[String],
) -> &'static str {
    match intervention {
        "repair ferry markers" => {
            if let Some(route) = state.phase5.routes.iter_mut().find(|route| {
                route.route_id == "saltmere-ferry"
                    && event_affects_route(
                        affected_location_ids,
                        &route.origin_location_id,
                        &route.destination_location_id,
                    )
            }) {
                route.condition = route.condition.saturating_add(12).min(100);
                route.risk_percent = route.risk_percent.saturating_sub(6);
                route.status = RouteStatus::Operational;
            }
            "the ferry route is marked safe again"
        }
        "escort the grain caravan" => {
            if let Some(route) = state.phase5.routes.iter_mut().find(|route| {
                route.route_id == "north-pack-road"
                    && event_affects_route(
                        affected_location_ids,
                        &route.origin_location_id,
                        &route.destination_location_id,
                    )
            }) {
                route.condition = route.condition.saturating_add(8).min(100);
                route.risk_percent = route.risk_percent.saturating_sub(8);
                route.status = RouteStatus::Delayed;
            }
            for settlement in &mut state.phase5.settlements {
                if event_affects_location(affected_location_ids, &settlement.location_id) {
                    settlement.food = settlement.food.saturating_add(6).min(100);
                    settlement.price_index_percent =
                        settlement.price_index_percent.saturating_sub(3);
                }
            }
            "the escorted grain reaches each settlement under watch"
        }
        "open the frontier storehouse" => {
            if event_affects_location(affected_location_ids, "whisperwood-outpost") {
                let stock = state
                    .phase5
                    .stock
                    .entry(stock_key("whisperwood-outpost", "seeds"))
                    .or_default();
                *stock = stock.saturating_add(4);
                if let Some(settlement) = state
                    .phase5
                    .settlements
                    .iter_mut()
                    .find(|settlement| settlement.location_id == "whisperwood-outpost")
                {
                    settlement.food = settlement.food.saturating_add(4).min(100);
                    settlement.price_index_percent =
                        settlement.price_index_percent.saturating_sub(2);
                }
            }
            "frontier reserves open and seed supply reaches the watch"
        }
        _ => "the chosen response steadies the region's supply line",
    }
}

pub(super) fn resolve_event(
    state: &mut RepositoryState,
    event_id: Option<&str>,
) -> (bool, Option<RegionalEvent>, Option<String>) {
    let Some(index) = state
        .phase5
        .events
        .iter()
        .position(|event| Some(event.event_id.as_str()) == event_id)
    else {
        return (
            false,
            None,
            Some("That regional event is not recorded.".to_owned()),
        );
    };
    if !matches!(
        state.phase5.events[index].stage,
        RegionalEventStage::Intervention
    ) {
        return (
            false,
            Some(state.phase5.events[index].clone()),
            Some("The event needs an intervention before it can resolve.".to_owned()),
        );
    }
    state.phase5.events[index].stage = RegionalEventStage::Resolution;
    complete_event_resolution(state, index);
    (true, Some(state.phase5.events[index].clone()), None)
}

fn complete_event_resolution(state: &mut RepositoryState, index: usize) {
    let chosen = state.phase5.events[index].chosen_intervention.as_deref();
    state.phase5.events[index].outcome = Some(event_resolution_outcome(chosen));
    state.phase5.events[index].updated_tick = state.tick;
    let outcome = state.phase5.events[index]
        .outcome
        .clone()
        .unwrap_or_else(|| "The event resolved.".to_owned());
    let affected_location_ids = state.phase5.events[index].affected_location_ids.clone();
    for settlement in &mut state.phase5.settlements {
        if event_affects_location(&affected_location_ids, &settlement.location_id) {
            settlement.safety = settlement.safety.saturating_add(4).min(100);
            settlement.price_index_percent = settlement.price_index_percent.saturating_sub(5);
        }
    }
    let affected_locations = event_location_refs(&affected_location_ids);
    record_regional(
        state,
        &affected_locations,
        "regional event resolution",
        &outcome,
    );
    state.phase5.events[index].cursor = state.cursor;
}

fn event_resolution_outcome(chosen: Option<&str>) -> String {
    match chosen {
        Some("repair ferry markers") => {
            "The repaired ferry route keeps the cost of the thaw from becoming a regional collapse."
                .to_owned()
        }
        Some("escort the grain caravan") => {
            "The escorted grain caravan restores food movement while the roads remain watched."
                .to_owned()
        }
        Some("open the frontier storehouse") => {
            "The frontier storehouse reserve keeps seed supply moving through the thaw.".to_owned()
        }
        Some(intervention) => {
            format!("The {intervention} response steadies the region after the thaw.")
        }
        None => "The region records a response that steadies the thaw's supply line.".to_owned(),
    }
}

pub(super) fn record_regional(
    state: &mut RepositoryState,
    locations: &[&str],
    kind: &str,
    text: &str,
) {
    super::super::phase3::record(state, kind, "The regional ledger remembers the road", text);
    let entry = state.phase3.chronicle.back().cloned();
    if let Some(entry) = entry {
        for settlement in &mut state.phase5.settlements {
            if locations.contains(&settlement.location_id.as_str()) {
                settlement.chronicle.push(entry.clone());
            }
        }
        super::state::trim_settlement_chronicles(&mut state.phase5);
    }
    state.phase5.cursor = state.cursor;
}

fn event_location_refs(locations: &[String]) -> Vec<&str> {
    locations.iter().map(String::as_str).collect()
}

pub(super) fn advance_travel(state: &mut RepositoryState) {
    let keys: Vec<String> = state.phase5.travel.keys().cloned().collect();
    let mut arrivals = Vec::new();
    for key in keys {
        let Some(travel) = state.phase5.travel.get_mut(&key) else {
            continue;
        };
        if travel.status != TravelStatus::Travelling {
            continue;
        }
        let total = travel.eta_tick.saturating_sub(travel.departure_tick).max(1);
        let elapsed = state.tick.saturating_sub(travel.departure_tick).min(total);
        travel.progress = ((u128::from(elapsed) * 100) / u128::from(total)).min(100) as u8;
        if state.tick >= travel.eta_tick {
            travel.status = TravelStatus::Arrived;
            travel.progress = 100;
            arrivals.push((key, travel.destination_location_id.clone()));
        }
    }
    for (key, destination) in arrivals {
        let position = location_position(state, &destination);
        if let Some(identity) = state.identities.get_mut(&key) {
            identity.position = position;
        }
        record_regional(
            state,
            &[destination.as_str()],
            "travel arrival",
            "A server-owned journey arrived once and left its route history behind.",
        );
    }
}

pub(super) fn advance_events(state: &mut RepositoryState) {
    let mut transitions = Vec::new();
    for event in &mut state.phase5.events {
        let age = state.tick.saturating_sub(event.started_tick);
        let old = event.stage;
        event.stage = match (event.stage, age) {
            (RegionalEventStage::Signal, 0..=1) => RegionalEventStage::Signal,
            (RegionalEventStage::Signal, _) => RegionalEventStage::Escalation,
            (RegionalEventStage::Intervention, _) if age > 4 => RegionalEventStage::Resolution,
            (RegionalEventStage::Resolution, _) if age > 6 => RegionalEventStage::Aftermath,
            (stage, _) => stage,
        };
        if event.stage != old {
            event.updated_tick = state.tick;
            transitions.push((
                event.event_id.clone(),
                event.stage,
                event.affected_location_ids.clone(),
            ));
        }
    }
    for (event_id, stage, affected_location_ids) in transitions {
        let affected_locations = event_location_refs(&affected_location_ids);
        match stage {
            RegionalEventStage::Resolution => {
                if let Some(index) = state
                    .phase5
                    .events
                    .iter()
                    .position(|event| event.event_id == event_id)
                {
                    complete_event_resolution(state, index);
                }
            }
            RegionalEventStage::Escalation => {
                if let Some(route) = state.phase5.routes.iter_mut().find(|route| {
                    route.route_id == "north-pack-road"
                        && event_affects_route(
                            &affected_location_ids,
                            &route.origin_location_id,
                            &route.destination_location_id,
                        )
                }) {
                    route.risk_percent = route.risk_percent.saturating_add(10).min(90);
                    route.status = tarrowyn_protocol::RouteStatus::Threatened;
                }
                for settlement in &mut state.phase5.settlements {
                    if event_affects_location(&affected_location_ids, &settlement.location_id) {
                        settlement.food = settlement.food.saturating_sub(4);
                        settlement.price_index_percent =
                            settlement.price_index_percent.saturating_add(8);
                    }
                }
                if event_affects_location(&affected_location_ids, "hearth") {
                    for household in &mut state.phase4.households {
                        household.service_quality = household.service_quality.saturating_sub(4);
                        household.clue = "The thaw reduced service until a safe route and supply chain are restored."
                            .to_owned();
                    }
                }
                record_regional(state, &affected_locations, "regional event escalation", &format!("Event {event_id} crossed the region: travel risk, farm supply, prices, and household choices now carry its cause."));
            }
            RegionalEventStage::Aftermath => {
                record_regional(
                    state,
                    &affected_locations,
                    "regional event aftermath",
                    &format!(
                        "Event {event_id} settled into regional history after its resolution."
                    ),
                );
            }
            _ => continue,
        }
        if let Some(event) = state
            .phase5
            .events
            .iter_mut()
            .find(|event| event.event_id == event_id)
        {
            event.cursor = state.cursor;
        }
    }
}

fn event_affects_location(affected_location_ids: &[String], location_id: &str) -> bool {
    affected_location_ids
        .iter()
        .any(|affected| affected == location_id)
}

fn event_affects_route(
    affected_location_ids: &[String],
    origin_location_id: &str,
    destination_location_id: &str,
) -> bool {
    event_affects_location(affected_location_ids, origin_location_id)
        || event_affects_location(affected_location_ids, destination_location_id)
}

pub(super) fn expire_market_orders(state: &mut RepositoryState) {
    let mut failed = Vec::new();
    for order in &mut state.phase5.market_orders {
        if order.status == MarketOrderStatus::Open
            && state.tick.saturating_sub(order.created_tick) > 48
        {
            order.status = MarketOrderStatus::Failed;
            failed.push((
                order.order_id.clone(),
                order.origin_location_id.clone(),
                order.destination_location_id.clone(),
            ));
        }
    }
    for (order_id, origin, destination) in failed {
        record_regional(
            state,
            &[origin.as_str(), destination.as_str()],
            "market fulfilment failed",
            &format!("Order {order_id} expired without hiding its stock or price telemetry."),
        );
    }
}

pub(super) fn take_commodity(
    state: &mut RepositoryState,
    key: &str,
    location: &str,
    commodity: CommodityKind,
    quantity: u32,
) -> bool {
    if matches!(
        commodity,
        CommodityKind::Timber | CommodityKind::Stone | CommodityKind::Bandages
    ) {
        let stock = state
            .phase5
            .stock
            .entry(stock_key(location, commodity.label()))
            .or_default();
        if *stock < quantity {
            return false;
        }
        *stock -= quantity;
        return true;
    }
    let identity = state.identities.get_mut(key).expect("identity exists");
    let available = match commodity {
        CommodityKind::Wheat => identity.inventory.wheat,
        CommodityKind::Turnips => identity.inventory.turnips,
        CommodityKind::Moonberries => identity.inventory.moonberries,
        CommodityKind::Seeds => identity.inventory.seeds,
        _ => 0,
    };
    if available < quantity {
        return false;
    }
    match commodity {
        CommodityKind::Wheat => identity.inventory.wheat -= quantity,
        CommodityKind::Turnips => identity.inventory.turnips -= quantity,
        CommodityKind::Moonberries => identity.inventory.moonberries -= quantity,
        CommodityKind::Seeds => identity.inventory.seeds -= quantity,
        _ => {}
    }
    true
}

pub(super) fn give_commodity(
    state: &mut RepositoryState,
    key: &str,
    location: &str,
    commodity: CommodityKind,
    quantity: u32,
) {
    if matches!(
        commodity,
        CommodityKind::Timber | CommodityKind::Stone | CommodityKind::Bandages
    ) {
        let stock = state
            .phase5
            .stock
            .entry(stock_key(location, commodity.label()))
            .or_default();
        *stock = stock.saturating_add(quantity);
        return;
    }
    let Some(identity) = state.identities.get_mut(key) else {
        return;
    };
    match commodity {
        CommodityKind::Wheat => {
            identity.inventory.wheat = identity.inventory.wheat.saturating_add(quantity)
        }
        CommodityKind::Turnips => {
            identity.inventory.turnips = identity.inventory.turnips.saturating_add(quantity)
        }
        CommodityKind::Moonberries => {
            identity.inventory.moonberries = identity.inventory.moonberries.saturating_add(quantity)
        }
        CommodityKind::Seeds => {
            identity.inventory.seeds = identity.inventory.seeds.saturating_add(quantity)
        }
        _ => {}
    }
}

pub(super) fn stock_key(location: &str, commodity: &str) -> String {
    format!("{location}:{commodity}")
}
pub(super) fn base_price(commodity: CommodityKind) -> u32 {
    crate::content::item_base_price(commodity.label())
}
pub(super) fn stock_notes(state: &RepositoryState, location: &str) -> Vec<String> {
    ["timber", "stone", "bandages", "seeds"]
        .into_iter()
        .map(|commodity| {
            format!(
                "{commodity}: {} at {location}",
                state
                    .phase5
                    .stock
                    .get(&stock_key(location, commodity))
                    .copied()
                    .unwrap_or(0)
            )
        })
        .collect()
}
pub(super) fn price_notes(state: &RepositoryState, location: &str) -> Vec<String> {
    let index = state
        .phase5
        .settlements
        .iter()
        .find(|settlement| settlement.location_id == location)
        .map(|settlement| settlement.price_index_percent)
        .unwrap_or(100);
    [
        CommodityKind::Wheat,
        CommodityKind::Seeds,
        CommodityKind::Timber,
        CommodityKind::Bandages,
    ]
    .into_iter()
    .map(|commodity| {
        format!(
            "{}: {} gold at {}% regional index",
            commodity.label(),
            indexed_price(base_price(commodity), index),
            index
        )
    })
    .collect()
}

pub(super) fn indexed_price(base: u32, index: u16) -> u32 {
    base.saturating_mul(u32::from(index)) / 100
}

pub(super) fn season(day: u32) -> String {
    crate::content::season_for_day(day)
}
