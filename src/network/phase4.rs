use super::phase5::Phase5Client;
use super::{NetworkNotice, OnlineClient, REQUEST_TIMEOUT_SECONDS};
use macroquad_toolkit::net::{HttpClient, Pending};
use std::collections::VecDeque;
use tarrowyn_protocol::{
    ApiResponse, AuthSession, ClaimLifecycleAction, ClaimLifecycleRequest, ClaimsResponse,
    GovernanceAction, GovernanceRequest, GovernanceResponse, GovernanceState, GuestSessionResponse,
    HouseholdsResponse, KnowledgeAction, KnowledgeRequest, KnowledgeResponse, LocalCombatAction,
    LocalCombatRequest, LocalCombatResponse, LocalCombatState, ProfessionAction, ProfessionKind,
    ProfessionRequest, ProfessionResponse, ProfessionsResponse, SkillAction, SkillRequest,
    SkillResponse, SkillStatus, SkillsResponse, WeaponKind,
};

mod combat;
mod lifecycle;
mod recovery;
mod registry;
mod summary;

enum Phase4Command {
    Governance(GovernanceRequest),
    Claim(ClaimLifecycleRequest),
    Profession(ProfessionRequest),
    Knowledge(KnowledgeRequest),
    Combat(LocalCombatRequest),
    Skill(SkillRequest),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum Phase4CommandResponse {
    Governance(GovernanceResponse),
    Claim(tarrowyn_protocol::ClaimLifecycleResponse),
    Profession(ProfessionResponse),
    Knowledge(KnowledgeResponse),
    Combat(LocalCombatResponse),
    Skill(SkillResponse),
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
    households: Option<HouseholdsResponse>,
    combat: Option<LocalCombatState>,
    crafting: Option<CraftingChallenge>,
    own_account_id: Option<String>,
    regional: Phase5Client,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CraftingView {
    pub(crate) progress: f32,
    pub(crate) target_start: f32,
    pub(crate) target_end: f32,
}

struct CraftingChallenge {
    order_id: String,
    progress: f32,
    direction: f32,
    target_start: f32,
    target_end: f32,
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
            households: None,
            combat: None,
            crafting: None,
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
        advance_crafting(&mut self.crafting, dt);
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
            |response| {
                self.households = Some(response.data);
            },
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
                    Phase4Command::Skill(request) => api.post_json("/v1/skills", &request),
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
            "technique" => self.queue_combat_action(request_id, LocalCombatAction::Technique),
            "guard" => self.queue_combat_action(request_id, LocalCombatAction::Guard),
            "item" => self.queue_combat_action(request_id, LocalCombatAction::UseItem),
            "reposition" => self.queue_combat_action(request_id, LocalCombatAction::Reposition),
            "spell" => self.queue_combat_action(request_id, LocalCombatAction::CastSpell),
            "practice" => {
                self.queue_skill_practice(request_id);
            }
            "tax-rate" => self.queue_tax_rate(request_id),
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
                    tax_rate_percent: None,
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
                    tax_rate_percent: None,
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
                    tax_rate_percent: None,
                }
            }
        });
        if let Some(request) = action {
            super::queue::try_push(&mut self.commands, Phase4Command::Governance(request));
        }
    }

    fn queue_tax_rate(&mut self, request_id: String) {
        let Some(governance) = self.governance.as_ref() else {
            return;
        };
        let current = governance
            .taxation
            .as_ref()
            .map(|policy| policy.rate_percent)
            .unwrap_or(0);
        let next = match current {
            0 => 5,
            5 => 10,
            _ => 0,
        };
        super::queue::try_push(
            &mut self.commands,
            Phase4Command::Governance(GovernanceRequest {
                request_id,
                action: GovernanceAction::SetTaxRate,
                office_id: None,
                proposal_id: None,
                public_action: None,
                target: None,
                cost: None,
                tax_rate_percent: Some(next),
            }),
        );
    }

    fn queue_order(&mut self, request_id: String) {
        if self.crafting.is_some() {
            return;
        }
        let own = self.own_account_id.as_deref();
        if self.professions.as_ref().is_some_and(|professions| {
            professions.orders.iter().any(|order| {
                own.is_some_and(|account_id| {
                    order.requester_account_id == account_id
                        && matches!(
                            order.status,
                            tarrowyn_protocol::ServiceOrderStatus::Open
                                | tarrowyn_protocol::ServiceOrderStatus::Accepted
                        )
                        && order.provider_account_id.as_deref() != Some(account_id)
                })
            })
        }) {
            return;
        }
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
                timing_score: None,
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
                timing_score: None,
            })
        } else if let Some(order_id) = self
            .professions
            .as_ref()
            .and_then(|professions| {
                professions.orders.iter().find(|order| {
                    order.provider_account_id.as_deref() == own
                        && order.status == tarrowyn_protocol::ServiceOrderStatus::Accepted
                })
            })
            .map(|order| order.order_id.clone())
        {
            self.begin_crafting(&order_id);
            return;
        } else {
            Phase4Command::Profession(ProfessionRequest {
                request_id,
                action: ProfessionAction::CreateOrder,
                order_id: None,
                profession: Some(ProfessionKind::Carpenter),
                capability_id: None,
                service: Some("Repair a field tool for the next harvest".to_owned()),
                timing_score: None,
            })
        };
        super::queue::try_push(&mut self.commands, action);
    }

    fn begin_crafting(&mut self, order_id: &str) {
        self.crafting = Some(CraftingChallenge {
            order_id: order_id.to_owned(),
            progress: 0.0,
            direction: 1.0,
            target_start: 0.38,
            target_end: 0.66,
        });
    }

    pub(super) fn crafting_view(&self) -> Option<(f32, f32, f32)> {
        self.crafting.as_ref().map(|challenge| {
            (
                challenge.progress,
                challenge.target_start,
                challenge.target_end,
            )
        })
    }

    pub(super) fn submit_crafting(&mut self, request_id: String) -> bool {
        let Some(challenge) = self.crafting.take() else {
            return false;
        };
        let center = (challenge.target_start + challenge.target_end) * 0.5;
        let distance = (challenge.progress - center).abs();
        let timing_score = (100.0 - distance * 140.0).clamp(0.0, 100.0) as u8;
        let request = Phase4Command::Profession(ProfessionRequest {
            request_id,
            action: ProfessionAction::CompleteOrder,
            order_id: Some(challenge.order_id.clone()),
            profession: None,
            capability_id: None,
            service: None,
            timing_score: Some(timing_score),
        });
        if !super::queue::try_push(&mut self.commands, request) {
            self.crafting = Some(challenge);
            return false;
        }
        true
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
        super::queue::try_push(
            &mut self.commands,
            Phase4Command::Knowledge(KnowledgeRequest {
                request_id,
                action: if known {
                    KnowledgeAction::Apply
                } else {
                    KnowledgeAction::Discover
                },
                knowledge_id: Some("moonberry-tending".to_owned()),
                target_account_id: None,
            }),
        );
    }

    pub(super) fn queue_school(&mut self, request_id: String, target_account_id: String) -> bool {
        if let Some(lesson) = self.skills.as_ref().and_then(|skills| {
            let own = self.own_account_id.as_deref()?;
            skills.lessons.iter().find(|lesson| {
                lesson.learner_account_id == own && lesson.teacher_account_id == target_account_id
            })
        }) {
            super::queue::try_push(
                &mut self.commands,
                Phase4Command::Skill(SkillRequest {
                    request_id,
                    action: SkillAction::CompleteLesson,
                    lesson_id: Some(lesson.lesson_id.clone()),
                    skill_id: Some(lesson.skill_id.clone()),
                    target_account_id: Some(lesson.teacher_account_id.clone()),
                }),
            );
            return true;
        }
        let Some(skill) = self.skills.as_ref().and_then(|skills| {
            skills.skills.iter().find(|skill| {
                skill.depth == 1 && skill.mastery >= 5 && skill.skill_id != "teaching"
            })
        }) else {
            return false;
        };
        super::queue::try_push(
            &mut self.commands,
            Phase4Command::Skill(SkillRequest {
                request_id,
                action: SkillAction::BeginLesson,
                lesson_id: None,
                skill_id: Some(skill.skill_id.clone()),
                target_account_id: Some(target_account_id),
            }),
        );
        true
    }

    fn queue_skill_practice(&mut self, request_id: String) {
        let Some(skill) = self.skills.as_ref().and_then(|skills| {
            skills
                .skills
                .iter()
                .find(|skill| skill.depth == 1 && skill.status == SkillStatus::Available)
        }) else {
            return;
        };
        super::queue::try_push(
            &mut self.commands,
            Phase4Command::Skill(SkillRequest {
                request_id,
                action: SkillAction::Practice,
                lesson_id: None,
                skill_id: Some(skill.skill_id.clone()),
                target_account_id: None,
            }),
        );
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
            Phase4CommandResponse::Skill(response) => {
                let message = response.message.clone();
                self.skills = Some(response.skills);
                phase4_notice(response.accepted, response.reason, &message, notices);
            }
        }
    }

    pub(super) fn summary(&self) -> String {
        summary::render(self)
    }

    pub(super) fn queue_region_cycle(&mut self, id: &str) {
        self.regional.queue_cycle(id);
    }

    pub(super) fn region_summary(&self) -> String {
        self.regional.summary()
    }

    pub(super) fn regional_inspection(&self) -> String {
        self.regional.inspection()
    }

    pub(super) fn regional_season(&self) -> Option<&str> {
        self.regional.season()
    }

    pub(super) fn regional_region(&self) -> Option<&tarrowyn_protocol::RegionSnapshot> {
        self.regional.region_snapshot()
    }

    pub(super) fn take_linked_account(
        &mut self,
        client_key: Option<&str>,
    ) -> Option<GuestSessionResponse> {
        self.regional.take_linked_account(client_key)
    }

    pub(super) fn take_logged_out(&mut self) -> bool {
        self.regional.take_logged_out()
    }

    pub(super) fn deletion_armed(&self) -> bool {
        self.regional.deletion_armed()
    }

    pub(super) fn take_refreshed_session(&mut self) -> Option<AuthSession> {
        self.regional.take_refreshed_session()
    }

    pub(super) fn storm_magic_unlocked(&self) -> bool {
        self.skills.as_ref().is_some_and(|skills| {
            skills.skills.iter().any(|skill| {
                skill.skill_id == "storm-magic" && skill.status == SkillStatus::Discovered
            })
        })
    }
}

