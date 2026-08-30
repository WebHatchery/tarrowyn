use super::phase5::Phase5Client;
use super::{is_transient_transport_error, NetworkNotice, REQUEST_TIMEOUT_SECONDS};
use macroquad_toolkit::net::{HttpClient, Pending};
use std::collections::VecDeque;
use tarrowyn_protocol::{
    ApiResponse, ClaimLifecycleAction, ClaimLifecycleRequest, ClaimsResponse, GovernanceAction,
    GovernanceRequest, GovernanceResponse, GovernanceState, HouseholdsResponse, KnowledgeAction,
    KnowledgeRequest, KnowledgeResponse, LocalCombatAction, LocalCombatRequest,
    LocalCombatResponse, LocalCombatState, ProfessionAction, ProfessionKind, ProfessionRequest,
    ProfessionResponse, ProfessionsResponse, SkillAction, SkillRequest, SkillResponse, SkillStatus,
    SkillsResponse, WeaponKind,
};

const MAX_COMMAND_RETRIES: u8 = 3;
const COMMAND_RETRY_DELAY_SECONDS: f32 = 1.0;

mod combat;
mod feedback;
mod lifecycle;
mod online;
mod polling;
mod recovery;
mod regional;
mod registry;
mod summary;
mod sync;

use combat::advance_crafting;
use feedback::{
    claim_success_message, governance_success_message, knowledge_success_message,
    profession_success_message,
};
use polling::{accept_projection_cursor, phase4_notice, poll_projection, short_error};

