use super::Phase5Client;
use tarrowyn_protocol::{MarketOrderStatus, RouteStatus};

pub(super) fn render(client: &Phase5Client) -> String {
    let region = client
        .region
        .as_ref()
        .map(|region| {
            let travel = region
                .travel
                .as_ref()
                .map(|travel| format!("{:?} {}%", travel.status, travel.progress))
                .unwrap_or_else(|| "ready".to_owned());
            let condition = client
                .settlements
                .as_ref()
                .and_then(|settlements| {
                    settlements
                        .settlements
                        .iter()
                        .find(|settlement| settlement.location_id == region.player_location_id)
                })
                .map(|settlement| {
                    if settlement.recovery_opportunity.is_some() {
                        format!(
                            "{:?} • recovery open • {} claims • {} free plots • {} works",
                            settlement.condition,
                            settlement.claim_count,
                            settlement.available_plot_count,
                            settlement.public_works.len()
                        )
                    } else {
                        format!(
                            "{:?} • {} claims • {} free plots • {} works",
                            settlement.condition,
                            settlement.claim_count,
                            settlement.available_plot_count,
                            settlement.public_works.len()
                        )
                    }
                })
                .unwrap_or_else(|| "condition loading".to_owned());
            format!(
                "{} • {} travel • {condition}",
                region.player_location_id, travel
            )
        })
        .unwrap_or_else(|| "Regional map loading".to_owned());
    let settlements = client
        .settlements
        .as_ref()
        .map(|settlements| {
            settlements
                .settlements
                .iter()
                .map(|settlement| {
                    let openings = settlement.vacancies.len();
                    format!(
                        "{} {:?} • {} opening{}",
                        settlement.name,
                        settlement.condition,
                        openings,
                        if openings == 1 { "" } else { "s" }
                    )
                })
                .collect::<Vec<_>>()
                .join(" • ")
        })
        .unwrap_or_else(|| "Settlements loading".to_owned());
    let market = client
        .market
        .as_ref()
        .map(|market| {
            let open_orders = market
                .orders
                .iter()
                .filter(|order| order.status == MarketOrderStatus::Open)
                .count();
            format!(
                "{} open order{}",
                open_orders,
                if open_orders == 1 { "" } else { "s" }
            )
        })
        .unwrap_or_else(|| "Market loading".to_owned());
    let law = client
        .law
        .as_ref()
        .map(|law| {
            if law.pvp_enabled {
                "PvP opt-in"
            } else {
                "Protected economy"
            }
        })
        .unwrap_or("Law loading");
    let roads = client
        .region
        .as_ref()
        .map(|region| {
            let available = region
                .routes
                .iter()
                .filter(|route| route.status != RouteStatus::Closed)
                .count();
            let at_risk = region
                .routes
                .iter()
                .filter(|route| {
                    matches!(
                        route.status,
                        RouteStatus::Delayed | RouteStatus::Threatened | RouteStatus::Repairing
                    )
                })
                .count();
            format!(
                "Roads {available}/{} available • {at_risk} at risk",
                region.routes.len()
            )
        })
        .unwrap_or_else(|| "Roads loading".to_owned());
    let account = client
        .account
        .as_ref()
        .map(|account| {
            if account.guest_fixture {
                "Guest fixture • tap Account to link"
            } else {
                "Linked account • tap Logout to leave safely"
            }
        })
        .unwrap_or("Account loading");
    format!("{region}\n{settlements}\n{roads} • {market} • {law}\n{account}")
}