#[cfg(test)]
mod tests;

fn advance_crafting(challenge: &mut Option<CraftingChallenge>, dt: f32) {
    let Some(challenge) = challenge else {
        return;
    };
    challenge.progress += dt.max(0.0) * 0.45 * challenge.direction;
    if challenge.progress >= 1.0 {
        challenge.progress = 1.0;
        challenge.direction = -1.0;
    } else if challenge.progress <= 0.0 {
        challenge.progress = 0.0;
        challenge.direction = 1.0;
    }
}

fn next_combat_weapon(current: Option<WeaponKind>) -> WeaponKind {
    match current {
        None => WeaponKind::IronSword,
        Some(WeaponKind::IronSword) => WeaponKind::Spear,
        Some(WeaponKind::Spear) => WeaponKind::Axe,
        Some(WeaponKind::Axe) => WeaponKind::Bow,
        Some(WeaponKind::Bow) => WeaponKind::Shield,
        Some(WeaponKind::Shield) => WeaponKind::ImprovisedClub,
        Some(WeaponKind::ImprovisedClub) => WeaponKind::IronSword,
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

    pub(crate) fn crafting_view(&self) -> Option<CraftingView> {
        self.phase4
            .crafting_view()
            .map(|(progress, target_start, target_end)| CraftingView {
                progress,
                target_start,
                target_end,
            })
    }

    pub(crate) fn combat_state(&self) -> Option<&LocalCombatState> {
        self.phase4.combat.as_ref()
    }

    pub(crate) fn storm_magic_unlocked(&self) -> bool {
        self.phase4.storm_magic_unlocked()
    }

    pub(crate) fn queue_crafting_timing(&mut self) {
        if self.state != super::ConnectionState::Online {
            return;
        }
        let request_id = self.next_request_id("craft");
        if self.phase4.submit_crafting(request_id) {
            self.status_message =
                "Crafting result sent; waiting for the workshop ledger…".to_owned();
        }
    }

    pub(crate) fn queue_skill_teach(&mut self, target_account_id: &str) -> bool {
        if self.state != super::ConnectionState::Online {
            return false;
        }
        let request_id = self.next_request_id("school");
        self.phase4
            .queue_school(request_id, target_account_id.to_owned())
    }
}