#[derive(Clone)]
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
    in_flight_command: Option<Phase4Command>,
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
    projection_cursor: u64,
    command_retry_timer: f32,
    command_retry_count: u8,
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
            in_flight_command: None,
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
            projection_cursor: 0,
            command_retry_timer: 0.0,
            command_retry_count: 0,
            regional: Phase5Client::new(),
        }
    }

    pub(super) fn set_account(&mut self, account_id: Option<&str>) {
        self.own_account_id = account_id.map(str::to_owned);
        self.regional.set_account(account_id);
    }

    pub(super) fn queue_cycle(&mut self, id: &str, request_id: String) -> bool {
        if ((id == "town-hall" || id == "tax-rate") && self.governance_command_pending())
            || (id == "registry" && self.claim_command_pending())
            || (id == "knowledge" && self.knowledge_command_pending())
            || (id == "order" && self.order_command_pending())
            || (id == "practice" && self.skill_command_pending())
            || (matches!(
                id,
                "local-fight" | "retreat" | "technique" | "guard" | "item" | "reposition" | "spell"
            ) && self.combat_command_pending())
        {
            return false;
        }
        let queue_len = self.commands.len();
        let refresh_households = id == "households";
        match id {
            "town-hall" => self.queue_governance(request_id),
            "registry" => self.queue_claim(request_id),
            "order" => self.queue_order(request_id),
            "knowledge" => {
                self.queue_knowledge(request_id, None);
            }
            "local-fight" => self.queue_combat(request_id),
            "retreat" => self.queue_combat_action(request_id, LocalCombatAction::Retreat),
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
        self.commands.len() > queue_len
            || refresh_households
            || (id == "order" && self.crafting.is_some())
    }

    pub(super) fn governance_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, Phase4Command::Governance(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, Phase4Command::Governance(_)))
    }

    pub(super) fn skill_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, Phase4Command::Skill(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, Phase4Command::Skill(_)))
    }

    pub(super) fn knowledge_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, Phase4Command::Knowledge(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, Phase4Command::Knowledge(_)))
    }

    pub(super) fn order_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, Phase4Command::Profession(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, Phase4Command::Profession(_)))
    }

    pub(super) fn combat_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, Phase4Command::Combat(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, Phase4Command::Combat(_)))
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
        if self.crafting.is_some() || self.order_command_pending() {
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

    fn next_knowledge_action(&self, target_account_id: Option<&str>) -> KnowledgeAction {
        let Some(response) = self.knowledge.as_ref() else {
            return KnowledgeAction::Discover;
        };
        let Some(item) = response
            .knowledge
            .items
            .iter()
            .find(|item| item.knowledge_id == "moonberry-tending")
        else {
            return KnowledgeAction::Discover;
        };
        if !response
            .knowledge
            .known_by_player
            .iter()
            .any(|id| id == "moonberry-tending")
        {
            KnowledgeAction::Discover
        } else if item.writable && !item.stored_in.contains("guild archive") {
            KnowledgeAction::Record
        } else if item.teachable && target_account_id.is_some() {
            KnowledgeAction::Teach
        } else {
            KnowledgeAction::Apply
        }
    }

    pub(super) fn knowledge_cycle_label(&self, has_target: bool) -> &'static str {
        match self.next_knowledge_action(has_target.then_some("target")) {
            KnowledgeAction::Discover => "Discover",
            KnowledgeAction::Record => "Record",
            KnowledgeAction::Teach => "Teach",
            KnowledgeAction::Apply => "Apply",
            KnowledgeAction::Inspect => "Inspect",
        }
    }

    pub(super) fn queue_knowledge(
        &mut self,
        request_id: String,
        target_account_id: Option<String>,
    ) -> bool {
        if self.knowledge_command_pending() {
            return false;
        }
        let action = self.next_knowledge_action(target_account_id.as_deref());
        let target_account_id = (action == KnowledgeAction::Teach)
            .then_some(target_account_id)
            .flatten();
        super::queue::try_push(
            &mut self.commands,
            Phase4Command::Knowledge(KnowledgeRequest {
                request_id,
                action,
                knowledge_id: Some("moonberry-tending".to_owned()),
                target_account_id,
            }),
        )
    }

    pub(super) fn queue_school(&mut self, request_id: String, target_account_id: String) -> bool {
        if self.skill_command_pending() {
            return false;
        }
        if let Some(lesson) = self.skills.as_ref().and_then(|skills| {
            let own = self.own_account_id.as_deref()?;
            skills.lessons.iter().find(|lesson| {
                lesson.learner_account_id == own && lesson.teacher_account_id == target_account_id
            })
        }) {
            return super::queue::try_push(
                &mut self.commands,
                Phase4Command::Skill(SkillRequest {
                    request_id,
                    action: SkillAction::CompleteLesson,
                    lesson_id: Some(lesson.lesson_id.clone()),
                    skill_id: Some(lesson.skill_id.clone()),
                    target_account_id: Some(lesson.teacher_account_id.clone()),
                }),
            );
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
        )
    }

    fn queue_skill_practice(&mut self, request_id: String) {
        let Some(skill_id) = self.skills.as_ref().and_then(|skills| {
            skills
                .skills
                .iter()
                .find(|skill| {
                    skill.depth == 1
                        && matches!(
                            skill.status,
                            SkillStatus::Available | SkillStatus::Practising
                        )
                })
                .map(|skill| skill.skill_id.clone())
        }) else {
            return;
        };
        self.queue_skill_practice_for(request_id, skill_id);
    }

    pub(super) fn queue_skill_practice_for(
        &mut self,
        request_id: String,
        skill_id: String,
    ) -> bool {
        if self.skill_command_pending() {
            return false;
        }
        let valid_choice = self.skills.as_ref().is_some_and(|skills| {
            skills.skills.iter().any(|skill| {
                skill.skill_id == skill_id
                    && skill.depth == 1
                    && matches!(
                        skill.status,
                        SkillStatus::Available | SkillStatus::Practising
                    )
            })
        });
        if !valid_choice {
            return false;
        }
        super::queue::try_push(
            &mut self.commands,
            Phase4Command::Skill(SkillRequest {
                request_id,
                action: SkillAction::Practice,
                lesson_id: None,
                skill_id: Some(skill_id),
                target_account_id: None,
            }),
        )
    }

    fn apply_command(
        &mut self,
        response: Phase4CommandResponse,
        response_cursor: Option<u64>,
        projection_current: bool,
        command: Option<&Phase4Command>,
        notices: &mut Vec<NetworkNotice>,
    ) {
        let current = projection_current
            && accept_projection_cursor(&mut self.projection_cursor, response_cursor);
        match response {
            Phase4CommandResponse::Governance(response) => {
                let request = command.and_then(|command| match command {
                    Phase4Command::Governance(request) => Some(request),
                    _ => None,
                });
                let message = governance_success_message(&response, request);
                if current {
                    self.governance = Some(response.governance);
                }
                phase4_notice(response.accepted, response.reason, &message, notices);
            }
            Phase4CommandResponse::Claim(response) => {
                let message = claim_success_message(response.claim.as_ref());
                if current {
                    self.claims = Some(response.claims);
                }
                phase4_notice(response.accepted, response.reason, &message, notices);
            }
            Phase4CommandResponse::Profession(response) => {
                let request = command.and_then(|command| match command {
                    Phase4Command::Profession(request) => Some(request),
                    _ => None,
                });
                let message = profession_success_message(response.order.as_ref(), request);
                if current {
                    self.professions = Some(response.professions);
                }
                phase4_notice(response.accepted, response.reason, &message, notices);
            }
            Phase4CommandResponse::Knowledge(response) => {
                let request = command.and_then(|command| match command {
                    Phase4Command::Knowledge(request) => Some(request),
                    _ => None,
                });
                let message = knowledge_success_message(&response, request);
                if current {
                    self.knowledge = Some(response.clone());
                }
                phase4_notice(response.accepted, response.reason, &message, notices);
            }
            Phase4CommandResponse::Combat(response) => {
                if current {
                    self.combat = Some(response.combat);
                }
                phase4_notice(
                    response.accepted,
                    response.reason,
                    &response.prompt,
                    notices,
                );
            }
            Phase4CommandResponse::Skill(response) => {
                let message = response.message.clone();
                if current {
                    self.skills = Some(response.skills);
                }
                phase4_notice(response.accepted, response.reason, &message, notices);
            }
        }
    }

    pub(super) fn summary(&self) -> String {
        summary::render(self)
    }
}

#[cfg(test)]
mod tests;
