use super::{account_id, account_name, cache_key, record, validate_request_id};
use crate::config::ServerConfig;
use tarrowyn_protocol::{
    ApiResponse, ClaimLifecycleAction, ClaimLifecycleRequest, ClaimLifecycleResponse,
    ClaimLifecycleStatus, ClaimRecord, ClaimsResponse,
};

impl super::super::WorldRepository {
    pub fn claims(
        &self,
        token: &str,
    ) -> Result<ApiResponse<ClaimsResponse>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        super::super::expire_sessions(&mut state, &self.config);
        super::super::authenticate(&mut state, token, &self.config)?;
        tick(&mut state, &self.config);
        self.persist(&state);
        Ok(ApiResponse {
            meta: super::super::meta(state.tick, None, Some(state.cursor)),
            data: claims_view(&state, &self.config),
        })
    }

    pub fn claim_lifecycle(
        &self,
        token: &str,
        request: ClaimLifecycleRequest,
    ) -> Result<ApiResponse<ClaimLifecycleResponse>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        super::super::expire_sessions(&mut state, &self.config);
        let key = super::super::authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let actor_id = account_id(&state, &key);
        let cache = cache_key(&actor_id, &request.request_id);
        if let Some(super::Phase4Response::Claim(response)) =
            state.phase4.request_results.get(&cache)
        {
            return Ok(ApiResponse {
                meta: super::super::meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        tick(&mut state, &self.config);
        let mut response = ClaimLifecycleResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            claim: None,
            claims: claims_view(&state, &self.config),
            reason: None,
        };
        match request.action {
            ClaimLifecycleAction::Inspect => {
                response.claim = find_claim(&state, request.claim_id.as_deref()).cloned();
                response.accepted = true;
            }
            ClaimLifecycleAction::Request => {
                if let Some(position) = state.phase4.available_plots.pop() {
                    let actor_name = account_name(&state, &key);
                    let claim = ClaimRecord {
                        claim_id: format!("lease-{}", state.phase4.next_claim_id),
                        plot_id: format!("plot-{}", state.phase4.next_claim_id),
                        owner_account_id: Some(actor_id.clone()),
                        owner_name: Some(actor_name.clone()),
                        position,
                        lease_days: super::lease_duration_days(&self.config),
                        started_tick: state.tick,
                        expires_tick: state.tick,
                        started_at_unix_seconds: 0,
                        expires_at_unix_seconds: 0,
                        last_active_tick: state.tick,
                        status: ClaimLifecycleStatus::Requested,
                        approved_by: None,
                        building_access: false,
                        protected_goods_policy:
                            "Stored goods and character progression remain safe outside the claim."
                                .to_owned(),
                        inspection_note:
                            "Awaiting a town-hall approval before building access begins."
                                .to_owned(),
                    };
                    state.phase4.next_claim_id += 1;
                    response.claim = Some(claim.clone());
                    state.phase4.claims.push(claim);
                    response.accepted = true;
                    record(
                        &mut state,
                        "lease requested",
                        "A player asks the registry for a piece of land",
                        &format!(
                            "{} requested a lease; approval is still visible in the registry.",
                            actor_name
                        ),
                    );
                } else {
                    response.reason = Some("No recognised plot is free; inspect abandoned opportunities or contribute to a public work.".to_owned());
                }
            }
            ClaimLifecycleAction::Approve => {
                let Some(index) = claim_index(&state, request.claim_id.as_deref()) else {
                    response.reason = Some("Name the requested lease to approve.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let owned_by_actor = state.phase4.claims[index].owner_account_id.as_deref()
                    == Some(actor_id.as_str());
                let steward = state.phase4.governance.offices.iter().any(|office| {
                    office.kind == tarrowyn_protocol::OfficeKind::Steward
                        && office.holder_account_id.as_deref() == Some(actor_id.as_str())
                });
                if !owned_by_actor && !steward {
                    response.reason = Some(
                        "The lease-holder or Settlement Steward must approve the claim.".to_owned(),
                    );
                } else if state.phase4.claims[index].status != ClaimLifecycleStatus::Requested {
                    response.reason = Some("That lease is not awaiting approval.".to_owned());
                } else {
                    let tick = state.tick;
                    let started_at = super::unix_time_seconds();
                    let claim = &mut state.phase4.claims[index];
                    claim.status = ClaimLifecycleStatus::Active;
                    claim.approved_by = Some(actor_id.clone());
                    claim.lease_days = super::lease_duration_days(&self.config);
                    claim.started_at_unix_seconds = started_at;
                    claim.expires_at_unix_seconds =
                        started_at.saturating_add(self.config.lease_duration_seconds.max(1));
                    claim.expires_tick = tick;
                    claim.building_access = true;
                    claim.inspection_note =
                        "Active recognised land right; renewal and transfer are recorded."
                            .to_owned();
                    response.claim = Some(claim.clone());
                    response.accepted = true;
                    record(&mut state, "lease approved", "The registry recognises a land right", "The requested plot now grants building access without touching stored goods.");
                }
            }
            ClaimLifecycleAction::Renew => {
                let Some(index) = claim_index(&state, request.claim_id.as_deref()) else {
                    response.reason = Some("Name the lease to renew.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                if state.phase4.claims[index].owner_account_id.as_deref() != Some(actor_id.as_str())
                {
                    response.reason =
                        Some("Only the recognised holder may renew this lease.".to_owned());
                } else if !matches!(
                    state.phase4.claims[index].status,
                    ClaimLifecycleStatus::Active | ClaimLifecycleStatus::Renewed
                ) {
                    response.reason = Some("Only an active lease can be renewed.".to_owned());
                } else {
                    let tick = state.tick;
                    let now = super::unix_time_seconds();
                    let claim = &mut state.phase4.claims[index];
                    claim.status = ClaimLifecycleStatus::Renewed;
                    claim.last_active_tick = tick;
                    claim.lease_days = super::lease_duration_days(&self.config);
                    if claim.expires_at_unix_seconds == 0 {
                        claim.started_at_unix_seconds = now;
                    }
                    claim.expires_at_unix_seconds = claim
                        .expires_at_unix_seconds
                        .max(now)
                        .saturating_add(self.config.lease_duration_seconds.max(1));
                    claim.expires_tick = tick;
                    response.claim = Some(claim.clone());
                    response.accepted = true;
                    record(&mut state, "lease renewed", "A land right is kept in good standing", "The registry extended a lease without changing the character or stored-goods ledger.");
                }
            }
            ClaimLifecycleAction::Transfer | ClaimLifecycleAction::Inherit => {
                let Some(index) = claim_index(&state, request.claim_id.as_deref()) else {
                    response.reason = Some("Name the lease to transfer.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let Some(target) = request.target_account_id.as_deref() else {
                    response.reason =
                        Some("A transfer or inheritance needs the receiving account.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let Some(target_key) = super::key_for_account(&state, target) else {
                    response.reason = Some(
                        "The receiving player must have a recognised settlement account."
                            .to_owned(),
                    );
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                if state.phase4.claims[index].owner_account_id.as_deref() != Some(actor_id.as_str())
                {
                    response.reason = Some(
                        "Only the recognised holder may transfer or bequeath the claim.".to_owned(),
                    );
                } else {
                    let tick = state.tick;
                    let target_name = account_name(&state, &target_key);
                    let claim = &mut state.phase4.claims[index];
                    claim.owner_account_id = Some(target.to_owned());
                    claim.owner_name = Some(target_name);
                    claim.status = if request.action == ClaimLifecycleAction::Transfer {
                        ClaimLifecycleStatus::Transferred
                    } else {
                        ClaimLifecycleStatus::Inherited
                    };
                    claim.last_active_tick = tick;
                    response.claim = Some(claim.clone());
                    response.accepted = true;
                    record(
                        &mut state,
                        "lease transferred",
                        "A land right changes hands without erasing its history",
                        &format!(
                            "The registry recorded a {} of a recognised lease.",
                            if request.action == ClaimLifecycleAction::Transfer {
                                "transfer"
                            } else {
                                "bounded inheritance"
                            }
                        ),
                    );
                }
            }
            ClaimLifecycleAction::Abandon => {
                let Some(index) = claim_index(&state, request.claim_id.as_deref()) else {
                    response.reason = Some("Name the lease to abandon.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                if state.phase4.claims[index].owner_account_id.as_deref() != Some(actor_id.as_str())
                {
                    response.reason =
                        Some("Only the recognised holder may abandon this lease.".to_owned());
                } else {
                    let tick = state.tick;
                    let claim = &mut state.phase4.claims[index];
                    claim.status = ClaimLifecycleStatus::Abandoned;
                    claim.building_access = false;
                    claim.last_active_tick = tick;
                    claim.inspection_note = "Abandoned claim; reclaim it to return the plot to the available land ledger.".to_owned();
                    response.claim = Some(claim.clone());
                    response.accepted = true;
                    record(&mut state, "lease abandoned", "A homestead is returned to the registry", "The abandoned building loses access, while unrelated character and stored goods remain safe.");
                }
            }
            ClaimLifecycleAction::Reclaim => {
                let Some(index) = claim_index(&state, request.claim_id.as_deref()) else {
                    response.reason =
                        Some("Name the expired or abandoned lease to reclaim.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                if !matches!(
                    state.phase4.claims[index].status,
                    ClaimLifecycleStatus::Expired | ClaimLifecycleStatus::Abandoned
                ) {
                    response.reason =
                        Some("Only an expired or abandoned claim can be reclaimed.".to_owned());
                } else {
                    let position = state.phase4.claims[index].position;
                    let claim_snapshot = {
                        let claim = &mut state.phase4.claims[index];
                        claim.status = ClaimLifecycleStatus::Reclaimed;
                        claim.owner_account_id = None;
                        claim.owner_name = None;
                        claim.building_access = false;
                        claim.inspection_note =
                            "Reclaimed plot is available to a late player or public contributor."
                                .to_owned();
                        claim.clone()
                    };
                    if !state.phase4.available_plots.contains(&position) {
                        state.phase4.available_plots.push(position);
                    }
                    response.claim = Some(claim_snapshot);
                    response.accepted = true;
                    record(
                        &mut state,
                        "lease reclaimed",
                        "The registry makes abandoned land available again",
                        "A reclaimed plot offers a late player a clean path into the settlement.",
                    );
                }
            }
        }
        response.claims = claims_view(&state, &self.config);
        finish(self, &mut state, cache, request.request_id, response)
    }
}

fn finish(
    repository: &super::super::WorldRepository,
    state: &mut super::super::models::RepositoryState,
    cache: String,
    request_id: String,
    response: ClaimLifecycleResponse,
) -> Result<ApiResponse<ClaimLifecycleResponse>, super::super::RepositoryError> {
    let actor = cache
        .strip_prefix("phase4:")
        .and_then(|value| value.split_once(':'))
        .map(|(account, _)| account)
        .unwrap_or("unknown-account")
        .to_owned();
    let target = response
        .claim
        .as_ref()
        .map(|claim| claim.claim_id.clone())
        .unwrap_or_else(|| request_id.clone());
    super::super::phase6::audit_command(
        state,
        &actor,
        "claim.lifecycle",
        &target,
        response.accepted,
        "A land-right command was recorded in the settlement audit stream.",
    );
    state
        .phase4
        .request_results
        .insert(cache, super::Phase4Response::Claim(response.clone()));
    super::super::record_command_outcome(state, response.accepted);
    repository.persist(state);
    Ok(ApiResponse {
        meta: super::super::meta(state.tick, Some(request_id), Some(state.cursor)),
        data: response,
    })
}

fn claims_view(
    state: &super::super::models::RepositoryState,
    config: &ServerConfig,
) -> ClaimsResponse {
    ClaimsResponse {
        claims: state.phase4.claims.clone(),
        available_plots: state.phase4.available_plots.clone(),
        lease_duration_days: super::lease_duration_days(config),
        cursor: state.cursor,
    }
}

fn claim_index(
    state: &super::super::models::RepositoryState,
    claim_id: Option<&str>,
) -> Option<usize> {
    claim_id
        .and_then(|claim_id| {
            state
                .phase4
                .claims
                .iter()
                .position(|claim| claim.claim_id == claim_id)
        })
        .or_else(|| {
            state
                .phase4
                .claims
                .iter()
                .position(|claim| claim.owner_account_id.is_some())
        })
}

fn find_claim<'a>(
    state: &'a super::super::models::RepositoryState,
    claim_id: Option<&str>,
) -> Option<&'a ClaimRecord> {
    claim_id
        .and_then(|claim_id| {
            state
                .phase4
                .claims
                .iter()
                .find(|claim| claim.claim_id == claim_id)
        })
        .or_else(|| state.phase4.claims.last())
}

pub(super) fn tick(state: &mut super::super::models::RepositoryState, config: &ServerConfig) {
    let now = super::unix_time_seconds();
    let mut expired = Vec::new();
    for claim in &mut state.phase4.claims {
        if matches!(
            claim.status,
            ClaimLifecycleStatus::Active
                | ClaimLifecycleStatus::Renewed
                | ClaimLifecycleStatus::Transferred
                | ClaimLifecycleStatus::Inherited
        ) && claim.expires_at_unix_seconds > 0
            && now >= claim.expires_at_unix_seconds
        {
            claim.status = ClaimLifecycleStatus::Expired;
            claim.building_access = false;
            claim.last_active_tick = state.tick;
            claim.inspection_note = "The lease expired; access is closed while the registry waits before reclaiming it.".to_owned();
            expired.push(claim.claim_id.clone());
        }
    }
    for claim_id in expired {
        record(state, "lease expired", "The registry closes an unattended building", &format!("Claim {claim_id} expired without deleting the character or protected stored goods."));
    }
    let mut reclaimed = Vec::new();
    for claim in &mut state.phase4.claims {
        if matches!(
            claim.status,
            ClaimLifecycleStatus::Abandoned | ClaimLifecycleStatus::Expired
        ) && state.tick.saturating_sub(claim.last_active_tick)
            >= config.claim_reclaim_grace_ticks
        {
            claim.status = ClaimLifecycleStatus::Reclaimed;
            claim.owner_account_id = None;
            claim.owner_name = None;
            claim.building_access = false;
            reclaimed.push((claim.claim_id.clone(), claim.position));
        }
    }
    for (claim_id, position) in reclaimed {
        if !state.phase4.available_plots.contains(&position) {
            state.phase4.available_plots.push(position);
        }
        record(
            state,
            "lease reclaimed",
            "The registry opens a path for a late player",
            &format!("Claim {claim_id} is available again after its grace period."),
        );
    }
}
