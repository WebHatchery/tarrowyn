use super::Phase5Client;
use tarrowyn_protocol::{MarketOrderStatus, RegionalEventStage, RouteStatus};

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
            let fallback_orders = market
                .orders
                .iter()
                .filter(|order| order.status == MarketOrderStatus::Open && order.fallback_used)
                .count();
            if fallback_orders == 0 {
                format!("Orders {open_orders}")
            } else {
                format!("Orders {open_orders} • {fallback_orders} fallback")
            }
        })
        .unwrap_or_else(|| "Market loading".to_owned());
    let law = client
        .law
        .as_ref()
        .map(|law| {
            if law.pvp_enabled {
                "PvP opt-in"
            } else {
                "Protected"
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
            format!("Roads {available}/{} • risk {at_risk}", region.routes.len())
        })
        .unwrap_or_else(|| "Roads loading".to_owned());
    let event = client
        .events
        .as_ref()
        .and_then(|events| events.events.last())
        .map(|event| match event.stage {
            RegionalEventStage::Signal => "Event signal".to_owned(),
            RegionalEventStage::Escalation => "Event escalation".to_owned(),
            RegionalEventStage::Intervention => "Event intervention".to_owned(),
            RegionalEventStage::Resolution => "Event resolution".to_owned(),
            RegionalEventStage::Aftermath => "Event aftermath".to_owned(),
        })
        .unwrap_or_else(|| "Events quiet".to_owned());
    let household = client
        .households
        .as_ref()
        .and_then(|households| households.households.first())
        .map(|household| format!("Service {}", household.status))
        .unwrap_or_else(|| "Service quiet".to_owned());
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
    format!(
        "{region}\n{settlements}\n{roads} • {market} • {law} • {event} • {household}\n{account}"
    )
}

pub(super) fn inspection(client: &Phase5Client) -> String {
    let Some(region) = client.region.as_ref() else {
        return "Regional details are still loading.".to_owned();
    };
    let routes = region
        .routes
        .iter()
        .map(|route| {
            format!(
                "{} {:?} • {}% risk • {} condition",
                route.name, route.status, route.risk_percent, route.condition
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let market = client
        .market
        .as_ref()
        .map(|market| {
            let open_orders = market
                .orders
                .iter()
                .filter(|order| order.status == MarketOrderStatus::Open)
                .count();
            let fallback_orders = market
                .orders
                .iter()
                .filter(|order| {
                    order.status == MarketOrderStatus::Open && order.fallback_used
                })
                .count();
            let stock = market
                .stock_notes
                .first()
                .map(String::as_str)
                .unwrap_or("no stock notes");
            let prices = market
                .prices
                .first()
                .map(String::as_str)
                .unwrap_or("no price notes");
            format!(
                "Market {open_orders} open • fallback {fallback_orders} • stock: {stock} • prices: {prices}"
            )
        })
        .unwrap_or_else(|| "Market details are still loading.".to_owned());
    let event = client
        .events
        .as_ref()
        .and_then(|events| events.events.last())
        .map(|event| {
            let choices = if event.intervention_options.is_empty() {
                "none listed".to_owned()
            } else {
                event.intervention_options.join(" | ")
            };
            let chosen = event.chosen_intervention.as_deref().unwrap_or("none");
            let outcome = event.outcome.as_deref().unwrap_or("pending");
            format!(
                "Event {:?}: {} • cause: {} • choices: {} • chosen: {chosen} • outcome: {outcome}",
                event.stage, event.title, event.cause, choices
            )
        })
        .unwrap_or_else(|| "Events quiet".to_owned());
    format!(
        "{} details\nRoads: {routes}\n{market}\n{event}",
        region.region_id
    )
}
