//! Bounded support repairs for durable claim and household corruption.

use super::super::models::RepositoryState;
use super::super::phase4::unix_time_seconds;
use super::super::{meta, record_command_outcome, RepositoryError, WorldRepository};
use super::{audit, is_support_operator, validate_request_id};
use tarrowyn_protocol::{
    ApiResponse, ClaimLifecycleStatus, SupportRepairAction, SupportRepairRequest,
    SupportRepairResponse,
};

impl WorldRepository {
    pub fn support_repair(
        &self,
        token: &str,
        request: SupportRepairRequest,
    ) -> Result<ApiResponse<SupportRepairResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        let actor_key = super::super::authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let actor = state
            .identities
            .get(&actor_key)
            .expect("identity exists")
            .account_id
            .clone();
        if !is_support_operator(&self.config, &actor) {
            return Err(RepositoryError::new(
                403,
                "support_operator_required",
                "A configured support operator account is required for repair actions.",
            ));
        }
        if request.note.trim().is_empty() {
            return Err(RepositoryError::new(
                400,
                "repair_note_required",
                "Every support repair needs an operator note.",
            ));
        }
        let cache = format!("repair:{}:{}", actor, request.request_id);
        if let Some(previous) = state.phase6.request_results.get(&cache) {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous.clone(),
            });
        }
        let target_account = request.account_id.clone().unwrap_or_else(|| actor.clone());
        let target_key = state
            .identities
            .iter()
            .find(|(_, identity)| identity.account_id == target_account)
            .map(|(key, _)| key.clone());
        let (accepted, summary, reason) = match request.action {
            SupportRepairAction::ClearStuckTravel => clear_stuck_travel(&mut state, target_key),
            SupportRepairAction::NormalizeInventory => normalize_inventory(&mut state, target_key),
            SupportRepairAction::ReconcileTrade => super::super::phase5::reconcile_market_order(
                &mut state,
                request.target_id.as_deref(),
            ),
            SupportRepairAction::RestoreClaim => {
                restore_claim(&mut state, &target_account, request.target_id.as_deref())
            }
            SupportRepairAction::MergeHousehold => {
                merge_household(&mut state, request.target_id.as_deref())
            }
            SupportRepairAction::ResolveModeration => {
                resolve_moderation(&mut state, request.target_id.as_deref())
            }
        };
        let audit_id = audit(
            &mut state,
            &actor,
            "support.repair",
            &target_account,
            if accepted { "accepted" } else { "rejected" },
            &request.note,
        );
        let response = SupportRepairResponse {
            request_id: request.request_id.clone(),
            audit_id,
            accepted,
            summary,
            reason,
        };
        state.phase6.request_results.insert(cache, response.clone());
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }
}

fn clear_stuck_travel(
    state: &mut RepositoryState,
    target_key: Option<String>,
) -> (bool, String, Option<String>) {
    let Some(target_key) = target_key else {
        return (
            false,
            String::new(),
            Some("The target account is not present.".to_owned()),
        );
    };
    state.phase5.travel.remove(&target_key);
    if let Some(identity) = state.identities.get_mut(&target_key) {
        identity.position = crate::content::region_location_profile("hearth").position;
    }
    (
        true,
        "Stuck travel cleared at the origin with cargo and rewards preserved.".to_owned(),
        None,
    )
}

fn normalize_inventory(
    state: &mut RepositoryState,
    target_key: Option<String>,
) -> (bool, String, Option<String>) {
    let Some(target_key) = target_key else {
        return (
            false,
            String::new(),
            Some("The target account is not present.".to_owned()),
        );
    };
    if let Some(identity) = state.identities.get_mut(&target_key) {
        identity.inventory.wheat = identity.inventory.wheat.min(9_999);
        identity.inventory.turnips = identity.inventory.turnips.min(9_999);
        identity.inventory.moonberries = identity.inventory.moonberries.min(9_999);
        identity.inventory.seeds = identity.inventory.seeds.min(9_999);
    }
    (
        true,
        "Inventory values were normalised to the documented support ceiling.".to_owned(),
        None,
    )
}

fn resolve_moderation(
    state: &mut RepositoryState,
    target_id: Option<&str>,
) -> (bool, String, Option<String>) {
    let Some(report) = target_id.and_then(|id| state.phase6.reports.get_mut(id)) else {
        return (
            false,
            String::new(),
            Some("That moderation report is not recorded.".to_owned()),
        );
    };
    report.status = "resolved".to_owned();
    (
        true,
        "Moderation report marked resolved and retained in the audit record.".to_owned(),
        None,
    )
}

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
