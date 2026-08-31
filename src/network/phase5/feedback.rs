use super::*;

pub(super) fn phase5_notice(
    accepted: bool,
    reason: Option<String>,
    success: &str,
    notices: &mut Vec<NetworkNotice>,
) {
    if accepted {
        notices.push(NetworkNotice::Success(success.to_owned()));
    } else {
        notices.push(NetworkNotice::Warning(reason.unwrap_or_else(|| {
            "The regional action was not accepted.".to_owned()
        })));
    }
}

pub(super) fn market_success_message(
    action: Option<MarketOrderAction>,
    fallback_used: bool,
) -> &'static str {
    match action {
        Some(MarketOrderAction::Create) if fallback_used => {
            "The limited travelling service accepted the shipment at a surcharge."
        }
        Some(MarketOrderAction::Create) => "The shipment is on the regional ledger.",
        Some(MarketOrderAction::Fulfil) if fallback_used => {
            "The travelling shipment reached its destination and settled."
        }
        Some(MarketOrderAction::Fulfil) => "The shipment reached its destination and settled.",
        Some(MarketOrderAction::Cancel) if fallback_used => {
            "The fallback shipment was cancelled; no player goods were escrowed."
        }
        Some(MarketOrderAction::Cancel) => "The shipment was cancelled and its escrow returned.",
        None => "The regional market accepted the action.",
    }
}

pub(super) fn market_result_message(
    action: Option<MarketOrderAction>,
    fallback_used: bool,
    order: Option<&MarketOrder>,
) -> String {
    let message = market_success_message(action, fallback_used);
    let Some(order) = order else {
        return message.to_owned();
    };
    format!(
        "{message} Details: {} from {} to {} • {} gold.",
        market_quantity_label(order),
        order.origin_location_id,
        order.destination_location_id,
        order.total_price
    )
}

fn market_quantity_label(order: &MarketOrder) -> String {
    let unit = match (order.commodity, order.quantity) {
        (tarrowyn_protocol::CommodityKind::Turnips, 1) => "turnip",
        (tarrowyn_protocol::CommodityKind::Moonberries, 1) => "moonberry",
        (tarrowyn_protocol::CommodityKind::Seeds, 1) => "seed",
        (tarrowyn_protocol::CommodityKind::Bandages, 1) => "bandage",
        (tarrowyn_protocol::CommodityKind::Wheat, _) => "wheat",
        (tarrowyn_protocol::CommodityKind::Turnips, _) => "turnips",
        (tarrowyn_protocol::CommodityKind::Moonberries, _) => "moonberries",
        (tarrowyn_protocol::CommodityKind::Seeds, _) => "seeds",
        (tarrowyn_protocol::CommodityKind::Timber, _) => "timber",
        (tarrowyn_protocol::CommodityKind::Stone, _) => "stone",
        (tarrowyn_protocol::CommodityKind::Bandages, _) => "bandages",
    };
    format!("{} {unit}", order.quantity)
}
