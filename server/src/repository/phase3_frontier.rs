use super::phase3::{cache_key, frontier_event, has_roles, record, Phase3Response};
use super::*;
use tarrowyn_protocol::{
    ApiResponse, ClaimAction, ClaimRequest, ClaimResponse, ClaimStatus, Expedition,
    ExpeditionAction, ExpeditionMember, ExpeditionRequest, ExpeditionResponse, ExpeditionRole,
    ExpeditionStatus, FrontierEvent, LandClaim, Position,
};

const MAX_OUTPOST_NAME_CHARS: usize = 80;
const MAX_EXPEDITION_SUPPLY: u32 = 99;
const PIONEER_EXPEDITION_ID: &str = "pioneer-1";

fn expedition_selector_matches(requested_id: Option<&str>, expedition_id: &str) -> bool {
    requested_id.is_none_or(|requested_id| requested_id == expedition_id)
}

fn expedition_is_staffed(expedition: &Expedition) -> bool {
    expedition.members.len() >= 3 && has_roles(&expedition.members)
}

fn expedition_has_member(expedition: &Expedition, account_id: &str) -> bool {
    expedition
        .members
        .iter()
        .any(|member| member.account_id == account_id)
}

fn expedition_is_prepared(expedition: &Expedition, config: &ServerConfig) -> bool {
    expedition_is_staffed(expedition)
        && expedition.food >= config.expedition_min_food
        && expedition.tools >= config.expedition_min_tools
        && expedition.materials >= config.expedition_min_materials
        && expedition.safety >= config.expedition_min_safety
}

fn validate_outpost_name(name: Option<&str>) -> Result<Option<String>, RepositoryError> {
    let Some(name) = name else {
        return Ok(None);
    };
    let name = name.trim();
    if name.is_empty() {
        return Ok(None);
    }
    if name.chars().count() > MAX_OUTPOST_NAME_CHARS || name.chars().any(char::is_control) {
        return Err(RepositoryError::new(
            400,
            "invalid_outpost_name",
            "The outpost name must be at most 80 characters and contain no control characters.",
        ));
    }
    Ok(Some(name.to_owned()))
}

fn add_expedition_supply(total: &mut u32, requested: u32) {
    *total = total
        .saturating_add(requested.min(MAX_EXPEDITION_SUPPLY))
        .min(MAX_EXPEDITION_SUPPLY);
}

pub(super) fn backfill_expedition_credentials(phase: &mut super::phase3::Phase3State) {
    let Some(expedition) = phase.expedition.as_ref() else {
        return;
    };
    if expedition.status != ExpeditionStatus::Succeeded {
        return;
    }
    let participants = expedition
        .members
        .iter()
        .map(|member| member.account_id.clone())
        .collect::<Vec<_>>();
    for account_id in participants {
        if !phase.expedition_credentials.contains(&account_id) {
            phase.expedition_credentials.push(account_id);
        }
    }
}

