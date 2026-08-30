//! Market-order recovery owned by the regional authority.

use super::super::models::RepositoryState;
use super::state::Phase5State;
use super::*;
use tarrowyn_protocol::{CommodityKind, MarketOrderStatus};

pub(crate) const MAX_MARKET_ORDERS: usize = 128;
const FALLBACK_MAX_QUANTITY: u32 = 2;
const FALLBACK_DAILY_CAPACITY: u8 = 2;
const FALLBACK_SURCHARGE: u32 = 5;
pub(super) const FALLBACK_DELAY_TICKS: u64 = 2;

pub(crate) fn trim_market_orders(phase: &mut Phase5State) {
    while phase.market_orders.len() > MAX_MARKET_ORDERS {
        let Some(index) = phase.market_orders.iter().position(|order| {
            matches!(
                order.status,
                MarketOrderStatus::Fulfilled | MarketOrderStatus::Cancelled
            )
        }) else {
            break;
        };
        phase.market_orders.remove(index);
    }
}

pub(crate) fn market_order_room(state: &mut RepositoryState) -> bool {
    trim_market_orders(&mut state.phase5);
    if state.phase5.market_orders.len() < MAX_MARKET_ORDERS {
        return true;
    }
    let Some(index) = state.phase5.market_orders.iter().position(|order| {
        matches!(
            order.status,
            MarketOrderStatus::Fulfilled | MarketOrderStatus::Cancelled
        )
    }) else {
        return false;
    };
    state.phase5.market_orders.remove(index);
    true
}

pub(crate) fn create_order(
    state: &mut RepositoryState,
    key: &str,
    origin: &str,
    request: &MarketOrderRequest,
) -> (bool, Option<MarketOrder>, Option<String>) {
    let Some(destination) = request.destination_location_id.as_deref() else {
        return (
            false,
            None,
            Some("Choose a destination settlement.".to_owned()),
        );
    };
    let Some(commodity) = request.commodity else {
        return (false, None, Some("Choose the good to move.".to_owned()));
    };
    let quantity = request.quantity.unwrap_or(0);
    if quantity == 0 || quantity > 99 {
        return (
            false,
            None,
            Some("Orders hold between 1 and 99 goods.".to_owned()),
        );
    }
    let Some(route) = state
        .phase5
        .routes
        .iter()
        .find(|route| {
            route.origin_location_id == origin
                && route.destination_location_id == destination
                && route.status != RouteStatus::Closed
        })
        .cloned()
    else {
        return (
            false,
            None,
            Some("No open route carries that good from here.".to_owned()),
        );
    };
    let player_supply = commodity_available(state, key, origin, commodity, quantity);
    let fallback_supply = !player_supply
        && fallback_eligible(commodity)
        && quantity <= FALLBACK_MAX_QUANTITY
        && fallback_available(state);
    if !player_supply && !fallback_supply {
        return (
            false,
            None,
            Some(format!(
                "There is not enough {} at the origin for player escrow or the limited travelling fallback.",
                commodity.label()
            )),
        );
    };
    if !market_order_room(state) {
        return (
            false,
            None,
            Some("The regional market ledger is full; settle an existing shipment before adding another.".to_owned()),
        );
    }
    let fallback_used = if player_supply {
        debug_assert!(take_commodity(state, key, origin, commodity, quantity));
        false
    } else {
        debug_assert!(reserve_fallback(state));
        true
    };
    let unit_price = base_price(commodity)
        .saturating_add(u32::from(route.risk_percent / 10))
        .saturating_add(if fallback_used { FALLBACK_SURCHARGE } else { 0 });
    let identity = state.identities.get(key).expect("identity exists");
    let order = MarketOrder {
        order_id: format!("market-order-{}", state.phase5.next_order_id),
        owner_account_id: identity.account_id.clone(),
        owner_name: identity.display_name.clone(),
        origin_location_id: origin.to_owned(),
        destination_location_id: destination.to_owned(),
        commodity,
        quantity,
        unit_price,
        total_price: unit_price.saturating_mul(quantity),
        status: MarketOrderStatus::Open,
        created_tick: state.tick,
        settled_tick: None,
        route_id: route.route_id,
        fallback_used,
    };
    state.phase5.next_order_id = state.phase5.next_order_id.saturating_add(1);
    state.phase5.market_orders.push(order.clone());
    (true, Some(order), None)
}

