use super::phase5::Phase5Client;
use super::{NetworkNotice, OnlineClient, REQUEST_TIMEOUT_SECONDS};
use macroquad_toolkit::net::{HttpClient, Pending};
use std::collections::VecDeque;
use tarrowyn_protocol::{
    ApiResponse, ClaimLifecycleAction, ClaimLifecycleRequest, ClaimsResponse, GovernanceAction,
    GovernanceRequest, GovernanceResponse, GovernanceState, HouseholdsResponse, KnowledgeAction,
    KnowledgeRequest, KnowledgeResponse, LocalCombatAction, LocalCombatRequest,
    LocalCombatResponse, LocalCombatState, ProfessionAction, ProfessionKind, ProfessionRequest,
    ProfessionResponse, ProfessionsResponse, SkillStatus, SkillsResponse,
};

enum Phase4Command {
    Governance(GovernanceRequest),
    Claim(ClaimLifecycleRequest),
    Profession(ProfessionRequest),
    Knowledge(KnowledgeRequest),
    Combat(LocalCombatRequest),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum Phase4CommandResponse {
    Governance(GovernanceResponse),
    Claim(tarrowyn_protocol::ClaimLifecycleResponse),
    Profession(ProfessionResponse),
    Knowledge(KnowledgeResponse),
    Combat(LocalCombatResponse),
}

pub(super) struct Phase4Client {
    pending_governance: Option<Pending<ApiResponse<GovernanceResponse>>>,
    pending_claims: Option<Pending<ApiResponse<ClaimsResponse>>>,
    pending_professions: Option<Pending<ApiResponse<ProfessionsResponse>>>,
    pending_knowledge: Option<Pending<ApiResponse<KnowledgeResponse>>>,
    pending_skills: Option<Pending<ApiResponse<SkillsResponse>>>,
    pending_households: Option<Pending<ApiResponse<HouseholdsResponse>>>,
    pending_combat: Option<Pending<ApiResponse<LocalCombatState>>>,
    pending_command: Option<Pending<ApiResponse<Phase4CommandResponse>>>,
    commands: VecDeque<Phase4Command>,
    governance: Option<GovernanceState>,
    claims: Option<ClaimsResponse>,
    professions: Option<ProfessionsResponse>,
    knowledge: Option<KnowledgeResponse>,
    skills: Option<SkillsResponse>,
    combat: Option<LocalCombatState>,
    own_account_id: Option<String>,
    regional: Phase5Client,
}

impl Phase4Client {
    pub(super) fn new() -> Self {
        Self {
            pending_governance: None,
            pending_claims: None,
            pending_professions: None,
            pending_knowledge: None,
            pending_skills: None,
            pending_households: None,
            pending_combat: None,
            pending_command: None,
            commands: VecDeque::new(),
            governance: None,
            claims: None,
            professions: None,
            knowledge: None,
            skills: None,
            combat: None,
            own_account_id: None,
            regional: Phase5Client::new(),
        }
    }

    pub(super) fn set_account(&mut self, account_id: Option<&str>) {
        self.own_account_id = account_id.map(str::to_owned);
        self.regional.set_account(account_id);
    }

    pub(super) fn update(
        &mut self,
        dt: f32,
        api: &mut HttpClient,
        online: bool,
        notices: &mut Vec<NetworkNotice>,
    ) {
        if !online {
            return;
        }
        poll_projection(
            &mut self.pending_governance,
            dt,
            |response| {
                self.governance = Some(response.data.governance);
            },
            notices,
            "town hall",
        );
        poll_projection(
            &mut self.pending_claims,
            dt,
            |response| {
                self.claims = Some(response.data);
            },
            notices,
            "land registry",
        );
        poll_projection(
            &mut self.pending_professions,
            dt,
            |response| {
                self.professions = Some(response.data);
            },
            notices,
            "profession ledger",
        );
        poll_projection(
            &mut self.pending_knowledge,
            dt,
            |response| {
                self.knowledge = Some(response.data);
            },
            notices,
            "knowledge archive",
        );
        poll_projection(
            &mut self.pending_skills,
            dt,
            |response| {
                self.skills = Some(response.data);
            },
            notices,
            "skill ledger",
        );
        poll_projection(
            &mut self.pending_households,
            dt,
            |_| {},
            notices,
            "household ledger",
        );
        poll_projection(
            &mut self.pending_combat,
            dt,
            |response| {
                self.combat = Some(response.data);
            },
            notices,
            "local combat ledger",
        );
        if let Some(result) = self
            .pending_command
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        {
            self.pending_command = None;
            match result {
                Ok(response) => self.apply_command(response.data, notices),
                Err(error) => notices.push(NetworkNotice::Warning(format!(
                    "The Phase 4 action timed out; tap the visible control again. {}",
                    short_error(&error)
                ))),
            }
        }
        self.dispatch(api);
        self.regional.update(dt, api, online, notices);
    }

