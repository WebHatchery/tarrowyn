use super::{
    is_transient_transport_error, NetworkNotice, WorldProjection, REQUEST_TIMEOUT_SECONDS,
};
use macroquad_toolkit::net::{HttpClient, Pending};
use std::collections::VecDeque;
use tarrowyn_protocol::{
    AdventurerContract, ApiResponse, ClaimAction, ClaimRequest, ClaimResponse, CombatAction,
    CombatRequest, CombatResponse, ContractAction, ContractRequest, ContractResponse,
    ContractStatus, ContractsResponse, Expedition, ExpeditionRequest, ExpeditionResponse,
    LandClaim, OpportunitiesResponse, RecoveryChoice, RecoveryRequest, RecoveryResponse,
    WeaponKind,
};

const MAX_COMMAND_RETRIES: u8 = 3;
const COMMAND_RETRY_DELAY_SECONDS: f32 = 1.0;

mod online;

fn contract_success_message(contract: &AdventurerContract) -> String {
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

#[derive(Clone)]
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
    pub(super) pending_chronicle_search:
        Option<Pending<ApiResponse<tarrowyn_protocol::ChronicleSearchResponse>>>,
    chronicle_search_request: Option<(String, u64)>,
    pub(super) pending_opportunities: Option<Pending<ApiResponse<OpportunitiesResponse>>>,
    pub(super) pending_command: Option<Pending<ApiResponse<FrontierCommandResponse>>>,
    in_flight_command: Option<FrontierCommand>,
    commands: VecDeque<FrontierCommand>,
    command_retry_timer: f32,
    command_retry_count: u8,
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
            pending_chronicle_search: None,
            chronicle_search_request: None,
            pending_opportunities: None,
            pending_command: None,
            in_flight_command: None,
            commands: VecDeque::new(),
            command_retry_timer: 0.0,
            command_retry_count: 0,
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
        self.command_retry_timer = (self.command_retry_timer - dt.max(0.0)).max(0.0);
        let mut cursor_boundary = false;
        if let Some(result) = self
            .pending_contracts
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        {
            self.pending_contracts = None;
            match result {
                Ok(response) => {
                    let cursor = response.meta.cursor.unwrap_or(response.data.cursor);
                    let current = projection.response_is_current(response.meta.server_tick, cursor);
                    projection.record_response_version(response.meta.server_tick, Some(cursor));
                    if current {
                        self.contracts = response.data.contracts;
                    }
                }
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
                    let cursor = response.meta.cursor.unwrap_or(response.data.cursor);
                    let current = projection.response_is_current(response.meta.server_tick, cursor);
                    projection.record_response_version(response.meta.server_tick, Some(cursor));
                    if current {
                        if response.data.summary.is_some() {
                            projection.chronicle_summary = response.data.summary;
                        }
                        for entry in response.data.entries {
                            super::merge_chronicle_entry(&mut projection.chronicle, entry);
                        }
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
            .pending_chronicle_search
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        {
            self.pending_chronicle_search = None;
            match result {
                Ok(response) => {
                    let cursor = response.meta.cursor.unwrap_or(response.data.cursor);
                    let current = projection.response_is_current(response.meta.server_tick, cursor);
                    projection.record_response_version(response.meta.server_tick, Some(cursor));
                    if current {
                        projection.chronicle_search = response.data.entries;
                        projection.chronicle_search_summary = response.data.summary;
                        projection.chronicle_search_query = Some(response.data.query);
                        projection.chronicle_search_next_cursor = response.data.next_cursor;
                    }
                }
                Err(error) if super::cursor::is_cursor_recovery_error(&error) => {
                    cursor_boundary = true;
                }
                Err(error) => notices.push(NetworkNotice::Warning(refresh_error_notice(
                    "chronicle archive search",
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
                Ok(response) => {
                    let cursor = response.meta.cursor.unwrap_or(response.data.cursor);
                    let current = projection.response_is_current(response.meta.server_tick, cursor);
                    projection.record_response_version(response.meta.server_tick, Some(cursor));
                    if current {
                        projection.opportunities = response.data.opportunities;
                    }
                }
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
            let in_flight_command = self.in_flight_command.take();
            match result {
                Ok(response) => {
                    self.command_retry_timer = 0.0;
                    self.command_retry_count = 0;
                    let cursor = response.meta.cursor.unwrap_or(projection.cursor);
                    let projection_current =
                        projection.response_is_current(response.meta.server_tick, cursor);
                    projection
                        .record_response_version(response.meta.server_tick, response.meta.cursor);
                    self.apply_command(response.data, projection, notices, projection_current);
                }
                Err(error)
                    if is_transient_transport_error(&error)
                        && self.command_retry_count < MAX_COMMAND_RETRIES
                        && in_flight_command.is_some() =>
                {
                    self.commands
                        .push_front(in_flight_command.expect("command exists"));
                    self.command_retry_count += 1;
                    self.command_retry_timer = COMMAND_RETRY_DELAY_SECONDS;
                    notices.push(NetworkNotice::Warning(format!(
                        "The frontier command could not be confirmed; retrying the same request ({}/{}). {}",
                        self.command_retry_count,
                        MAX_COMMAND_RETRIES,
                        short_error(&error)
                    )));
                }
                Err(error) => {
                    self.command_retry_count = 0;
                    notices.push(NetworkNotice::Warning(format!(
                        "The frontier command could not be confirmed: {}",
                        short_error(&error)
                    )));
                }
            }
        }
        cursor_boundary
    }

    pub(super) fn dispatch(
        &mut self,
        api: &mut HttpClient,
        online: bool,
        cursor: u64,
        auth_refresh_pending: bool,
    ) {
        if !online {
            return;
        }
        if auth_refresh_pending {
            return;
        }
        if self.pending_contracts.is_none() {
            self.pending_contracts = Some(api.get("/v1/contracts"));
        }
        if self.pending_chronicle.is_none() {
            self.pending_chronicle =
                Some(api.get(&format!("/v1/settlement/chronicle?since={cursor}")));
        }
        if self.pending_chronicle_search.is_none() {
            if let Some((query, since)) = self.chronicle_search_request.take() {
                let query = super::chronicle::encode_query_value(&query);
                self.pending_chronicle_search =
                    Some(api.get(&format!("/v1/chronicle/search?since={since}&q={query}")));
            }
        }
        if self.pending_opportunities.is_none() {
            self.pending_opportunities = Some(api.get("/v1/settlement/opportunities"));
        }
        if self.pending_command.is_none() && self.command_retry_timer <= 0.0 {
            if let Some(command) = self.commands.pop_front() {
                self.pending_command = Some(match &command {
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
                self.in_flight_command = Some(command);
            }
        }
    }

    pub(super) fn clear(&mut self) {
        self.contracts.clear();
        self.pending_contracts = None;
        self.pending_chronicle = None;
        self.pending_chronicle_search = None;
        self.chronicle_search_request = None;
        self.pending_opportunities = None;
        self.pending_command = None;
        self.in_flight_command = None;
        self.commands.clear();
        self.command_retry_timer = 0.0;
        self.command_retry_count = 0;
    }

    pub(super) fn has_pending_command(&self) -> bool {
        self.pending_command.is_some()
    }

    pub(super) fn recovery_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, FrontierCommand::Recovery(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, FrontierCommand::Recovery(_)))
    }

    pub(super) fn contract_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, FrontierCommand::Contract(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, FrontierCommand::Contract(_)))
    }

    pub(super) fn expedition_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, FrontierCommand::Expedition(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, FrontierCommand::Expedition(_)))
    }

    pub(super) fn combat_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, FrontierCommand::Combat(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, FrontierCommand::Combat(_)))
    }

    pub(super) fn claim_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, FrontierCommand::Claim(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, FrontierCommand::Claim(_)))
    }

    pub(super) fn queue_chronicle_search(&mut self, query: String, since: u64) {
        if self.pending_chronicle_search.is_none() {
            self.chronicle_search_request = Some((query, since));
        }
    }

    pub(super) fn chronicle_search_pending(&self) -> bool {
        self.pending_chronicle_search.is_some() || self.chronicle_search_request.is_some()
    }

    pub(super) fn queue_contract(&mut self, request_id: String, action: ContractAction) -> bool {
        if self.contract_command_pending()
            && self.commands.len() < super::queue::MAX_PENDING_COMMANDS
        {
            return false;
        }
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
        if self.combat_command_pending() && self.commands.len() < super::queue::MAX_PENDING_COMMANDS
        {
            return false;
        }
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
        if self.recovery_command_pending()
            && self.commands.len() < super::queue::MAX_PENDING_COMMANDS
        {
            return false;
        }
        super::queue::try_push(
            &mut self.commands,
            FrontierCommand::Recovery(RecoveryRequest { request_id, choice }),
        )
    }

    pub(super) fn queue_claim(&mut self, request_id: String, action: ClaimAction) -> bool {
        if self.claim_command_pending() && self.commands.len() < super::queue::MAX_PENDING_COMMANDS
        {
            return false;
        }
        super::queue::try_push(
            &mut self.commands,
            FrontierCommand::Claim(ClaimRequest { request_id, action }),
        )
    }

    pub(super) fn queue_expedition(&mut self, request: ExpeditionRequest) -> bool {
        if self.expedition_command_pending()
            && self.commands.len() < super::queue::MAX_PENDING_COMMANDS
        {
            return false;
        }
        super::queue::try_push(&mut self.commands, FrontierCommand::Expedition(request))
    }

    fn apply_command(
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
                        self.contracts = vec![response.contract.clone()];
                        projection.player = Some(response.player);
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

fn expedition_notice(response: &ExpeditionResponse, notices: &mut Vec<NetworkNotice>) {
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

fn homestead_success_message(claim: Option<&LandClaim>) -> String {
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

fn apply_recovery(
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

fn command_notice(
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

#[cfg(test)]
mod tests;
