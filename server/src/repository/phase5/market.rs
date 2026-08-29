//! Market-order recovery owned by the regional authority.

use super::super::models::RepositoryState;
use super::state::Phase5State;
use super::*;
use tarrowyn_protocol::MarketOrderStatus;

pub(crate) const MAX_MARKET_ORDERS: usize = 128;

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
    if !market_order_room(state) {
        return (
            false,
            None,
            Some("The regional market ledger is full; settle an existing shipment before adding another.".to_owned()),
        );
    }
    if !take_commodity(state, key, origin, commodity, quantity) {
        return (
            false,
            None,
            Some(format!(
                "There is not enough {} at the origin to escrow.",
                commodity.label()
            )),
        );
    }
    let unit_price = base_price(commodity).saturating_add(u32::from(route.risk_percent / 10));
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
        fallback_used: false,
    };
    state.phase5.next_order_id = state.phase5.next_order_id.saturating_add(1);
    state.phase5.market_orders.push(order.clone());
    (true, Some(order), None)
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
            let stock = state
                .phase5
                .stock
                .entry(stock_key(
                    &order.origin_location_id,
                    order.commodity.label(),
                ))
                .or_default();
            *stock = stock.saturating_add(order.quantity);
            state.phase5.market_orders[index].status = MarketOrderStatus::Cancelled;
            state.phase5.market_orders[index].settled_tick = Some(state.tick);
            record_regional(
                state,
                &[
                    order.origin_location_id.as_str(),
                    order.destination_location_id.as_str(),
                ],
                "market order account cleanup",
                "An account departure returned unsettled shipment escrow to regional stock.",
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
    state.phase5.market_orders[index].status = MarketOrderStatus::Cancelled;
    state.phase5.market_orders[index].settled_tick = Some(state.tick);
    record_regional(
        state,
        &[
            order.origin_location_id.as_str(),
            order.destination_location_id.as_str(),
        ],
        "market order repair",
        "Support restored failed shipment escrow and closed its authoritative order.",
    );
    (
        true,
        "The open or failed order was closed and its escrow was restored exactly once.".to_owned(),
        None,
    )
}
