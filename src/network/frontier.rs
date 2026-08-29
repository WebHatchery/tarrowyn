use super::{NetworkNotice, OnlineClient, WorldProjection, REQUEST_TIMEOUT_SECONDS};
use macroquad_toolkit::net::{HttpClient, Pending};
use std::collections::VecDeque;
use tarrowyn_protocol::{
    AdventurerContract, ApiResponse, ClaimAction, ClaimRequest, ClaimResponse, CombatAction,
    CombatRequest, CombatResponse, ContractAction, ContractRequest, ContractResponse,
    ContractStatus, ContractsResponse, ExpeditionAction, ExpeditionRequest, ExpeditionResponse,
    ExpeditionRole, OpportunitiesResponse, RecoveryChoice, RecoveryRequest, RecoveryResponse,
    WeaponKind,
};

enum FrontierCommand {
    Contract(ContractRequest),
    Combat(CombatRequest),
    Recovery(RecoveryRequest),
    Claim(ClaimRequest),
    Expedition(ExpeditionRequest),
}

pub(super) struct FrontierClient {
    pub(super) contracts: Vec<AdventurerContract>,
    pub(super) pending_contracts: Option<Pending<ApiResponse<ContractsResponse>>>,
    pub(super) pending_chronicle:
        Option<Pending<ApiResponse<tarrowyn_protocol::ChronicleResponse>>>,
    pub(super) pending_opportunities: Option<Pending<ApiResponse<OpportunitiesResponse>>>,
    pub(super) pending_command: Option<Pending<ApiResponse<FrontierCommandResponse>>>,
    commands: VecDeque<FrontierCommand>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub(super) enum FrontierCommandResponse {
    Contract(ContractResponse),
    Combat(CombatResponse),
    Recovery(RecoveryResponse),
    Claim(ClaimResponse),
    Expedition(ExpeditionResponse),
}

impl FrontierClient {
    pub(super) fn new() -> Self {
        Self {
            contracts: Vec::new(),
            pending_contracts: None,
            pending_chronicle: None,
            pending_opportunities: None,
            pending_command: None,
            commands: VecDeque::new(),
        }
    }

    pub(super) fn update(
        &mut self,
        projection: &mut WorldProjection,
        dt: f32,
        online: bool,
        notices: &mut Vec<NetworkNotice>,
    ) -> bool {
        if !online {
            return false;
        }
        let mut cursor_boundary = false;
        if let Some(result) = self
            .pending_contracts
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        {
            self.pending_contracts = None;
            match result {
                Ok(response) => self.contracts = response.data.contracts,
                Err(error) => notices.push(NetworkNotice::Warning(refresh_error_notice(
                    "tavern contracts",
                    &error,
                ))),
            }
        }
        if let Some(result) = self
            .pending_chronicle
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        {
            self.pending_chronicle = None;
            match result {
                Ok(response) => {
                    if response.data.summary.is_some() {
                        projection.chronicle_summary = response.data.summary;
                    }
                    for entry in response.data.entries {
                        super::merge_chronicle_entry(&mut projection.chronicle, entry);
                    }
                }
                Err(error) if super::cursor::is_cursor_recovery_error(&error) => {
                    cursor_boundary = true;
                }
                Err(error) => notices.push(NetworkNotice::Warning(refresh_error_notice(
                    "settlement chronicle",
                    &error,
                ))),
            }
        }
        if let Some(result) = self
            .pending_opportunities
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        {
            self.pending_opportunities = None;
            match result {
                Ok(response) => projection.opportunities = response.data.opportunities,
                Err(error) => notices.push(NetworkNotice::Warning(refresh_error_notice(
                    "frontier opportunities",
                    &error,
                ))),
            }
        }
        if let Some(result) = self
            .pending_command
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        {
            self.pending_command = None;
            match result {
                Ok(response) => self.apply_command(response.data, projection, notices),
                Err(error) => notices.push(NetworkNotice::Warning(format!(
                    "The frontier command could not be confirmed; tap the visible action to retry. {}",
                    short_error(&error)
                ))),
            }
        }
        cursor_boundary
    }

    pub(super) fn dispatch(&mut self, api: &mut HttpClient, online: bool, cursor: u64) {
        if !online {
            return;
        }
        if self.pending_contracts.is_none() {
            self.pending_contracts = Some(api.get("/v1/contracts"));
        }
        if self.pending_chronicle.is_none() {
            self.pending_chronicle =
                Some(api.get(&format!("/v1/settlement/chronicle?since={cursor}")));
        }
        if self.pending_opportunities.is_none() {
            self.pending_opportunities = Some(api.get("/v1/settlement/opportunities"));
        }
        if self.pending_command.is_none() {
            if let Some(command) = self.commands.pop_front() {
                self.pending_command = Some(match command {
                    FrontierCommand::Contract(request) => {
                        api.post_json("/v1/contracts/brambleback-watch", &request)
                    }
                    FrontierCommand::Combat(request) => {
                        api.post_json("/v1/combat/actions", &request)
                    }
                    FrontierCommand::Recovery(request) => api.post_json("/v1/recovery", &request),
                    FrontierCommand::Claim(request) => api.post_json("/v1/claims", &request),
                    FrontierCommand::Expedition(request) => {
                        api.post_json("/v1/expeditions", &request)
                    }
                });
            }
        }
    }

