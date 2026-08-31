use super::{
    is_transient_transport_error, NetworkNotice, WorldProjection, REQUEST_TIMEOUT_SECONDS,
};
use macroquad_toolkit::net::{HttpClient, Pending};
use std::collections::VecDeque;
use tarrowyn_protocol::{
    AdventurerContract, ApiResponse, ClaimAction, ClaimRequest, ClaimResponse, CombatAction,
    CombatRequest, CombatResponse, ContractAction, ContractRequest, ContractResponse,
    ContractsResponse, ExpeditionRequest, ExpeditionResponse, OpportunitiesResponse,
    RecoveryChoice, RecoveryRequest, RecoveryResponse, WeaponKind,
};

const MAX_COMMAND_RETRIES: u8 = 3;
const COMMAND_RETRY_DELAY_SECONDS: f32 = 1.0;

mod feedback;
mod online;
#[cfg(test)]
use feedback::{
    apply_recovery, command_notice, contract_success_message, expedition_notice,
    homestead_success_message,
};
use feedback::{refresh_error_notice, short_error};
#[cfg(test)]
use tarrowyn_protocol::ContractStatus;

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
                    self.in_flight_command = in_flight_command;
                    self.command_retry_count += 1;
                    self.command_retry_timer = COMMAND_RETRY_DELAY_SECONDS;
                    notices.push(NetworkNotice::Warning(format!(
                        "The frontier action could not be confirmed; retrying the same action ({}/{}). {}",
                        self.command_retry_count,
                        MAX_COMMAND_RETRIES,
                        short_error(&error)
                    )));
                }
                Err(error) => {
                    self.command_retry_count = 0;
                    notices.push(NetworkNotice::Warning(format!(
                        "The frontier action could not be confirmed: {}",
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
        another_mutation_pending: bool,
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
        if self.pending_command.is_none()
            && !another_mutation_pending
            && self.command_retry_timer <= 0.0
        {
            let command = self
                .in_flight_command
                .take()
                .or_else(|| self.commands.pop_front());
            if let Some(command) = command {
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
            || self.in_flight_command.is_some()
            || !self.commands.is_empty()
    }

    pub(super) fn command_in_flight(&self) -> bool {
        self.pending_command.is_some() || self.in_flight_command.is_some()
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
}

#[cfg(test)]
mod tests;
