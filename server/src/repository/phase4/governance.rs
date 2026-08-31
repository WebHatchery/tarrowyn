use super::{
    account_id, account_name, cache_key, record, validate_optional_identifier, validate_request_id,
};
use tarrowyn_protocol::{
    ApiResponse, GovernanceAction, GovernanceRequest, GovernanceResponse, OfficeKind,
    ProposalStatus, PublicAction, PublicProposal,
};

mod tick;
pub(super) use tick::tick;

const MAX_TAX_RATE_PERCENT: u8 = 10;
const MAX_PROPOSAL_TARGET_CHARS: usize = 80;

fn validate_proposal_target(
    target: Option<&str>,
) -> Result<Option<String>, super::super::RepositoryError> {
    let Some(target) = target else {
        return Ok(None);
    };
    let target = target.trim();
    if target.is_empty() {
        return Ok(None);
    }
    if target.chars().count() > MAX_PROPOSAL_TARGET_CHARS || target.chars().any(char::is_control) {
        return Err(super::super::RepositoryError::new(
            400,
            "invalid_proposal_target",
            "The proposal target must be at most 80 characters and contain no control characters.",
        ));
    }
    Ok(Some(target.to_owned()))
}

impl super::super::WorldRepository {
    pub fn governance(
        &self,
        token: &str,
        request: GovernanceRequest,
    ) -> Result<ApiResponse<GovernanceResponse>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let key = super::super::authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let office_id = validate_optional_identifier(
            request.office_id.as_deref(),
            "invalid_office_id",
            "An office selector must be bounded and contain no control characters.",
        )?;
        let proposal_id = validate_optional_identifier(
            request.proposal_id.as_deref(),
            "invalid_proposal_id",
            "A proposal selector must be bounded and contain no control characters.",
        )?;
        let requested_target = if request.action == GovernanceAction::Propose {
            validate_proposal_target(request.target.as_deref())?
        } else {
            None
        };
        if request.action == GovernanceAction::Inspect {
            return Ok(ApiResponse {
                meta: super::super::meta(
                    state.tick,
                    Some(request.request_id.clone()),
                    Some(state.cursor),
                ),
                data: GovernanceResponse {
                    request_id: request.request_id,
                    accepted: true,
                    governance: state.phase4.governance.clone(),
                    reason: None,
                },
            });
        }
        let cache = cache_key(&account_id(&state, &key), &request.request_id);
        if let Some(super::Phase4Response::Governance(response)) =
            state.phase4.request_results.get(&cache)
        {
            return Ok(ApiResponse {
                meta: super::super::meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }

        let actor_id = account_id(&state, &key);
        let actor_name = account_name(&state, &key);
        let mut response = GovernanceResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            governance: state.phase4.governance.clone(),
            reason: None,
        };
        match request.action {
            GovernanceAction::Inspect => {
                unreachable!("inspection returned before command handling")
            }
            GovernanceAction::ClaimOffice => {
                let office_id = office_id.as_deref().unwrap_or("steward").to_owned();
                let tick = state.tick;
                let Some(office) = state
                    .phase4
                    .governance
                    .offices
                    .iter_mut()
                    .find(|office| office.office_id == office_id)
                else {
                    response.reason =
                        Some("That office is not recorded in the town hall.".to_owned());
                    response.governance = state.phase4.governance.clone();
                    return finish(self, &mut state, cache, &request, response);
                };
                if office
                    .holder_account_id
                    .as_deref()
                    .is_some_and(|holder| holder != actor_id)
                {
                    response.reason =
                        Some("That office is occupied; inspect its authority first.".to_owned());
                } else {
                    office.holder_account_id = Some(actor_id.clone());
                    office.holder_name = Some(actor_name.clone());
                    office.last_active_tick = tick;
                    office.vacant = false;
                    office.vacancy_reason = None;
                    response.accepted = true;
                    record(
                        &mut state,
                        "office filled",
                        "A town-hall office finds a responsible hand",
                        &format!("{actor_name} now holds the {office_id} office."),
                    );
                }
            }
            GovernanceAction::SetTaxRate => {
                let Some(rate) = request.tax_rate_percent else {
                    response.reason = Some("Name the new public tax rate.".to_owned());
                    response.governance = state.phase4.governance.clone();
                    return finish(self, &mut state, cache, &request, response);
                };
                if !holds_office(&state, OfficeKind::Steward, &actor_id) {
                    response.reason = Some(
                        "Only the Settlement Steward may set the public settlement tax.".to_owned(),
                    );
                } else if rate > MAX_TAX_RATE_PERCENT {
                    response.reason = Some(format!(
                        "The public tax rate must stay between 0% and {MAX_TAX_RATE_PERCENT}%.",
                    ));
                } else {
                    let mut policy = state
                        .phase4
                        .governance
                        .taxation
                        .clone()
                        .unwrap_or_else(super::default_tax_policy);
                    let previous_rate = policy.rate_percent;
                    policy.rate_percent = rate;
                    state.phase4.governance.taxation = Some(policy);
                    touch_office(&mut state, &actor_id);
                    response.accepted = true;
                    record(
                        &mut state,
                        "tax policy changed",
                        "The mayor posts a bounded settlement tax",
                        &format!(
                            "{actor_name} changed the Hearth tax from {previous_rate}% to {rate}% on nearby carried gold.",
                        ),
                    );
                }
            }
            GovernanceAction::Propose => {
                let Some(action) = request.public_action else {
                    response.reason =
                        Some("Choose the bounded public action before proposing it.".to_owned());
                    response.governance = state.phase4.governance.clone();
                    return finish(self, &mut state, cache, &request, response);
                };
                let cost = request.cost.unwrap_or_else(|| action.default_cost());
                if cost == 0 || cost > state.phase4.governance.public_treasury {
                    response.reason =
                        Some("The proposal cost must fit the public treasury.".to_owned());
                } else if !super::proposal_room(&mut state.phase4.governance) {
                    response.reason = Some(
                        "The town-hall proposal ledger is full; complete an existing proposal before adding another."
                            .to_owned(),
                    );
                } else {
                    let created_tick = state.tick;
                    let proposal_id = format!("public-work-{}", state.phase4.next_proposal_id);
                    state.phase4.next_proposal_id = state.phase4.next_proposal_id.saturating_add(1);
                    state.phase4.governance.proposals.push(PublicProposal {
                        proposal_id,
                        proposer_account_id: actor_id.clone(),
                        proposer_name: actor_name.clone(),
                        action,
                        target: requested_target.unwrap_or_else(|| action.label().to_owned()),
                        cost,
                        status: ProposalStatus::Proposed,
                        created_tick,
                        approved_by: None,
                        completed_tick: None,
                    });
                    response.accepted = true;
                    record(
                        &mut state,
                        "public proposal",
                        "The town hall receives a public proposal",
                        &format!(
                            "{actor_name} proposes to {} for {cost} public gold.",
                            action.label()
                        ),
                    );
                }
            }
            GovernanceAction::Approve => {
                let Some(proposal_id) = proposal_id.as_deref() else {
                    response.reason = Some("Name the proposal to approve.".to_owned());
                    response.governance = state.phase4.governance.clone();
                    return finish(self, &mut state, cache, &request, response);
                };
                if !holds_office(&state, OfficeKind::Steward, &actor_id) {
                    response.reason =
                        Some("Only the Settlement Steward may approve public spending.".to_owned());
                } else if let Some(proposal) = state
                    .phase4
                    .governance
                    .proposals
                    .iter_mut()
                    .find(|proposal| proposal.proposal_id == proposal_id)
                {
                    if proposal.status != ProposalStatus::Proposed {
                        response.reason =
                            Some("That proposal is no longer awaiting approval.".to_owned());
                    } else {
                        proposal.status = ProposalStatus::Approved;
                        proposal.approved_by = Some(actor_id.clone());
                        touch_office(&mut state, &actor_id);
                        response.accepted = true;
                        record(
                            &mut state,
                            "public approval",
                            "The town hall makes a costed decision",
                            &format!("{actor_name} approves proposal {proposal_id}."),
                        );
                    }
                } else {
                    response.reason =
                        Some("That proposal is not in the town-hall ledger.".to_owned());
                }
            }
            GovernanceAction::Complete => {
                let Some(proposal_id) = proposal_id.as_deref() else {
                    response.reason = Some("Name the approved proposal to complete.".to_owned());
                    response.governance = state.phase4.governance.clone();
                    return finish(self, &mut state, cache, &request, response);
                };
                let Some(index) = state
                    .phase4
                    .governance
                    .proposals
                    .iter()
                    .position(|proposal| proposal.proposal_id == proposal_id)
                else {
                    response.reason =
                        Some("That proposal is not in the town-hall ledger.".to_owned());
                    response.governance = state.phase4.governance.clone();
                    return finish(self, &mut state, cache, &request, response);
                };
                let proposal = state.phase4.governance.proposals[index].clone();
                if proposal.status != ProposalStatus::Approved {
                    response.reason = Some(
                        "An approved proposal must be completed before it changes a service."
                            .to_owned(),
                    );
                } else if !can_complete(&state, &actor_id, proposal.action) {
                    response.reason = Some(
                        "Your office does not have authority for that public action.".to_owned(),
                    );
                } else if proposal.action == PublicAction::CommissionPublicWork
                    && state
                        .phase4
                        .infrastructure
                        .iter()
                        .any(|record| record.infrastructure_id == "hearth-workshop")
                {
                    response.reason = Some(
                        "The Hearth public workshop has already been commissioned.".to_owned(),
                    );
                } else if state.phase4.governance.public_treasury < proposal.cost {
                    response.reason =
                        Some("The public treasury cannot cover that approved cost.".to_owned());
                } else {
                    let tick = state.tick;
                    let decision_id = format!("decision-{}", state.phase4.next_decision_id);
                    state.phase4.governance.public_treasury -= proposal.cost;
                    state.phase4.governance.proposals[index].status = ProposalStatus::Completed;
                    state.phase4.governance.proposals[index].completed_tick = Some(tick);
                    apply_action(&mut state, proposal.action, tick);
                    state
                        .phase4
                        .governance
                        .decisions
                        .push(tarrowyn_protocol::GovernanceDecision {
                            decision_id,
                            actor_account_id: actor_id.clone(),
                            actor_name: actor_name.clone(),
                            action: proposal.action,
                            proposal_id: proposal.proposal_id.clone(),
                            cost: proposal.cost,
                            service_affected: proposal.target.clone(),
                            created_tick: tick,
                        });
                    state.phase4.next_decision_id = state.phase4.next_decision_id.saturating_add(1);
                    super::retain_recent(
                        &mut state.phase4.governance.decisions,
                        super::MAX_GOVERNANCE_DECISIONS,
                    );
                    touch_office(&mut state, &actor_id);
                    response.accepted = true;
                    record(
                        &mut state,
                        "public action completed",
                        "A public resource reaches the service it promised",
                        &format!(
                            "{actor_name} completed {} for {} public gold; {} changed.",
                            proposal.action.label(),
                            proposal.cost,
                            proposal.target
                        ),
                    );
                }
            }
        }
        response.governance = state.phase4.governance.clone();
        finish(self, &mut state, cache, &request, response)
    }