    pub(super) fn clear(&mut self) {
        self.contracts.clear();
        self.pending_contracts = None;
        self.pending_chronicle = None;
        self.pending_opportunities = None;
        self.pending_command = None;
        self.commands.clear();
    }

    pub(super) fn queue_contract(&mut self, request_id: String, action: ContractAction) -> bool {
        super::queue::try_push(
            &mut self.commands,
            FrontierCommand::Contract(ContractRequest {
                request_id,
                action,
                contract_id: "brambleback-watch".to_owned(),
            }),
        )
    }

    pub(super) fn queue_combat(
        &mut self,
        request_id: String,
        action: CombatAction,
        weapon: WeaponKind,
    ) -> bool {
        super::queue::try_push(
            &mut self.commands,
            FrontierCommand::Combat(CombatRequest {
                request_id,
                action,
                weapon,
            }),
        )
    }

    pub(super) fn queue_recovery(&mut self, request_id: String, choice: RecoveryChoice) -> bool {
        super::queue::try_push(
            &mut self.commands,
            FrontierCommand::Recovery(RecoveryRequest { request_id, choice }),
        )
    }

    pub(super) fn queue_claim(&mut self, request_id: String, action: ClaimAction) -> bool {
        super::queue::try_push(
            &mut self.commands,
            FrontierCommand::Claim(ClaimRequest { request_id, action }),
        )
    }

    pub(super) fn queue_expedition(&mut self, request: ExpeditionRequest) -> bool {
        super::queue::try_push(&mut self.commands, FrontierCommand::Expedition(request))
    }

