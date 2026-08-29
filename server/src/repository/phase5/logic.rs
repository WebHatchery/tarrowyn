//! Focused Phase 5 tick and commodity helpers.

use super::super::models::RepositoryState;
use super::super::*;
use super::*;
use tarrowyn_protocol::{CommodityKind, MarketOrderStatus, RegionalEventStage, TravelStatus};

pub(super) fn validate_request_id(request_id: &str) -> Result<(), RepositoryError> {
    if request_id.trim().is_empty() || request_id.len() > 64 {
        Err(RepositoryError::new(
            400,
            "invalid_request_id",
            "Regional request IDs must contain 1 to 64 characters.",
        ))
    } else {
        Ok(())
    }
}

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
        affected_location_ids: vec![
            "hearth".to_owned(),
            "whisperwood-outpost".to_owned(),
            "saltmere".to_owned(),
        ],
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
    record_regional(
        state,
        &["hearth", "whisperwood-outpost", "saltmere"],
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
    let Some(intervention) = intervention
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
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
    if intervention == "repair ferry markers" {
        if let Some(route) = state
            .phase5
            .routes
            .iter_mut()
            .find(|route| route.route_id == "saltmere-ferry")
        {
            route.condition = route.condition.saturating_add(12).min(100);
            route.risk_percent = route.risk_percent.saturating_sub(6);
            route.status = RouteStatus::Operational;
        }
    }
    record_regional(state, &["hearth", "whisperwood-outpost", "saltmere"], "regional intervention", "Players chose an intervention that changed travel, supply, prices, and household confidence.");
    state.phase5.events[index].cursor = state.cursor;
    (true, Some(state.phase5.events[index].clone()), None)
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
        RegionalEventStage::Intervention | RegionalEventStage::Escalation
    ) {
        return (
            false,
            Some(state.phase5.events[index].clone()),
            Some("The event needs an intervention before it can resolve.".to_owned()),
        );
    }
    state.phase5.events[index].stage = RegionalEventStage::Resolution;
    state.phase5.events[index].outcome = Some("The region keeps the cost of the thaw but the repaired route and open supply chain prevent a collapse.".to_owned());
    state.phase5.events[index].updated_tick = state.tick;
    let outcome = state.phase5.events[index]
        .outcome
        .clone()
        .unwrap_or_else(|| "The event resolved.".to_owned());
    for settlement in &mut state.phase5.settlements {
        settlement.safety = settlement.safety.saturating_add(4).min(100);
        settlement.price_index_percent = settlement.price_index_percent.saturating_sub(5);
    }
    record_regional(
        state,
        &["hearth", "whisperwood-outpost", "saltmere"],
        "regional event resolution",
        &outcome,
    );
    state.phase5.events[index].cursor = state.cursor;
    (true, Some(state.phase5.events[index].clone()), None)
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
    }
    state.phase5.cursor = state.cursor;
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
        travel.progress = ((elapsed * 100) / total).min(100) as u8;
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
            transitions.push((event.event_id.clone(), event.stage));
        }
    }
    for (event_id, stage) in transitions {
        if stage == RegionalEventStage::Escalation {
            if let Some(route) = state
                .phase5
                .routes
                .iter_mut()
                .find(|route| route.route_id == "north-pack-road")
            {
                route.risk_percent = route.risk_percent.saturating_add(10).min(90);
                route.status = tarrowyn_protocol::RouteStatus::Threatened;
            }
            for settlement in &mut state.phase5.settlements {
                settlement.food = settlement.food.saturating_sub(4);
                settlement.price_index_percent = settlement.price_index_percent.saturating_add(8);
            }
            for household in &mut state.phase4.households {
                household.service_quality = household.service_quality.saturating_sub(4);
                household.clue =
                    "The thaw reduced service until a safe route and supply chain are restored."
                        .to_owned();
            }
            record_regional(state, &["hearth", "whisperwood-outpost", "saltmere"], "regional event escalation", &format!("Event {event_id} crossed the region: travel risk, farm supply, prices, and household choices now carry its cause."));
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
}

pub(super) fn expire_market_orders(state: &mut RepositoryState) {
    let mut failed = Vec::new();
    for order in &mut state.phase5.market_orders {
        if order.status == MarketOrderStatus::Open
            && state.tick.saturating_sub(order.created_tick) > 48
        {
            order.status = MarketOrderStatus::Failed;
            failed.push(order.order_id.clone());
        }
    }
    for order_id in failed {
        record_regional(
            state,
            &["hearth"],
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
            base_price(commodity) * u32::from(index) / 100,
            index
        )
    })
    .collect()
}
pub(super) fn season(day: u32) -> String {
    crate::content::season_for_day(day)
}
