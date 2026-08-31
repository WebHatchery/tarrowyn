use super::*;
use tarrowyn_protocol::{
    AdventurerContract, CombatResponse, ContractStatus, Expedition, ExpeditionResponse, LandClaim,
    RecoveryResponse,
};

pub fn contract_success_message(contract: &AdventurerContract) -> String {
    match contract.status {
        ContractStatus::Accepted => format!(
            "{} accepted • progress {}/{}.",
            contract.title, contract.progress, contract.required_progress
        ),
        ContractStatus::Cooldown => format!(
            "{} reported • reward paid; available after beat {}.",
            contract.title, contract.available_at_tick
        ),
        ContractStatus::Available => {
            format!("{} is available in the tavern ledger.", contract.title)
        }
        ContractStatus::Completed => format!(
            "{} complete • progress {}/{}.",
            contract.title, contract.progress, contract.required_progress
        ),
    }
}

impl FrontierClient {
    pub(super) fn apply_command(
        &mut self,
        response: FrontierCommandResponse,
        projection: &mut WorldProjection,
        notices: &mut Vec<NetworkNotice>,
        projection_current: bool,
    ) {
        match response {
            FrontierCommandResponse::Contract(response) => {
                if response.accepted {
                    if projection_current {
                        let position = response.player.position;
                        self.contracts = vec![response.contract.clone()];
                        projection.player = Some(response.player);
                        projection.set_authoritative_player_position(
                            macroquad_toolkit::grid::TilePos::new(position.x, position.y),
                        );
                    }
                    notices.push(NetworkNotice::Success(contract_success_message(
                        &response.contract,
                    )));
                } else {
                    notices.push(NetworkNotice::Warning(response.reason.unwrap_or_else(
                        || "The frontier contract was not accepted.".to_owned(),
                    )));
                }
            }
            FrontierCommandResponse::Combat(response) => {
                apply_combat(response, projection, notices, projection_current)
            }
            FrontierCommandResponse::Recovery(response) => {
                apply_recovery(response, projection, notices, projection_current)
            }
            FrontierCommandResponse::Claim(response) => {
                let message = homestead_success_message(response.claim.as_ref());
                if projection_current {
                    projection.claim = response.claim;
                }
                command_notice(response.accepted, response.reason, &message, notices);
            }
            FrontierCommandResponse::Expedition(response) => {
                expedition_notice(&response, notices);
                if projection_current {
                    if let Some(expedition) = response.expedition {
                        projection.expedition = Some(expedition.clone());
                        projection.outpost = (expedition.status
                            == tarrowyn_protocol::ExpeditionStatus::Succeeded)
                            .then_some(macroquad_toolkit::grid::TilePos::new(
                                expedition.outpost_position.x,
                                expedition.outpost_position.y,
                            ));
                    }
                }
            }
        }
    }
}

pub fn expedition_notice(response: &ExpeditionResponse, notices: &mut Vec<NetworkNotice>) {
    if !response.accepted {
        notices.push(NetworkNotice::Warning(
            response
                .reason
                .clone()
                .unwrap_or_else(|| "The pioneer action was not accepted.".to_owned()),
        ));
        return;
    }
    let Some(expedition) = response.expedition.as_ref() else {
        notices.push(NetworkNotice::Success(
            "The pioneer registry updated.".to_owned(),
        ));
        return;
    };
    match expedition.status {
        tarrowyn_protocol::ExpeditionStatus::Planning => notices.push(NetworkNotice::Success(
            "The pioneer registry is gathering companions and supplies.".to_owned(),
        )),
        tarrowyn_protocol::ExpeditionStatus::Launched => notices.push(NetworkNotice::Success(
            "The staffed pioneer party has left for the frontier.".to_owned(),
        )),
        tarrowyn_protocol::ExpeditionStatus::Succeeded => {
            notices.push(NetworkNotice::Success(format!(
                "{} is founded. {}",
                expedition.outpost_name,
                outcome_text(expedition)
            )))
        }
        tarrowyn_protocol::ExpeditionStatus::Retreated => {
            notices.push(NetworkNotice::Info(format!(
                "The pioneer party retreated before founding {}. {}",
                expedition.outpost_name,
                outcome_text(expedition)
            )))
        }
    }
}