    pub fn infrastructure(
        &self,
        token: &str,
    ) -> Result<ApiResponse<tarrowyn_protocol::InfrastructureResponse>, super::super::RepositoryError>
    {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        super::super::authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: super::super::meta(state.tick, None, Some(state.cursor)),
            data: tarrowyn_protocol::InfrastructureResponse {
                records: state.phase4.infrastructure.clone(),
                cursor: state.cursor,
            },
        })
    }
}

fn finish(
    repository: &super::super::WorldRepository,
    state: &mut super::super::models::RepositoryState,
    cache: String,
    request: &GovernanceRequest,
    response: GovernanceResponse,
) -> Result<ApiResponse<GovernanceResponse>, super::super::RepositoryError> {
    let actor = cache
        .strip_prefix("phase4:")
        .and_then(|value| value.split_once(':'))
        .map(|(account, _)| account)
        .unwrap_or("unknown-account")
        .to_owned();
    let request_id = request.request_id.clone();
    let target = match request.action {
        GovernanceAction::ClaimOffice => request
            .office_id
            .as_deref()
            .map(str::trim)
            .filter(|office_id| !office_id.is_empty())
            .unwrap_or("steward")
            .to_owned(),
        GovernanceAction::Approve | GovernanceAction::Complete => request
            .proposal_id
            .as_deref()
            .map(str::trim)
            .filter(|proposal_id| !proposal_id.is_empty())
            .unwrap_or(request_id.as_str())
            .to_owned(),
        GovernanceAction::Propose => response
            .accepted
            .then(|| response.governance.proposals.last())
            .flatten()
            .map(|proposal| proposal.proposal_id.clone())
            .unwrap_or_else(|| request_id.clone()),
        GovernanceAction::SetTaxRate => "tax-policy".to_owned(),
        GovernanceAction::Inspect => request_id.clone(),
    };
    super::super::phase6::audit_command(
        state,
        &actor,
        "governance.action",
        &target,
        response.accepted,
        "A bounded settlement-governance command was recorded in the audit stream.",
    );
    state
        .phase4
        .request_results
        .insert(cache, super::Phase4Response::Governance(response.clone()));
    super::super::record_command_outcome(state, response.accepted);
    repository.persist(state);
    Ok(ApiResponse {
        meta: super::super::meta(state.tick, Some(request_id), Some(state.cursor)),
        data: response,
    })
}

