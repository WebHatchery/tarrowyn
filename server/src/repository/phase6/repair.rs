//! Bounded support repairs for durable claim and household corruption.

use super::super::models::RepositoryState;
use super::super::phase4::unix_time_seconds;
use tarrowyn_protocol::ClaimLifecycleStatus;

pub(super) fn restore_claim(
    state: &mut RepositoryState,
    target_account: &str,
    target_id: Option<&str>,
) -> (bool, String, Option<String>) {
    let Some(claim_id) = target_id.filter(|id| !id.trim().is_empty()) else {
        return (
            false,
            String::new(),
            Some("A claim ID is required for claim repair.".to_owned()),
        );
    };
    let Some(index) = state
        .phase4
        .claims
        .iter()
        .position(|claim| claim.claim_id == claim_id)
    else {
        return (
            false,
            String::new(),
            Some("That claim is not present in the authoritative registry.".to_owned()),
        );
    };
    let claim = &state.phase4.claims[index];
    if claim.owner_account_id.as_deref() != Some(target_account) {
        return (
            false,
            String::new(),
            Some("Claim repair may only restore the named account's land right.".to_owned()),
        );
    }
    if !matches!(
        claim.status,
        ClaimLifecycleStatus::Active
            | ClaimLifecycleStatus::Renewed
            | ClaimLifecycleStatus::Transferred
            | ClaimLifecycleStatus::Inherited
    ) {
        return (
            false,
            String::new(),
            Some("Only a recognised active lease can have its access restored.".to_owned()),
        );
    }
    if claim.expires_at_unix_seconds > 0 && claim.expires_at_unix_seconds <= unix_time_seconds() {
        return (
            false,
            String::new(),
            Some("The lease has expired; claim repair cannot extend its term.".to_owned()),
        );
    }
    let claim = &mut state.phase4.claims[index];
    if claim.building_access {
        return (
            true,
            "The recognised claim already has consistent building access.".to_owned(),
            None,
        );
    }
    claim.building_access = true;
    claim.last_active_tick = state.tick;
    claim.inspection_note =
        "Support restored access to an active recognised land right without extending its lease."
            .to_owned();
    super::super::phase3::record(
        state,
        "support claim repair",
        "The registry restored a recognised land right",
        "Support repaired claim access without changing the lease term or protected goods policy.",
    );
    state.phase5.cursor = state.cursor;
    (
        true,
        "Claim access was restored without extending the lease or changing protected goods."
            .to_owned(),
        None,
    )
}

pub(super) fn merge_household(
    state: &mut RepositoryState,
    target_id: Option<&str>,
) -> (bool, String, Option<String>) {
    let Some(target_id) = target_id.filter(|id| !id.trim().is_empty()) else {
        return (
            false,
            String::new(),
            Some("A regional household ID is required for household repair.".to_owned()),
        );
    };
    let indexes: Vec<usize> = state
        .phase5
        .households
        .iter()
        .enumerate()
        .filter(|(_, household)| household.household_id == target_id)
        .map(|(index, _)| index)
        .collect();
    if indexes.is_empty() {
        return (
            false,
            String::new(),
            Some("That regional household is not present in the authoritative world.".to_owned()),
        );
    }
    if indexes.len() == 1 {
        return (
            true,
            "The regional household already has one authoritative record.".to_owned(),
            None,
        );
    }
    let canonical_index = indexes
        .iter()
        .copied()
        .max_by_key(|index| household_activity_tick(&state.phase5.households[*index]))
        .expect("duplicate household indexes are non-empty");
    let canonical = state.phase5.households[canonical_index].clone();
    for index in &indexes {
        let duplicate = &state.phase5.households[*index];
        if duplicate.household_name != canonical.household_name
            || duplicate.origin_location_id != canonical.origin_location_id
            || duplicate.destination_location_id != canonical.destination_location_id
        {
            return (
                false,
                String::new(),
                Some("Duplicate household records disagree on identity or route.".to_owned()),
            );
        }
        if household_activity_tick(duplicate) == household_activity_tick(&canonical)
            && duplicate.status != canonical.status
        {
            return (
                false,
                String::new(),
                Some("Duplicate household records disagree on their current status.".to_owned()),
            );
        }
    }
    let mut merged = canonical.clone();
    for index in &indexes {
        for entry in &state.phase5.households[*index].history {
            if !merged.history.contains(entry) {
                merged.history.push(entry.clone());
            }
        }
    }
    let original = std::mem::take(&mut state.phase5.households);
    state.phase5.households = original
        .into_iter()
        .enumerate()
        .filter_map(|(index, household)| {
            if household.household_id != target_id {
                Some(household)
            } else if index == canonical_index {
                Some(merged.clone())
            } else {
                None
            }
        })
        .collect();
    super::super::phase3::record(
        state,
        "support household repair",
        "The regional registry merged duplicate household records",
        "Support retained the household's route history while restoring one authoritative regional record.",
    );
    state.phase5.cursor = state.cursor;
    (
        true,
        "Duplicate regional household records were merged and their history was retained."
            .to_owned(),
        None,
    )
}

fn household_activity_tick(household: &tarrowyn_protocol::RegionalHousehold) -> u64 {
    household
        .departure_tick
        .unwrap_or(0)
        .max(household.arrival_tick.unwrap_or(0))
}