fn outcome_text(expedition: &Expedition) -> &str {
    expedition
        .outcome
        .as_deref()
        .unwrap_or("The registry recorded the result.")
}

pub fn homestead_success_message(claim: Option<&LandClaim>) -> String {
    let Some(claim) = claim else {
        return "The homestead ledger updated.".to_owned();
    };
    let plot = format!("({}, {})", claim.position.x, claim.position.y);
    match claim.status {
        tarrowyn_protocol::ClaimStatus::Active => format!(
            "Homestead lease active at plot {plot}; {}-day access is recognised.",
            claim.lease_days
        ),
        tarrowyn_protocol::ClaimStatus::Abandoned => format!(
            "Homestead lease abandoned at plot {plot}; reclamation opens after {} inactive beats.",
            claim.reclaim_after_ticks
        ),
        tarrowyn_protocol::ClaimStatus::Reclaimed => {
            "Homestead lease reclaimed; tap Claim to request a new lease.".to_owned()
        }
    }
}

pub fn short_error(error: &str) -> String {
    error
        .lines()
        .next()
        .unwrap_or(error)
        .chars()
        .take(100)
        .collect()
}

pub fn refresh_error_notice(label: &str, error: &str) -> String {
    format!(
        "The {label} could not be refreshed; reconnect or tap the visible control to retry. {}",
        short_error(error)
    )
}

pub fn apply_combat(
    response: CombatResponse,
    projection: &mut WorldProjection,
    notices: &mut Vec<NetworkNotice>,
    projection_current: bool,
) {
    if projection_current {
        let position = response.player.position;
        projection.player = Some(response.player);
        projection.set_authoritative_player_position(macroquad_toolkit::grid::TilePos::new(
            position.x, position.y,
        ));
        projection.wilderness = Some(response.zone);
    }
    match response.outcome {
        Some(tarrowyn_protocol::CombatOutcome::Victory) => notices.push(NetworkNotice::Success(
            "The Brambleback falls; the north road is open.".to_owned(),
        )),
        Some(tarrowyn_protocol::CombatOutcome::KnockedOut) => notices.push(NetworkNotice::Danger(
            response.recovery_prompt.unwrap_or_else(|| {
                "You were knocked out; tap Self, Rescuer, or Healer to recover.".to_owned()
            }),
        )),
        Some(tarrowyn_protocol::CombatOutcome::Retreated) => notices.push(NetworkNotice::Info(
            "You retreat to the Hearth before the threat closes in.".to_owned(),
        )),
        None => command_notice(
            response.accepted,
            response.reason,
            "The combat intent was accepted.",
            notices,
        ),
    }
}

pub fn apply_recovery(
    response: RecoveryResponse,
    projection: &mut WorldProjection,
    notices: &mut Vec<NetworkNotice>,
    projection_current: bool,
) {
    if projection_current {
        let position = response.player.position;
        projection.player = Some(response.player);
        projection.set_authoritative_player_position(macroquad_toolkit::grid::TilePos::new(
            position.x, position.y,
        ));
    }
    command_notice(
        response.accepted,
        response.reason,
        &response.consequence,
        notices,
    );
}

pub fn command_notice(
    accepted: bool,
    reason: Option<String>,
    success: &str,
    notices: &mut Vec<NetworkNotice>,
) {
    if accepted {
        notices.push(NetworkNotice::Success(success.to_owned()));
    } else {
        notices.push(NetworkNotice::Warning(reason.unwrap_or_else(|| {
            "The frontier action was not accepted.".to_owned()
        })));
    }
}