impl WorldRepository {
    pub fn claim(
        &self,
        token: &str,
        request: ClaimRequest,
    ) -> Result<ApiResponse<ClaimResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let cache_key = cache_key(&key, &request.request_id);
        if let Some(Phase3Response::Claim(response)) = state.phase3.request_results.get(&cache_key)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let account = state.identities.get(&key).expect("identity exists");
        let account_id = account.account_id.clone();
        let owner_name = account.display_name.clone();
        let mut response = ClaimResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            claim: state.phase3.claim.clone(),
            reason: None,
        };
        match request.action {
            ClaimAction::Inspect => response.accepted = true,
            ClaimAction::Request => {
                if state
                    .phase3
                    .claim
                    .as_ref()
                    .is_some_and(|claim| claim.status == ClaimStatus::Active)
                {
                    response.reason =
                        Some("The recognised homestead is already leased.".to_owned());
                } else {
                    let claim = LandClaim {
                        claim_id: "homestead-1".to_owned(),
                        owner_account_id: account_id.clone(),
                        owner_name,
                        position: Position { x: 10, y: 8 },
                        lease_days: 3,
                        last_active_tick: state.tick,
                        reclaim_after_ticks: self.config.claim_reclaim_ticks,
                        status: ClaimStatus::Active,
                    };
                    state.phase3.claim = Some(claim.clone());
                    response.claim = Some(claim.clone());
                    response.accepted = true;
                    frontier_event(&mut state, FrontierEvent::Claim(claim));
                    record(
                        &mut state,
                        "claim founded",
                        "A homestead lease is recognised beyond the first road",
                        "A player has taken the renewable homestead lease; inactivity will reclaim it.",
                    );
                }
            }
            ClaimAction::Renew | ClaimAction::Abandon => {
                let Some(existing) = state.phase3.claim.clone() else {
                    response.reason = Some("There is no homestead lease to change.".to_owned());
                    state
                        .phase3
                        .request_results
                        .insert(cache_key, Phase3Response::Claim(response.clone()));
                    record_command_outcome(&mut state, response.accepted);
                    self.persist(&state);
                    return Ok(ApiResponse {
                        meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                        data: response,
                    });
                };
                if existing.owner_account_id != account_id {
                    response.reason =
                        Some("Only the lease holder can change this homestead.".to_owned());
                } else if existing.status != ClaimStatus::Active {
                    response.reason =
                        Some("That lease has already returned to the frontier.".to_owned());
                } else {
                    let changed_claim = {
                        let tick = state.tick;
                        let claim = state.phase3.claim.as_mut().expect("claim exists");
                        if request.action == ClaimAction::Renew {
                            claim.last_active_tick = tick;
                        } else {
                            claim.status = ClaimStatus::Abandoned;
                        }
                        claim.clone()
                    };
                    response.accepted = true;
                    response.claim = Some(changed_claim.clone());
                    frontier_event(&mut state, FrontierEvent::Claim(changed_claim));
                    record(
                        &mut state,
                        if request.action == ClaimAction::Renew {
                            "claim renewed"
                        } else {
                            "claim abandoned"
                        },
                        "The homestead ledger changes",
                        if request.action == ClaimAction::Renew {
                            "The lease holder renewed the homestead."
                        } else {
                            "The lease holder released the homestead."
                        },
                    );
                }
            }
        }
        state
            .phase3
            .request_results
            .insert(cache_key, Phase3Response::Claim(response.clone()));
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }

    pub fn expedition(
        &self,
        token: &str,
        request: ExpeditionRequest,
    ) -> Result<ApiResponse<ExpeditionResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let expedition_id = super::validate_optional_identifier(
            request.expedition_id.as_deref(),
            "invalid_expedition_id",
            "An expedition selector must be bounded and contain no control characters.",
        )?;
        let requested_outpost_name = if request.action == ExpeditionAction::Announce {
            validate_outpost_name(request.outpost_name.as_deref())?
        } else {
            None
        };
        let cache_key = cache_key(&key, &request.request_id);
        if let Some(Phase3Response::Expedition(response)) =
            state.phase3.request_results.get(&cache_key)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let identity = state.identities.get(&key).expect("identity exists");
        let account_id = identity.account_id.clone();
        let display_name = identity.display_name.clone();
        let mut response = ExpeditionResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            expedition: state.phase3.expedition.clone(),
            reason: None,
        };
        match request.action {
            ExpeditionAction::Announce => {
                if !expedition_selector_matches(expedition_id.as_deref(), PIONEER_EXPEDITION_ID) {
                    response.reason = Some("That expedition is no longer current.".to_owned());
                } else if state.phase3.expedition.as_ref().is_some_and(|expedition| {
                    matches!(
                        expedition.status,
                        ExpeditionStatus::Planning | ExpeditionStatus::Launched
                    )
                }) {
                    response.reason =
                        Some("A pioneer expedition is already on the registry.".to_owned());
                } else {
                    backfill_expedition_credentials(&mut state.phase3);
                    let role = request.role.unwrap_or(ExpeditionRole::Scout);
                    let expedition = Expedition {
                        expedition_id: PIONEER_EXPEDITION_ID.to_owned(),
                        outpost_name: requested_outpost_name
                            .unwrap_or_else(|| "Lantern Rest".to_owned()),
                        leader_account_id: account_id.clone(),
                        members: vec![ExpeditionMember {
                            account_id: account_id.clone(),
                            display_name: display_name.clone(),
                            role,
                        }],
                        food: 0,
                        tools: 0,
                        materials: 0,
                        safety: 0,
                        status: ExpeditionStatus::Planning,
                        outcome: None,
                        outpost_position: crate::content::region_location_profile(
                            "whisperwood-outpost",
                        )
                        .position,
                    };
                    state.phase3.expedition = Some(expedition.clone());
                    response.expedition = Some(expedition.clone());
                    response.accepted = true;
                    record(
                        &mut state,
                        "expedition announced",
                        "A pioneer road is proposed",
                        "A prepared group may now join the first outpost attempt.",
                    );
                }
            }
            ExpeditionAction::Join => {
                let Some(expedition) = state.phase3.expedition.as_mut() else {
                    response.reason =
                        Some("Announce the pioneer expedition at the registry first.".to_owned());
                    state
                        .phase3
                        .request_results
                        .insert(cache_key, Phase3Response::Expedition(response.clone()));
                    record_command_outcome(&mut state, response.accepted);
                    self.persist(&state);
                    return Ok(ApiResponse {
                        meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                        data: response,
                    });
                };
                let Some(role) = request.role else {
                    response.reason = Some("Choose a complementary expedition role.".to_owned());
                    state
                        .phase3
                        .request_results
                        .insert(cache_key, Phase3Response::Expedition(response.clone()));
                    record_command_outcome(&mut state, response.accepted);
                    self.persist(&state);
                    return Ok(ApiResponse {
                        meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                        data: response,
                    });
                };
                if !expedition_selector_matches(expedition_id.as_deref(), &expedition.expedition_id)
                {
                    response.reason = Some("That expedition is no longer current.".to_owned());
                } else if expedition.status != ExpeditionStatus::Planning {
                    response.reason = Some("The pioneer party is no longer gathering.".to_owned());
                } else if expedition
                    .members
                    .iter()
                    .any(|member| member.account_id == account_id)
                {
                    response.reason = Some("You are already named on this expedition.".to_owned());
                } else if expedition.members.len() >= super::phase3::MAX_EXPEDITION_MEMBERS {
                    response.reason = Some(
                        "The pioneer party has reached its 20-member planning limit.".to_owned(),
                    );
                } else {
                    expedition.members.push(ExpeditionMember {
                        account_id,
                        display_name,
                        role,
                    });
                    response.accepted = true;
                }
                response.expedition = Some(expedition.clone());
            }
            ExpeditionAction::Supply => {
                let Some(expedition) = state.phase3.expedition.as_mut() else {
                    response.reason = Some("There is no pioneer expedition to supply.".to_owned());
                    state
                        .phase3
                        .request_results
                        .insert(cache_key, Phase3Response::Expedition(response.clone()));
                    record_command_outcome(&mut state, response.accepted);
                    self.persist(&state);
                    return Ok(ApiResponse {
                        meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                        data: response,
                    });
                };
                if !expedition_selector_matches(expedition_id.as_deref(), &expedition.expedition_id)
                {
                    response.reason = Some("That expedition is no longer current.".to_owned());
                } else if expedition.status != ExpeditionStatus::Planning {
                    response.reason =
                        Some("Only a gathering expedition can receive supplies.".to_owned());
                } else if expedition
                    .members
                    .iter()
                    .any(|member| member.account_id == account_id)
                {
                    add_expedition_supply(&mut expedition.food, request.food);
                    add_expedition_supply(&mut expedition.tools, request.tools);
                    add_expedition_supply(&mut expedition.materials, request.materials);
                    add_expedition_supply(&mut expedition.safety, request.safety);
                    response.accepted = true;
                } else {
                    response.reason = Some("Join the group before adding supplies.".to_owned());
                }
                response.expedition = Some(expedition.clone());
            }
            ExpeditionAction::Launch => {
                let Some(existing) = state.phase3.expedition.clone() else {
                    response.reason = Some("There is no pioneer expedition to launch.".to_owned());
                    state
                        .phase3
                        .request_results
                        .insert(cache_key, Phase3Response::Expedition(response.clone()));
                    record_command_outcome(&mut state, response.accepted);
                    self.persist(&state);
                    return Ok(ApiResponse {
                        meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                        data: response,
                    });
                };
                if !expedition_selector_matches(expedition_id.as_deref(), &existing.expedition_id) {
                    response.reason = Some("That expedition is no longer current.".to_owned());
                } else if !expedition_has_member(&existing, &account_id) {
                    response.reason =
                        Some("Join the pioneer party before launching it.".to_owned());
                } else if existing.status != ExpeditionStatus::Planning {
                    response.reason =
                        Some("Only a gathering expedition can be launched.".to_owned());
                } else if !expedition_is_staffed(&existing) {
                    response.reason = Some(
                        "The party still needs scout, farmer, and builder roles before it can leave."
                            .to_owned(),
                    );
                } else {
                    let expedition = state.phase3.expedition.as_mut().expect("expedition exists");
                    expedition.status = ExpeditionStatus::Launched;
                    response.accepted = true;
                    response.expedition = Some(expedition.clone());
                    record(
                        &mut state,
                        "expedition launched",
                        "A staffed group leaves for the frontier",
                        "Three complementary roles are on the road; their supplies will decide whether the outpost can be founded.",
                    );
                }
                if !response.accepted {
                    response.expedition = Some(existing.clone());
                }
            }
            ExpeditionAction::Resolve => {
                let Some(existing) = state.phase3.expedition.clone() else {
                    response.reason = Some("There is no pioneer expedition to resolve.".to_owned());
                    state
                        .phase3
                        .request_results
                        .insert(cache_key, Phase3Response::Expedition(response.clone()));
                    record_command_outcome(&mut state, response.accepted);
                    self.persist(&state);
                    return Ok(ApiResponse {
                        meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                        data: response,
                    });
                };
                if !expedition_selector_matches(expedition_id.as_deref(), &existing.expedition_id) {
                    response.reason = Some("That expedition is no longer current.".to_owned());
                    response.expedition = Some(existing.clone());
                } else if !expedition_has_member(&existing, &account_id) {
                    response.reason =
                        Some("Join the pioneer party before resolving it.".to_owned());
                    response.expedition = Some(existing.clone());
                } else if existing.status != ExpeditionStatus::Launched {
                    response.reason =
                        Some("Launch a prepared expedition before resolving it.".to_owned());
                    response.expedition = Some(existing.clone());
                } else {
                    let (outpost_position, completed) = {
                        let expedition =
                            state.phase3.expedition.as_mut().expect("expedition exists");
                        if expedition_is_prepared(expedition, &self.config) {
                            expedition.status = ExpeditionStatus::Succeeded;
                            expedition.outcome = Some("Lantern Rest is founded; the party retreats only after the outpost is safe.".to_owned());
                        } else {
                            expedition.status = ExpeditionStatus::Retreated;
                            expedition.outcome = Some("The party retreats before founding the outpost; the road was understaffed by supplies, not by courage.".to_owned());
                        }
                        (expedition.outpost_position, expedition.clone())
                    };
                    if completed.status == ExpeditionStatus::Succeeded {
                        state.phase3.outpost = Some(outpost_position);
                        for member in &completed.members {
                            if !state
                                .phase3
                                .expedition_credentials
                                .contains(&member.account_id)
                            {
                                state
                                    .phase3
                                    .expedition_credentials
                                    .push(member.account_id.clone());
                            }
                        }
                    }
                    response.accepted = true;
                    response.expedition = Some(completed);
                    if response
                        .expedition
                        .as_ref()
                        .is_some_and(|expedition| expedition.status == ExpeditionStatus::Succeeded)
                    {
                        record(
                            &mut state,
                            "outpost founded",
                            "Lantern Rest joins the settlement chronicle",
                            "The pioneer party establishes a small outpost and all characters return safely.",
                        );
                    } else {
                        record(
                            &mut state,
                            "expedition retreated",
                            "The pioneer party returns before the outpost is founded",
                            "The expedition was staffed, but its supplies were not enough to establish a safe outpost.",
                        );
                    }
                }
            }
        }
        if response.accepted {
            if let Some(expedition) = &response.expedition {
                push_event(
                    &mut state,
                    WorldEvent::Frontier(FrontierEvent::Expedition(expedition.clone())),
                );
            }
        }
        state
            .phase3
            .request_results
            .insert(cache_key, Phase3Response::Expedition(response.clone()));
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }
}
