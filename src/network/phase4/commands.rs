use super::feedback::{
    claim_success_message, governance_success_message, knowledge_success_message,
    profession_success_message,
};
use super::polling::{accept_projection_cursor, phase4_notice};
use super::{NetworkNotice, Phase4Client, Phase4Command, Phase4CommandResponse};

impl Phase4Client {
    pub(super) fn apply_command(
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
}
