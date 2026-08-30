use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
mod tests;

pub(super) fn render(client: &Phase4Client) -> String {
    let offices = client
        .governance
        .as_ref()
        .map(|governance| {
            let filled = governance
                .offices
                .iter()
                .filter(|office| !office.vacant)
                .count();
            format!("Town hall {filled}/{} offices", governance.offices.len())
        })
        .unwrap_or_else(|| "Town hall loading".to_owned());
    let registry = client
        .claims
        .as_ref()
        .map(|claims| lease_registry_summary(client, claims))
        .unwrap_or_else(|| "Registry loading".to_owned());
    let orders = client
        .professions
        .as_ref()
        .map(|professions| {
            let open = professions
                .orders
                .iter()
                .filter(|order| order.status == tarrowyn_protocol::ServiceOrderStatus::Open)
                .count();
            format!("{open} orders open")
        })
        .unwrap_or_else(|| "Orders loading".to_owned());
    let knowledge = client
        .knowledge
        .as_ref()
        .map(|knowledge| {
            format!(
                "{} knowledge records",
                knowledge.knowledge.known_by_player.len()
            )
        })
        .unwrap_or_else(|| "Knowledge loading".to_owned());
    let skills = client
        .skills
        .as_ref()
        .map(|skills| {
            let mastered = skills
                .skills
                .iter()
                .filter(|skill| skill.status == SkillStatus::Mastered)
                .count();
            let resonating = skills
                .skills
                .iter()
                .filter(|skill| skill.status == SkillStatus::Resonating)
                .count();
            format!("Skills {mastered} mastered, {resonating} resonating")
        })
        .unwrap_or_else(|| "Skills loading".to_owned());
    let household = client
        .households
        .as_ref()
        .and_then(|households| households.households.first())
        .map(|household| {
            let status = match household.status {
                tarrowyn_protocol::HouseholdLifeStatus::Arrived => "arrived",
                tarrowyn_protocol::HouseholdLifeStatus::ReducedService => "reduced service",
                tarrowyn_protocol::HouseholdLifeStatus::ConsideringDeparture => {
                    "considering departure"
                }
                tarrowyn_protocol::HouseholdLifeStatus::Departed => "departed",
            };
            format!(
                "Local life {status} • service {}%",
                household.service_quality
            )
        })
        .unwrap_or_else(|| "Local life loading".to_owned());
    let treasury = client
        .governance
        .as_ref()
        .map(|governance| {
            let rate = governance
                .taxation
                .as_ref()
                .map(|policy| policy.rate_percent)
                .unwrap_or(0);
            format!(
                "Tax {rate}% • Treasury {} • {} receipts",
                governance.public_treasury,
                governance.tax_ledger.len()
            )
        })
        .unwrap_or_else(|| "Tax and treasury loading".to_owned());
    format!("{household} • {offices} • {registry}\n{treasury}\n{orders} • {knowledge} • {skills}")
}

fn lease_registry_summary(client: &Phase4Client, claims: &ClaimsResponse) -> String {
    let free = claims.available_plots.len();
    let own_claim = client.own_account_id.as_deref().and_then(|account_id| {
        claims
            .claims
            .iter()
            .rev()
            .find(|claim| claim.owner_account_id.as_deref() == Some(account_id))
    });
    let lease = own_claim
        .map(|claim| {
            let status = match claim.status {
                tarrowyn_protocol::ClaimLifecycleStatus::Requested => "requested",
                tarrowyn_protocol::ClaimLifecycleStatus::Active
                | tarrowyn_protocol::ClaimLifecycleStatus::Renewed
                | tarrowyn_protocol::ClaimLifecycleStatus::Transferred
                | tarrowyn_protocol::ClaimLifecycleStatus::Inherited => "access open",
                tarrowyn_protocol::ClaimLifecycleStatus::Abandoned => "abandoned; grace pending",
                tarrowyn_protocol::ClaimLifecycleStatus::Expired => "expired; grace pending",
                tarrowyn_protocol::ClaimLifecycleStatus::Reclaimed => "reclaimed",
            };
            if claim.expires_at_unix_seconds > 0
                && matches!(
                    claim.status,
                    tarrowyn_protocol::ClaimLifecycleStatus::Active
                        | tarrowyn_protocol::ClaimLifecycleStatus::Renewed
                        | tarrowyn_protocol::ClaimLifecycleStatus::Transferred
                        | tarrowyn_protocol::ClaimLifecycleStatus::Inherited
                )
            {
                format!(
                    "lease {status}, {}",
                    lease_remaining(claim.expires_at_unix_seconds, unix_time_seconds())
                )
            } else {
                format!("lease {status}")
            }
        })
        .unwrap_or_else(|| format!("{}-day real leases", claims.lease_duration_days.max(1)));
    format!("{free} plots free • {lease}")
}

fn lease_remaining(expires_at: u64, now: u64) -> String {
    let seconds = expires_at.saturating_sub(now);
    if seconds >= 24 * 60 * 60 {
        format!("{}d left", seconds.div_ceil(24 * 60 * 60))
    } else {
        format!("{}h left", seconds.div_ceil(60 * 60))
    }
}

fn unix_time_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
