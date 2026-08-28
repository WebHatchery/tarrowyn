use super::*;

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
        .map(|claims| format!("{} plots available", claims.available_plots.len()))
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
                "{} lessons known",
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
    format!("{offices} • {registry}\n{treasury}\n{orders} • {knowledge} • {skills}")
}
