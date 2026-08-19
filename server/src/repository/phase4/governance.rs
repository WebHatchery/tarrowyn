use super::{account_id, account_name, cache_key, record, validate_request_id};
use crate::config::ServerConfig;
use tarrowyn_protocol::{
    ApiResponse, GovernanceAction, GovernanceRequest, GovernanceResponse, OfficeKind,
    ProposalStatus, PublicAction, PublicProposal,
};

impl super::super::WorldRepository {
    pub fn governance(
        &self,
        token: &str,
        request: GovernanceRequest,
    ) -> Result<ApiResponse<GovernanceResponse>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        super::super::expire_sessions(&mut state, &self.config);
        let key = super::super::authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
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
            GovernanceAction::Inspect => response.accepted = true,
            GovernanceAction::ClaimOffice => {
                let office_id = request.office_id.as_deref().unwrap_or("steward").to_owned();
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
                    return finish(self, &mut state, cache, request.request_id, response);
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
            GovernanceAction::Propose => {
                let Some(action) = request.public_action else {
                    response.reason =
                        Some("Choose the bounded public action before proposing it.".to_owned());
                    response.governance = state.phase4.governance.clone();
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let cost = request.cost.unwrap_or_else(|| action.default_cost());
                if cost == 0 || cost > state.phase4.governance.public_treasury {
                    response.reason =
                        Some("The proposal cost must fit the public treasury.".to_owned());
                } else {
                    let created_tick = state.tick;
                    let proposal_id = format!("public-work-{}", state.phase4.next_proposal_id);
                    state.phase4.next_proposal_id += 1;
                    state.phase4.governance.proposals.push(PublicProposal {
                        proposal_id,
                        proposer_account_id: actor_id.clone(),
                        proposer_name: actor_name.clone(),
                        action,
                        target: request.target.unwrap_or_else(|| action.label().to_owned()),
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
                let Some(proposal_id) = request.proposal_id.as_deref() else {
                    response.reason = Some("Name the proposal to approve.".to_owned());
                    response.governance = state.phase4.governance.clone();
                    return finish(self, &mut state, cache, request.request_id, response);
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
                let Some(proposal_id) = request.proposal_id.as_deref() else {
                    response.reason = Some("Name the approved proposal to complete.".to_owned());
                    response.governance = state.phase4.governance.clone();
                    return finish(self, &mut state, cache, request.request_id, response);
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
                    return finish(self, &mut state, cache, request.request_id, response);
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
                    state.phase4.next_decision_id += 1;
                    state.phase4.governance.decisions.truncate(64);
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
        finish(self, &mut state, cache, request.request_id, response)
    }

    pub fn infrastructure(
        &self,
        token: &str,
    ) -> Result<ApiResponse<tarrowyn_protocol::InfrastructureResponse>, super::super::RepositoryError>
    {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        super::super::expire_sessions(&mut state, &self.config);
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
    request_id: String,
    response: GovernanceResponse,
) -> Result<ApiResponse<GovernanceResponse>, super::super::RepositoryError> {
    state
        .phase4
        .request_results
        .insert(cache, super::Phase4Response::Governance(response.clone()));
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
            state.phase4.infrastructure.truncate(32);
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

pub(super) fn tick(state: &mut super::super::models::RepositoryState, config: &ServerConfig) {
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
        let upkeep: u32 = state
            .phase4
            .infrastructure
            .iter()
            .map(|record| record.upkeep_per_day)
            .sum();
        let funded = state.phase4.governance.public_treasury >= upkeep;
        if funded {
            state.phase4.governance.public_treasury -= upkeep;
        }
        let mut failures = Vec::new();
        for record in &mut state.phase4.infrastructure {
            if funded {
                record.last_maintained_tick = state.tick;
            } else {
                record.condition = record.condition.saturating_sub(5);
                record.status = super::infrastructure_status(record.condition);
                if record.status == tarrowyn_protocol::InfrastructureStatus::Failed {
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
}