fn holds_office(
    state: &super::super::models::RepositoryState,
    kind: OfficeKind,
    account: &str,
) -> bool {
    state
        .phase4
        .governance
        .offices
        .iter()
        .any(|office| office.kind == kind && office.holder_account_id.as_deref() == Some(account))
}

fn touch_office(state: &mut super::super::models::RepositoryState, account: &str) {
    for office in &mut state.phase4.governance.offices {
        if office.holder_account_id.as_deref() == Some(account) {
            office.last_active_tick = state.tick;
        }
    }
}

fn can_complete(
    state: &super::super::models::RepositoryState,
    account: &str,
    action: PublicAction,
) -> bool {
    holds_office(state, OfficeKind::Steward, account)
        || match action {
            PublicAction::RepairRoad | PublicAction::CommissionPublicWork => {
                holds_office(state, OfficeKind::WorksWarden, account)
            }
            PublicAction::UpdateContractBoard => {
                holds_office(state, OfficeKind::Registrar, account)
            }
            PublicAction::FundService | PublicAction::HostFestival => false,
        }
}

fn apply_action(
    state: &mut super::super::models::RepositoryState,
    action: PublicAction,
    tick: u64,
) {
    match action {
        PublicAction::RepairRoad => {
            if let Some(road) = state
                .phase4
                .infrastructure
                .iter_mut()
                .find(|record| record.infrastructure_id == "north-road")
            {
                road.condition = 100;
                road.service_quality = 94;
                road.status = super::infrastructure_status(road.condition);
                road.last_maintained_tick = tick;
                road.failure_note = Some("A public repair crew has reopened the road.".to_owned());
            }
        }
        PublicAction::FundService => {
            state.phase4.governance.service_funding_until_tick = tick.saturating_add(24);
            if let Some(service) = state
                .phase4
                .infrastructure
                .iter_mut()
                .find(|record| record.infrastructure_id == "hearth-services")
            {
                service.condition = service.condition.max(80);
                service.service_quality = 92;
                service.last_maintained_tick = tick;
                service.status = super::infrastructure_status(service.condition);
            }
        }
        PublicAction::HostFestival => {
            for household in &mut state.phase4.households {
                household.demand = household.demand.saturating_add(12).min(100);
                household.service_quality = household.service_quality.saturating_add(8).min(100);
            }
        }
        PublicAction::CommissionPublicWork => {
            state.phase4.infrastructure.push(super::infrastructure(
                "hearth-workshop",
                "Public workshop",
                tarrowyn_protocol::InfrastructureKind::PublicBuilding,
                tarrowyn_protocol::Position { x: 7, y: 5 },
                100,
                2,
                88,
                "The new workshop is available to service orders and repairs.",
            ));
            super::retain_recent(
                &mut state.phase4.infrastructure,
                super::MAX_INFRASTRUCTURE_RECORDS,
            );
        }
        PublicAction::UpdateContractBoard => {
            state.phase4.governance.administration_quality = state
                .phase4
                .governance
                .administration_quality
                .saturating_add(4)
                .min(100);
        }
    }
}
