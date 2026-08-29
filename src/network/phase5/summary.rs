use super::Phase5Client;
use tarrowyn_protocol::MarketOrderStatus;

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
            format!(
                "{} open orders",
                market
                    .orders
                    .iter()
                    .filter(|order| order.status == MarketOrderStatus::Open)
                    .count()
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
    format!("{region}\n{settlements}\n{market} • {law}\n{account}")
}