pub(crate) fn fulfil_order(
    state: &mut RepositoryState,
    key: &str,
    order_id: Option<&str>,
) -> (bool, Option<MarketOrder>, Option<String>) {
    let Some(index) = state
        .phase5
        .market_orders
        .iter()
        .position(|order| Some(order.order_id.as_str()) == order_id)
    else {
        return (
            false,
            None,
            Some("That market order is not recorded.".to_owned()),
        );
    };
    let location = player_location(state, key);
    let order = state.phase5.market_orders[index].clone();
    if order.status != MarketOrderStatus::Open {
        return (
            false,
            Some(order),
            Some("That order has already been settled or closed.".to_owned()),
        );
    }
    if order.fallback_used && state.tick < order.created_tick.saturating_add(FALLBACK_DELAY_TICKS) {
        return (
            false,
            Some(order),
            Some("The travelling fallback needs more time before it can arrive.".to_owned()),
        );
    }
    if order.destination_location_id != location {
        return (
            false,
            Some(order),
            Some("Arrive at the order destination before settling the shipment.".to_owned()),
        );
    }
    let fulfiller_account = state
        .identities
        .get(key)
        .expect("identity exists")
        .account_id
        .clone();
    if order.owner_account_id == fulfiller_account {
        return (
            false,
            Some(order),
            Some("The order owner cannot fulfil their own shipment.".to_owned()),
        );
    }
    let owner_key = state
        .identities
        .iter()
        .find(|(_, identity)| identity.account_id == order.owner_account_id)
        .map(|(key, _)| key.clone());
    if let Some(owner_key) = owner_key {
        give_commodity(
            state,
            &owner_key,
            &order.destination_location_id,
            order.commodity,
            order.quantity,
        );
    }
    if let Some(identity) = state.identities.get_mut(key) {
        identity.gold = identity.gold.saturating_add(order.total_price);
        identity.reputation = identity.reputation.saturating_add(1);
    }
    state.phase5.market_orders[index].status = MarketOrderStatus::Fulfilled;
    state.phase5.market_orders[index].settled_tick = Some(state.tick);
    (true, Some(state.phase5.market_orders[index].clone()), None)
}

pub(crate) fn cancel_order(
    state: &mut RepositoryState,
    key: &str,
    order_id: Option<&str>,
) -> (bool, Option<MarketOrder>, Option<String>) {
    let Some(index) = state
        .phase5
        .market_orders
        .iter()
        .position(|order| Some(order.order_id.as_str()) == order_id)
    else {
        return (
            false,
            None,
            Some("That market order is not recorded.".to_owned()),
        );
    };
    let account = state
        .identities
        .get(key)
        .expect("identity exists")
        .account_id
        .clone();
    let order = state.phase5.market_orders[index].clone();
    if order.owner_account_id != account {
        return (
            false,
            Some(order.clone()),
            Some("Only the order owner can cancel an escrow.".to_owned()),
        );
    }
    if order.status != MarketOrderStatus::Open {
        return (
            false,
            Some(order.clone()),
            Some("Only an open order can be cancelled.".to_owned()),
        );
    }
    if !order.fallback_used {
        give_commodity(
            state,
            key,
            &order.origin_location_id,
            order.commodity,
            order.quantity,
        );
    }
    state.phase5.market_orders[index].status = MarketOrderStatus::Cancelled;
    state.phase5.market_orders[index].settled_tick = Some(state.tick);
    (true, Some(state.phase5.market_orders[index].clone()), None)
}

fn fallback_eligible(commodity: CommodityKind) -> bool {
    matches!(
        commodity,
        CommodityKind::Wheat
            | CommodityKind::Turnips
            | CommodityKind::Moonberries
            | CommodityKind::Seeds
            | CommodityKind::Bandages
    )
}

fn reserve_fallback(state: &mut RepositoryState) -> bool {
    if state.phase5.fallback_day != state.clock.day {
        state.phase5.fallback_day = state.clock.day;
        state.phase5.fallback_orders_today = 0;
    }
    if state.phase5.fallback_orders_today >= FALLBACK_DAILY_CAPACITY {
        return false;
    }
    state.phase5.fallback_orders_today = state.phase5.fallback_orders_today.saturating_add(1);
    true
}