    fn dispatch(&mut self, api: &mut HttpClient) {
        if self.pending_governance.is_none() {
            self.pending_governance = Some(api.get("/v1/settlement/governance"));
        }
        if self.pending_claims.is_none() {
            self.pending_claims = Some(api.get("/v1/claims"));
        }
        if self.pending_professions.is_none() {
            self.pending_professions = Some(api.get("/v1/professions"));
        }
        if self.pending_knowledge.is_none() {
            self.pending_knowledge = Some(api.get("/v1/knowledge"));
        }
        if self.pending_skills.is_none() {
            self.pending_skills = Some(api.get("/v1/skills"));
        }
        if self.pending_households.is_none() {
            self.pending_households = Some(api.get("/v1/households"));
        }
        if self.pending_combat.is_none() {
            self.pending_combat = Some(api.get("/v1/combat/local"));
        }
        if self.pending_command.is_none() {
            if let Some(command) = self.commands.pop_front() {
                self.pending_command = Some(match command {
                    Phase4Command::Governance(request) => {
                        api.post_json("/v1/settlement/governance", &request)
                    }
                    Phase4Command::Claim(request) => {
                        api.post_json("/v1/claims/lifecycle", &request)
                    }
                    Phase4Command::Profession(request) => {
                        api.post_json("/v1/professions/orders", &request)
                    }
                    Phase4Command::Knowledge(request) => api.post_json("/v1/knowledge", &request),
                    Phase4Command::Combat(request) => api.post_json("/v1/combat/local", &request),
                });
            }
        }
    }

    pub(super) fn queue_cycle(&mut self, id: &str, request_id: String) {
        match id {
            "town-hall" => self.queue_governance(request_id),
            "registry" => self.queue_claim(request_id),
            "order" => self.queue_order(request_id),
            "knowledge" => self.queue_knowledge(request_id),
            "local-fight" => self.queue_combat(request_id),
            "households" => self.pending_households = None,
            _ => {}
        }
    }

    fn queue_governance(&mut self, request_id: String) {
        let action = self.governance.as_ref().map(|governance| {
            let own = self.own_account_id.as_deref();
            let steward = governance
                .offices
                .iter()
                .find(|office| office.office_id == "steward");
            if steward.and_then(|office| office.holder_account_id.as_deref()) != own {
                GovernanceRequest {
                    request_id,
                    action: GovernanceAction::ClaimOffice,
                    office_id: Some("steward".to_owned()),
                    proposal_id: None,
                    public_action: None,
                    target: None,
                    cost: None,
                }
            } else if let Some(proposal) = governance.proposals.iter().find(|proposal| {
                !matches!(
                    proposal.status,
                    tarrowyn_protocol::ProposalStatus::Completed
                        | tarrowyn_protocol::ProposalStatus::Rejected
                )
            }) {
                let action = match proposal.status {
                    tarrowyn_protocol::ProposalStatus::Proposed => GovernanceAction::Approve,
                    tarrowyn_protocol::ProposalStatus::Approved => GovernanceAction::Complete,
                    _ => GovernanceAction::Inspect,
                };
                GovernanceRequest {
                    request_id,
                    action,
                    office_id: None,
                    proposal_id: Some(proposal.proposal_id.clone()),
                    public_action: None,
                    target: None,
                    cost: None,
                }
            } else {
                GovernanceRequest {
                    request_id,
                    action: GovernanceAction::Propose,
                    office_id: None,
                    proposal_id: None,
                    public_action: Some(tarrowyn_protocol::PublicAction::RepairRoad),
                    target: Some("North road safety".to_owned()),
                    cost: None,
                }
            }
        });
        if let Some(request) = action {
            self.commands.push_back(Phase4Command::Governance(request));
        }
    }

