use super::super::{infrastructure_status, MAX_TAX_COLLECTIONS};
use super::record;
use crate::config::ServerConfig;
use tarrowyn_protocol::{OfficeKind, TaxCollection};

const TAX_TERRITORY: &str = "hearth-settlement";
const HEARTH_POSITION: tarrowyn_protocol::Position = tarrowyn_protocol::Position { x: 8, y: 5 };
const TAX_RADIUS: u32 = 4;

pub(crate) fn tick(
    state: &mut super::super::super::models::RepositoryState,
    config: &ServerConfig,
) {
    if state.tick == 0 {
        return;
    }
    let interval = config.governance_inactivity_ticks.max(1);
    if state.tick.is_multiple_of(interval) {
        let mut vacant = Vec::new();
        for office in &mut state.phase4.governance.offices {
            if office.holder_account_id.is_some()
                && state.tick.saturating_sub(office.last_active_tick) > interval
            {
                let title = office.title.clone();
                office.holder_account_id = None;
                office.holder_name = None;
                office.vacant = true;
                office.vacancy_reason = Some(
                    "The office-holder has been absent; a new player may take responsibility."
                        .to_owned(),
                );
                vacant.push(title);
            }
        }
        if !vacant.is_empty() {
            state.phase4.governance.administration_quality = state
                .phase4
                .governance
                .administration_quality
                .saturating_sub(12);
            for title in vacant {
                record(
                    state,
                    "office vacancy",
                    "The town hall leaves a lamp lit for a successor",
                    &format!("{title} is vacant; the settlement remains usable while it waits."),
                );
            }
        } else if state
            .phase4
            .governance
            .offices
            .iter()
            .any(|office| office.kind == OfficeKind::Steward && !office.vacant)
        {
            state.phase4.governance.administration_quality = state
                .phase4
                .governance
                .administration_quality
                .saturating_add(1)
                .min(100);
        } else {
            state.phase4.governance.administration_quality = state
                .phase4
                .governance
                .administration_quality
                .saturating_sub(2);
        }
    }
    if state.tick.is_multiple_of(12) {
        let upkeep = state
            .phase4
            .infrastructure
            .iter()
            .map(|record| record.upkeep_per_day)
            .fold(0, u32::saturating_add);
        let funded = state.phase4.governance.public_treasury >= upkeep;
        if funded {
            state.phase4.governance.public_treasury -= upkeep;
        }
        let mut failures = Vec::new();
        for record in &mut state.phase4.infrastructure {
            let old_status = record.status;
            if funded {
                record.last_maintained_tick = state.tick;
            } else {
                record.condition = record.condition.saturating_sub(5);
                record.status = infrastructure_status(record.condition);
                if record.status == tarrowyn_protocol::InfrastructureStatus::Failed
                    && old_status != record.status
                {
                    failures.push(record.name.clone());
                }
            }
        }
        for name in failures {
            record(
                state,
                "infrastructure failure",
                "A shared structure needs a public response",
                &format!("{name} has failed because its upkeep account ran dry."),
            );
        }
    }
    collect_taxes(state);
}

fn collect_taxes(state: &mut super::super::super::models::RepositoryState) {
    let Some(policy) = state.phase4.governance.taxation.clone() else {
        return;
    };
    if !state
        .phase4
        .governance
        .offices
        .iter()
        .any(|office| office.kind == OfficeKind::Steward && !office.vacant)
    {
        return;
    }

    let day = state.clock.day;
    let rate = u32::from(policy.rate_percent);
    let mut total: u32 = 0;
    let mut payers = Vec::new();
    for identity in state.identities.values_mut() {
        if identity.last_tax_day >= day {
            continue;
        }
        identity.last_tax_day = day;
        if identity.knocked_out
            || identity.position.manhattan_distance(HEARTH_POSITION) > TAX_RADIUS
            || rate == 0
        {
            continue;
        }
        let amount = identity.gold.saturating_mul(rate) / 100;
        if amount == 0 {
            continue;
        }
        identity.gold -= amount;
        total = total.saturating_add(amount);
        payers.push((
            identity.account_id.clone(),
            identity.display_name.clone(),
            amount,
        ));
    }
    if total == 0 {
        return;
    }

    let tick = state.tick;
    let payer_count = payers.len();
    for (account_id, payer_name, amount) in payers {
        let collection_id = format!("tax-{}", state.phase4.next_tax_id);
        state.phase4.next_tax_id = state.phase4.next_tax_id.saturating_add(1);
        state.phase4.governance.tax_ledger.push(TaxCollection {
            collection_id,
            payer_account_id: account_id,
            payer_name,
            amount,
            rate_percent: policy.rate_percent,
            territory: TAX_TERRITORY.to_owned(),
            day,
            created_tick: tick,
        });
    }
    super::super::retain_recent(&mut state.phase4.governance.tax_ledger, MAX_TAX_COLLECTIONS);
    state.phase4.governance.public_treasury = state
        .phase4
        .governance
        .public_treasury
        .saturating_add(total);
    record(
        state,
        "tax collection",
        "The Hearth treasury receives its daily public tax",
        &format!(
            "{payer_count} nearby player balance(s) contributed {total} public gold at {}% for day {day}.",
            policy.rate_percent
        ),
    );
}