fn fallback_available(state: &RepositoryState) -> bool {
    let orders_today = if state.phase5.fallback_day == state.clock.day {
        state.phase5.fallback_orders_today
    } else {
        0
    };
    orders_today < FALLBACK_DAILY_CAPACITY
}

pub fn close_deleted_account_orders(state: &mut RepositoryState, account_id: &str) {
    let indexes: Vec<usize> = state
        .phase5
        .market_orders
        .iter()
        .enumerate()
        .filter(|(_, order)| order.owner_account_id == account_id)
        .map(|(index, _)| index)
        .collect();
    for index in indexes {
        let order = state.phase5.market_orders[index].clone();
        if matches!(
            order.status,
            MarketOrderStatus::Open | MarketOrderStatus::Failed
        ) {
            if !order.fallback_used {
                let stock = state
                    .phase5
                    .stock
                    .entry(stock_key(
                        &order.origin_location_id,
                        order.commodity.label(),
                    ))
                    .or_default();
                *stock = stock.saturating_add(order.quantity);
            }
            state.phase5.market_orders[index].status = MarketOrderStatus::Cancelled;
            state.phase5.market_orders[index].settled_tick = Some(state.tick);
            record_regional(
                state,
                &[
                    order.origin_location_id.as_str(),
                    order.destination_location_id.as_str(),
                ],
                "market order account cleanup",
                if order.fallback_used {
                    "An account departure closed a travelling fallback shipment without refunding unescrowed goods."
                } else {
                    "An account departure returned unsettled shipment escrow to regional stock."
                },
            );
        }
        state.phase5.market_orders[index].owner_account_id = "former-resident".to_owned();
        state.phase5.market_orders[index].owner_name = "Former resident".to_owned();
    }
}

pub fn reconcile_market_order(
    state: &mut RepositoryState,
    target_id: Option<&str>,
) -> (bool, String, Option<String>) {
    let Some(order_id) = target_id.filter(|id| !id.trim().is_empty()) else {
        return (
            false,
            String::new(),
            Some("A market order ID is required.".to_owned()),
        );
    };
    let Some(index) = state
        .phase5
        .market_orders
        .iter()
        .position(|order| order.order_id == order_id)
    else {
        return (
            false,
            String::new(),
            Some("That market order is not recorded.".to_owned()),
        );
    };
    let order = state.phase5.market_orders[index].clone();
    if !matches!(
        order.status,
        MarketOrderStatus::Open | MarketOrderStatus::Failed
    ) {
        return (
            false,
            String::new(),
            Some("Only an open or failed market order can be reconciled.".to_owned()),
        );
    }
    if !order.fallback_used {
        let owner_key = state
            .identities
            .iter()
            .find(|(_, identity)| identity.account_id == order.owner_account_id)
            .map(|(key, _)| key.clone());
        let owner_key = if matches!(
            order.commodity,
            tarrowyn_protocol::CommodityKind::Wheat
                | tarrowyn_protocol::CommodityKind::Turnips
                | tarrowyn_protocol::CommodityKind::Moonberries
                | tarrowyn_protocol::CommodityKind::Seeds
        ) {
            let Some(owner_key) = owner_key else {
                return (
                    false,
                    String::new(),
                    Some("The market order owner is not present for escrow recovery.".to_owned()),
                );
            };
            owner_key
        } else {
            owner_key.unwrap_or_default()
        };
        give_commodity(
            state,
            &owner_key,
            &order.origin_location_id,
            order.commodity,
            order.quantity,
        );
    }
    state.phase5.market_orders[index].status = MarketOrderStatus::Cancelled;
    state.phase5.market_orders[index].settled_tick = Some(state.tick);
    let (chronicle, response) = if order.fallback_used {
        (
            "Support closed a travelling fallback shipment without refunding unescrowed goods.",
            "The open or failed fallback order was closed without inventing an escrow refund."
                .to_owned(),
        )
    } else {
        (
            "Support restored failed shipment escrow and closed its authoritative order.",
            "The open or failed order was closed and its escrow was restored exactly once."
                .to_owned(),
        )
    };
    record_regional(
        state,
        &[
            order.origin_location_id.as_str(),
            order.destination_location_id.as_str(),
        ],
        "market order repair",
        chronicle,
    );
    (true, response, None)
}