    fn queue_claim(&mut self, request_id: String) {
        let (action, claim_id) = match self.claims.as_ref().and_then(|claims| claims.claims.last())
        {
            None => (ClaimLifecycleAction::Request, None),
            Some(claim) => {
                let action = match claim.status {
                    tarrowyn_protocol::ClaimLifecycleStatus::Requested => {
                        ClaimLifecycleAction::Approve
                    }
                    tarrowyn_protocol::ClaimLifecycleStatus::Active
                    | tarrowyn_protocol::ClaimLifecycleStatus::Renewed
                    | tarrowyn_protocol::ClaimLifecycleStatus::Transferred
                    | tarrowyn_protocol::ClaimLifecycleStatus::Inherited => {
                        ClaimLifecycleAction::Renew
                    }
                    tarrowyn_protocol::ClaimLifecycleStatus::Abandoned
                    | tarrowyn_protocol::ClaimLifecycleStatus::Expired => {
                        ClaimLifecycleAction::Reclaim
                    }
                    tarrowyn_protocol::ClaimLifecycleStatus::Reclaimed => {
                        ClaimLifecycleAction::Request
                    }
                };
                (action, Some(claim.claim_id.clone()))
            }
        };
        self.commands
            .push_back(Phase4Command::Claim(ClaimLifecycleRequest {
                request_id,
                action,
                claim_id,
                target_account_id: None,
            }));
    }

    fn queue_order(&mut self, request_id: String) {
        let own = self.own_account_id.as_deref();
        let action = if self.professions.as_ref().is_none_or(|professions| {
            !professions
                .profiles
                .iter()
                .any(|profile| profile.profession == ProfessionKind::Carpenter)
        }) {
            Phase4Command::Profession(ProfessionRequest {
                request_id,
                action: ProfessionAction::LearnCapability,
                order_id: None,
                profession: Some(ProfessionKind::Carpenter),
                capability_id: None,
                service: None,
            })
        } else if let Some(order) = self.professions.as_ref().and_then(|professions| {
            professions.orders.iter().find(|order| {
                order.status == tarrowyn_protocol::ServiceOrderStatus::Open
                    && order.requester_account_id != own.unwrap_or_default()
            })
        }) {
            Phase4Command::Profession(ProfessionRequest {
                request_id,
                action: ProfessionAction::AcceptOrder,
                order_id: Some(order.order_id.clone()),
                profession: None,
                capability_id: None,
                service: None,
            })
        } else if let Some(order) = self.professions.as_ref().and_then(|professions| {
            professions.orders.iter().find(|order| {
                order.provider_account_id.as_deref() == own
                    && order.status == tarrowyn_protocol::ServiceOrderStatus::Accepted
            })
        }) {
            Phase4Command::Profession(ProfessionRequest {
                request_id,
                action: ProfessionAction::CompleteOrder,
                order_id: Some(order.order_id.clone()),
                profession: None,
                capability_id: None,
                service: None,
            })
        } else {
            Phase4Command::Profession(ProfessionRequest {
                request_id,
                action: ProfessionAction::CreateOrder,
                order_id: None,
                profession: Some(ProfessionKind::Carpenter),
                capability_id: None,
                service: Some("Repair a field tool for the next harvest".to_owned()),
            })
        };
        self.commands.push_back(action);
    }

    fn queue_knowledge(&mut self, request_id: String) {
        let known = self
            .knowledge
            .as_ref()
            .map(|response| {
                response
                    .knowledge
                    .known_by_player
                    .iter()
                    .any(|id| id == "moonberry-tending")
            })
            .unwrap_or(false);
        self.commands
            .push_back(Phase4Command::Knowledge(KnowledgeRequest {
                request_id,
                action: if known {
                    KnowledgeAction::Apply
                } else {
                    KnowledgeAction::Discover
                },
                knowledge_id: Some("moonberry-tending".to_owned()),
                target_account_id: None,
            }));
    }

    fn queue_combat(&mut self, request_id: String) {
        let action = match self.combat.as_ref().map(|combat| combat.status) {
            Some(tarrowyn_protocol::LocalCombatStatus::Engaged) => LocalCombatAction::Strike,
            Some(tarrowyn_protocol::LocalCombatStatus::KnockedOut) => LocalCombatAction::Retreat,
            _ => LocalCombatAction::Prepare,
        };
        self.commands
            .push_back(Phase4Command::Combat(LocalCombatRequest {
                request_id,
                action,
                weapon: tarrowyn_protocol::WeaponKind::IronSword,
            }));
    }

