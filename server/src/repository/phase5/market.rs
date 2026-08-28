//! Market-order recovery owned by the regional authority.

use super::super::models::RepositoryState;
use super::*;
use tarrowyn_protocol::MarketOrderStatus;

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
