use super::super::super::models::RepositoryState;
use tarrowyn_protocol::CommodityKind;

pub fn take_commodity(
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

pub fn commodity_available(
    state: &RepositoryState,
    key: &str,
    location: &str,
    commodity: CommodityKind,
    quantity: u32,
) -> bool {
    let available = if matches!(
        commodity,
        CommodityKind::Timber | CommodityKind::Stone | CommodityKind::Bandages
    ) {
        state
            .phase5
            .stock
            .get(&stock_key(location, commodity.label()))
            .copied()
            .unwrap_or(0)
    } else {
        let Some(identity) = state.identities.get(key) else {
            return false;
        };
        match commodity {
            CommodityKind::Wheat => identity.inventory.wheat,
            CommodityKind::Turnips => identity.inventory.turnips,
            CommodityKind::Moonberries => identity.inventory.moonberries,
            CommodityKind::Seeds => identity.inventory.seeds,
            _ => 0,
        }
    };
    available >= quantity
}

pub fn give_commodity(
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

pub fn stock_key(location: &str, commodity: &str) -> String {
    format!("{location}:{commodity}")
}

pub fn base_price(commodity: CommodityKind) -> u32 {
    crate::content::item_base_price(commodity.label())
}

pub fn stock_notes(state: &RepositoryState, location: &str) -> Vec<String> {
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

pub fn price_notes(state: &RepositoryState, location: &str) -> Vec<String> {
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

pub fn indexed_price(base: u32, index: u16) -> u32 {
    base.saturating_mul(u32::from(index)) / 100
}

pub fn season(day: u32) -> String {
    crate::content::season_for_day(day)
}