    fn apply_command(
        &mut self,
        response: FrontierCommandResponse,
        projection: &mut WorldProjection,
        notices: &mut Vec<NetworkNotice>,
    ) {
        match response {
            FrontierCommandResponse::Contract(response) => {
                if response.accepted {
                    self.contracts = vec![response.contract];
                    projection.player = Some(response.player);
                    notices.push(NetworkNotice::Success(
                        "The tavern ledger accepted the frontier contract.".to_owned(),
                    ));
                } else if let Some(reason) = response.reason {
                    notices.push(NetworkNotice::Warning(reason));
                }
            }
            FrontierCommandResponse::Combat(response) => {
                apply_combat(response, projection, notices)
            }
            FrontierCommandResponse::Recovery(response) => {
                apply_recovery(response, projection, notices)
            }
            FrontierCommandResponse::Claim(response) => {
                projection.claim = response.claim;
                command_notice(
                    response.accepted,
                    response.reason,
                    "The homestead ledger updated.",
                    notices,
                );
            }
            FrontierCommandResponse::Expedition(response) => {
                command_notice(
                    response.accepted,
                    response.reason,
                    "The pioneer registry updated.",
                    notices,
                );
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

fn short_error(error: &str) -> String {
    error
        .lines()
        .next()
        .unwrap_or(error)
        .chars()
        .take(100)
        .collect()
}

fn refresh_error_notice(label: &str, error: &str) -> String {
    format!(
        "The {label} could not be refreshed; reconnect or tap the visible control to retry. {}",
        short_error(error)
    )
}

fn apply_combat(
    response: CombatResponse,
    projection: &mut WorldProjection,
    notices: &mut Vec<NetworkNotice>,
) {
    projection.player = Some(response.player);
    projection.player_position = macroquad_toolkit::grid::TilePos::new(
        projection.player.as_ref().unwrap().position.x,
        projection.player.as_ref().unwrap().position.y,
    );
    projection.wilderness = Some(response.zone);
    match response.outcome {
        Some(tarrowyn_protocol::CombatOutcome::Victory) => notices.push(NetworkNotice::Success(
            "The Brambleback falls; the north road is open.".to_owned(),
        )),
        Some(tarrowyn_protocol::CombatOutcome::KnockedOut) => notices.push(NetworkNotice::Danger(
            response
                .recovery_prompt
                .unwrap_or_else(|| "You were knocked out; choose recovery.".to_owned()),
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

fn apply_recovery(
    response: RecoveryResponse,
    projection: &mut WorldProjection,
    notices: &mut Vec<NetworkNotice>,
) {
    projection.player = Some(response.player);
    command_notice(
        response.accepted,
        response.reason,
        &response.consequence,
        notices,
    );
}

fn command_notice(
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

impl OnlineClient {
    pub fn queue_contract_cycle(&mut self) {
        let action = match self.frontier.contracts.first() {
            Some(contract) if contract.status == ContractStatus::Cooldown => {
                self.status_message =
                    "The frontier contract is cooling down; return after its visible availability tick."
                        .to_owned();
                return;
            }
            Some(contract)
                if contract.status == ContractStatus::Accepted && contract.progress < 3 =>
            {
                ContractAction::Progress
            }
            Some(contract) if contract.status == ContractStatus::Accepted => ContractAction::Report,
            _ => ContractAction::Accept,
        };
        self.queue_contract(action);
    }

    pub fn queue_claim_cycle(&mut self) {
        let action = if self.projection.claim.as_ref().is_some_and(|claim| {
            self.account.as_ref().is_some_and(|account| {
                claim.owner_account_id == account.account_id
                    && claim.status == tarrowyn_protocol::ClaimStatus::Active
            })
        }) {
            ClaimAction::Renew
        } else {
            ClaimAction::Request
        };
        self.queue_claim(action);
    }

    pub fn queue_expedition_cycle(&mut self) {
        let (action, role) = match self.projection.expedition.as_ref() {
            None => (ExpeditionAction::Announce, Some(ExpeditionRole::Scout)),
            Some(expedition)
                if expedition.status == tarrowyn_protocol::ExpeditionStatus::Launched =>
            {
                (ExpeditionAction::Resolve, None)
            }
            Some(expedition)
                if matches!(
                    expedition.status,
                    tarrowyn_protocol::ExpeditionStatus::Succeeded
                        | tarrowyn_protocol::ExpeditionStatus::Retreated
                ) =>
            {
                (ExpeditionAction::Announce, Some(ExpeditionRole::Scout))
            }
            Some(expedition) => {
                let own = self
                    .account
                    .as_ref()
                    .map(|account| account.account_id.as_str());
                if !expedition
                    .members
                    .iter()
                    .any(|member| Some(member.account_id.as_str()) == own)
                {
                    let role = if !expedition
                        .members
                        .iter()
                        .any(|member| member.role == ExpeditionRole::Farmer)
                    {
                        ExpeditionRole::Farmer
                    } else {
                        ExpeditionRole::Builder
                    };
                    (ExpeditionAction::Join, Some(role))
                } else if expedition.food < 6
                    || expedition.tools < 3
                    || expedition.materials < 8
                    || expedition.safety < 3
                {
                    (ExpeditionAction::Supply, None)
                } else {
                    (ExpeditionAction::Launch, None)
                }
            }
        };
        self.queue_expedition(action, role);
    }

    pub fn queue_contract(&mut self, action: ContractAction) {
        if self.state == super::ConnectionState::Online {
            let request_id = self.next_request_id("contract");
            if !self.frontier.queue_contract(request_id, action) {
                self.status_message =
                    "That frontier action is not ready; wait for its ledger or queue to clear."
                        .to_owned();
            }
        }
    }

    pub fn queue_combat(&mut self, action: CombatAction, weapon: WeaponKind) {
        if self.state == super::ConnectionState::Online {
            let request_id = self.next_request_id("combat");
            if !self.frontier.queue_combat(request_id, action, weapon) {
                self.status_message =
                    "That frontier action is not ready; wait for its ledger or queue to clear."
                        .to_owned();
            }
        }
    }

    pub fn queue_recovery(&mut self, choice: RecoveryChoice) {
        if self.state == super::ConnectionState::Online {
            let request_id = self.next_request_id("recovery");
            if !self.frontier.queue_recovery(request_id, choice) {
                self.status_message =
                    "That frontier action is not ready; wait for its ledger or queue to clear."
                        .to_owned();
            }
        }
    }

    pub fn queue_claim(&mut self, action: ClaimAction) {
        if self.state == super::ConnectionState::Online {
            let request_id = self.next_request_id("claim");
            if !self.frontier.queue_claim(request_id, action) {
                self.status_message =
                    "That frontier action is not ready; wait for its ledger or queue to clear."
                        .to_owned();
            }
        }
    }

    pub fn queue_expedition(&mut self, action: ExpeditionAction, role: Option<ExpeditionRole>) {
        if self.state == super::ConnectionState::Online {
            let request_id = self.next_request_id("expedition");
            if !self.frontier.queue_expedition(ExpeditionRequest {
                request_id,
                action,
                expedition_id: Some("pioneer-1".to_owned()),
                role,
                food: if action == ExpeditionAction::Supply {
                    6
                } else {
                    0
                },
                tools: if action == ExpeditionAction::Supply {
                    3
                } else {
                    0
                },
                materials: if action == ExpeditionAction::Supply {
                    8
                } else {
                    0
                },
                safety: if action == ExpeditionAction::Supply {
                    3
                } else {
                    0
                },
                outpost_name: Some("Lantern Rest".to_owned()),
            }) {
                self.status_message =
                    "That frontier action is not ready; wait for its ledger or queue to clear."
                        .to_owned();
            }
        }
    }
}

#[cfg(test)]
mod tests;