    fn apply_command(&mut self, response: Phase4CommandResponse, notices: &mut Vec<NetworkNotice>) {
        match response {
            Phase4CommandResponse::Governance(response) => {
                self.governance = Some(response.governance);
                phase4_notice(
                    response.accepted,
                    response.reason,
                    "The town-hall ledger recorded the public action.",
                    notices,
                );
            }
            Phase4CommandResponse::Claim(response) => {
                self.claims = Some(response.claims);
                phase4_notice(
                    response.accepted,
                    response.reason,
                    "The land registry updated the lease lifecycle.",
                    notices,
                );
            }
            Phase4CommandResponse::Profession(response) => {
                self.professions = Some(response.professions);
                phase4_notice(
                    response.accepted,
                    response.reason,
                    "The profession ledger recorded the order step.",
                    notices,
                );
            }
            Phase4CommandResponse::Knowledge(response) => {
                self.knowledge = Some(response.clone());
                phase4_notice(
                    response.accepted,
                    response.reason,
                    &response.message,
                    notices,
                );
            }
            Phase4CommandResponse::Combat(response) => {
                self.combat = Some(response.combat);
                phase4_notice(
                    response.accepted,
                    response.reason,
                    &response.prompt,
                    notices,
                );
            }
        }
    }

    pub(super) fn clear(&mut self) {
        self.pending_governance = None;
        self.pending_claims = None;
        self.pending_professions = None;
        self.pending_knowledge = None;
        self.pending_skills = None;
        self.pending_households = None;
        self.pending_combat = None;
        self.pending_command = None;
        self.commands.clear();
        self.regional.clear();
    }

    pub(super) fn summary(&self) -> String {
        let offices = self
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
        let registry = self
            .claims
            .as_ref()
            .map(|claims| format!("{} plots available", claims.available_plots.len()))
            .unwrap_or_else(|| "Registry loading".to_owned());
        let orders = self
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
        let knowledge = self
            .knowledge
            .as_ref()
            .map(|knowledge| {
                format!(
                    "{} lessons known",
                    knowledge.knowledge.known_by_player.len()
                )
            })
            .unwrap_or_else(|| "Knowledge loading".to_owned());
        let skills = self
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
        format!("{offices} • {registry}\n{orders} • {knowledge} • {skills}")
    }

    pub(super) fn queue_region_cycle(&mut self, id: &str) {
        self.regional.queue_cycle(id);
    }

    pub(super) fn region_summary(&self) -> String {
        self.regional.summary()
    }
}

fn poll_projection<T, F>(
    pending: &mut Option<Pending<ApiResponse<T>>>,
    dt: f32,
    apply: F,
    notices: &mut Vec<NetworkNotice>,
    label: &str,
) where
    T: serde::de::DeserializeOwned,
    F: FnOnce(ApiResponse<T>),
{
    if let Some(result) = pending
        .as_mut()
        .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
    {
        *pending = None;
        match result {
            Ok(response) => apply(response),
            Err(error) => notices.push(NetworkNotice::Warning(format!(
                "The Phase 4 {label} could not be refreshed: {}",
                short_error(&error)
            ))),
        }
    }
}

fn phase4_notice(
    accepted: bool,
    reason: Option<String>,
    success: &str,
    notices: &mut Vec<NetworkNotice>,
) {
    if accepted {
        notices.push(NetworkNotice::Success(success.to_owned()));
    } else if let Some(reason) = reason {
        notices.push(NetworkNotice::Warning(reason));
    }
}

fn short_error(error: &str) -> String {
    error
        .lines()
        .next()
        .unwrap_or(error)
        .chars()
        .take(100)
        .collect()
}

impl OnlineClient {
    pub(crate) fn queue_phase4(&mut self, id: &str) {
        if self.state == super::ConnectionState::Online {
            let request_id = self.next_request_id("phase4");
            self.phase4.queue_cycle(id, request_id);
        }
    }

    pub(crate) fn phase4_summary(&self) -> String {
        self.phase4.summary()
    }
}
