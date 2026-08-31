//! Frame-polled online session and the client-side world projection.

use crate::data::GameConfig;
use crate::state::{TileKind, WorldState};
use macroquad_toolkit::grid::{FlatGrid, TilePos};
use macroquad_toolkit::net::{HttpClient, Pending};
use std::collections::VecDeque;
use tarrowyn_protocol::{
    ApiResponse, ChatMessage, ChatRequest, ChronicleEntry, ChronicleSummary, EventsResponse,
    Expedition, ExpeditionRequirements, FarmAnimal, FarmingAction, FarmingRequest, FrontierEvent,
    GuestSessionRequest, GuestSessionResponse, LandClaim, MovementIntent, OpportunitySignal,
    OpsHealthResponse, PlayerPresence, PlayerProjection, StateSnapshot, TavernFeedResponse,
    TimeOfDay, TradeAction, TradeOffer, TradeRequest, TradesResponse, WildernessZone, WorldClock,
    WorldEvent, WorldSnapshot, MAX_CHAT_MESSAGE_LENGTH,
};

const REQUEST_TIMEOUT_SECONDS: f32 = 6.0;

#[derive(Clone, Copy)]
pub(crate) struct MutationContext {
    pub(crate) online: bool,
    pub(crate) another_mutation_pending: bool,
    pub(crate) session_only: bool,
}

pub(super) fn is_transient_transport_error(error: &str) -> bool {
    error.contains("persistence_unavailable")
        || error.contains(" timed out after ")
        || (error.contains("HTTP request '") && error.contains("' failed:"))
}

mod chronicle;
mod commands;
mod cursor;
mod frontier;
mod input;
mod maintenance;
mod phase4;
mod phase5;
mod projection;
mod queue;
mod requests;
mod trade_client;
mod types;
mod world;

pub(super) use chronicle::merge_chronicle_entry;
use frontier::FrontierClient;
pub(crate) use phase4::CraftingView;
use phase4::Phase4Client;
use requests::{PendingChat, PendingFarming, PendingMovement, PendingTrade};
pub use types::{ConnectionState, NetworkNotice, RemotePlayer};

pub struct WorldProjection {
    pub world: WorldState,
    pub player_position: TilePos,
    player_position_authoritative: bool,
    pub players: Vec<RemotePlayer>,
    pub chat: Vec<ChatMessage>,
    pub day: u32,
    pub day_seconds: f32,
    pub day_length_seconds: f32,
    pub server_tick: u64,
    pub cursor: u64,
    pub player: Option<PlayerProjection>,
    pub animals: Vec<FarmAnimal>,
    pub feed: TavernFeedResponse,
    pub trades: Vec<TradeOffer>,
    pub wilderness: Option<WildernessZone>,
    pub chronicle: Vec<ChronicleEntry>,
    pub chronicle_summary: Option<ChronicleSummary>,
    pub chronicle_search: Vec<ChronicleEntry>,
    pub chronicle_search_summary: Option<ChronicleSummary>,
    pub chronicle_search_query: Option<String>,
    pub chronicle_search_next_cursor: Option<u64>,
    pub opportunities: Vec<OpportunitySignal>,
    pub claim: Option<LandClaim>,
    pub outpost: Option<macroquad_toolkit::grid::TilePos>,
    pub expedition: Option<Expedition>,
    pub expedition_requirements: ExpeditionRequirements,
}

pub struct OnlineClient {
    api: HttpClient,
    pub projection: WorldProjection,
    pub state: ConnectionState,
    pub status_message: String,
    pub client_key: Option<String>,
    pub account: Option<GuestSessionResponse>,
    pending_guest: Option<Pending<ApiResponse<GuestSessionResponse>>>,
    pending_state: Option<Pending<ApiResponse<StateSnapshot>>>,
    pending_ops_health: Option<Pending<ApiResponse<OpsHealthResponse>>>,
    maintenance_status: Option<String>,
    readiness_degraded: bool,
    pending_events: Option<Pending<ApiResponse<EventsResponse>>>,
    pending_movement: Option<PendingMovement>,
    pending_chat: Option<PendingChat>,
    pending_farming: Option<PendingFarming>,
    pending_trades: Option<Pending<ApiResponse<TradesResponse>>>,
    pending_trade: Option<PendingTrade>,
    movement_queue: VecDeque<MovementIntent>,
    chat_queue: VecDeque<ChatRequest>,
    farming_queue: VecDeque<FarmingRequest>,
    trade_queue: VecDeque<TradeRequest>,
    next_request_id: u64,
    retry_cooldown: f32,
    retry_count: u8,
    max_retry_count: u8,
    state_refresh: f32,
    had_world: bool,
    pub pending_request_type: Option<String>,
    pub pending_request_id: Option<String>,
    pub action_awaiting_confirmation: bool,
    pub trades: Vec<TradeOffer>,
    pending_trade_action: Option<TradeAction>,
    frontier: FrontierClient,
    phase4: Phase4Client,
    state_reload_pending: bool,
}

impl OnlineClient {
    pub fn new(server_url: &str, config: &GameConfig) -> Self {
        let client_key = std::env::var("TARROWYN_CLIENT_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty());
        let mut client = Self {
            api: HttpClient::new(server_url),
            projection: WorldProjection::new(config),
            state: ConnectionState::Connecting,
            status_message: "Contacting the development road…".to_owned(),
            client_key,
            account: None,
            pending_guest: None,
            pending_state: None,
            pending_ops_health: None,
            maintenance_status: None,
            readiness_degraded: false,
            pending_events: None,
            pending_movement: None,
            pending_chat: None,
            pending_farming: None,
            pending_trades: None,
            pending_trade: None,
            movement_queue: VecDeque::new(),
            chat_queue: VecDeque::new(),
            farming_queue: VecDeque::new(),
            trade_queue: VecDeque::new(),
            next_request_id: 1,
            retry_cooldown: 0.0,
            retry_count: 0,
            max_retry_count: 3,
            state_refresh: 0.0,
            had_world: false,
            pending_request_type: None,
            pending_request_id: None,
            action_awaiting_confirmation: false,
            trades: Vec::new(),
            pending_trade_action: None,
            frontier: FrontierClient::new(),
            phase4: Phase4Client::new(),
            state_reload_pending: false,
        };
        client.begin_guest(false);
        client
    }

    pub fn update(&mut self, dt: f32) -> Vec<NetworkNotice> {
        self.retry_cooldown = (self.retry_cooldown - dt.max(0.0)).max(0.0);
        self.state_refresh = (self.state_refresh - dt.max(0.0)).max(0.0);
        let mut notices = Vec::new();
        self.poll_guest(dt, &mut notices);
        self.poll_state(dt, &mut notices);
        maintenance::poll_ops_health(self, dt);
        self.poll_events(dt, &mut notices);
        self.poll_movement(dt, &mut notices);
        self.poll_chat(dt, &mut notices);
        self.poll_farming(dt, &mut notices);
        self.poll_trade_requests(dt, &mut notices);
        let mutations_ready = self.mutations_ready();
        let frontier_cursor_boundary =
            self.frontier
                .update(&mut self.projection, dt, mutations_ready, &mut notices);
        if frontier_cursor_boundary {
            cursor::recover_from_cursor_boundary(self, &mut notices);
        }
        self.sync_regional_player_location();
        let player_knocked_out = self
            .projection
            .player
            .as_ref()
            .is_some_and(|player| player.knocked_out);
        self.phase4.sync_combat_to_player_state(player_knocked_out);
        self.phase4.set_account(
            self.account
                .as_ref()
                .map(|account| account.account_id.as_str()),
        );
        let other_mutation_pending = self.frontier.has_pending_command()
            || self.general_mutation_pending()
            || self.pending_trade.is_some()
            || !self.trade_queue.is_empty();
        let mutations_ready = self.mutations_ready();
        let session_only = self.state == ConnectionState::Online && !mutations_ready;
        self.phase4.update_with_mode(
            dt,
            &mut self.api,
            &mut self.projection,
            MutationContext {
                online: self.state == ConnectionState::Online,
                another_mutation_pending: other_mutation_pending,
                session_only,
            },
            &mut notices,
        );
        if let Some(account) = self.phase4.take_linked_account(self.client_key.as_deref()) {
            self.apply_linked_account(account);
        }
        if let Some(session) = self.phase4.take_refreshed_session() {
            if let Some(account) = self.account.as_mut() {
                account.account_token = session.account_token;
                account.expires_in_seconds = session.expires_in_seconds;
            }
            self.state_reload_pending = true;
            self.projection.forget_authoritative_player_position();
            self.state_refresh = 0.0;
        }
        if self.phase4.take_logged_out() {
            self.clear_logged_out_session();
        }
        self.dispatch_requests();
        self.dispatch_trade_requests();
        let frontier_another_mutation_pending = self.phase4.mutation_in_flight()
            || self.general_mutation_pending()
            || self.pending_trade.is_some()
            || !self.trade_queue.is_empty();
        let mutations_ready = self.mutations_ready();
        self.frontier.dispatch(
            &mut self.api,
            mutations_ready,
            self.projection.cursor,
            self.phase4.auth_refresh_pending(),
            frontier_another_mutation_pending,
        );
        maintenance::restore_status(self);
        notices
    }

    fn apply_linked_account(&mut self, account: GuestSessionResponse) {
        self.account = Some(account);
        self.pending_state = None;
        self.pending_events = None;
        self.pending_movement = None;
        self.pending_chat = None;
        self.pending_farming = None;
        self.pending_trades = None;
        self.pending_trade = None;
        self.movement_queue.clear();
        self.chat_queue.clear();
        self.farming_queue.clear();
        self.trade_queue.clear();
        self.pending_request_type = None;
        self.pending_request_id = None;
        self.action_awaiting_confirmation = false;
        self.pending_trade_action = None;
        self.trades.clear();
        self.frontier.clear();
        self.projection.player = None;
        self.projection.forget_authoritative_player_position();
        self.projection.players.clear();
        self.projection.trades.clear();
        self.projection.claim = None;
        self.projection.outpost = None;
        self.projection.expedition = None;
        self.state_refresh = 0.0;
        self.state_reload_pending = true;
    }

    fn sync_regional_player_location(&mut self) {
        let Some(position) = self.projection.authoritative_player_position() else {
            return;
        };
        self.phase4.sync_regional_player_location(position);
    }

    fn mutations_ready(&self) -> bool {
        self.state == ConnectionState::Online && !self.state_reload_pending
    }

    fn session_mutations_ready(&self) -> bool {
        self.state == ConnectionState::Online && !self.phase4.auth_refresh_pending()
    }

    pub fn refresh_tavern(&mut self) {
        if self.state == ConnectionState::Online {
            self.state_refresh = 0.0;
        }
    }

    pub fn search_chronicle(&mut self, query: &str) {
        self.search_chronicle_page(query, 0);
    }

    pub fn search_chronicle_page(&mut self, query: &str, since: u64) {
        if self.mutations_ready() {
            self.frontier
                .queue_chronicle_search(query.to_owned(), since);
            self.status_message = "Searching the durable chronicle…".to_owned();
        }
    }

    pub(crate) fn chronicle_search_pending(&self) -> bool {
        self.frontier.chronicle_search_pending()
    }

    pub fn reconnect(&mut self) -> bool {
        if self.retry_cooldown > 0.0
            || matches!(
                self.state,
                ConnectionState::Connecting | ConnectionState::Online
            )
        {
            return false;
        }
        let production_reconnect = self.clear_session_state();
        self.retry_count = 0;
        if production_reconnect {
            self.state = ConnectionState::Online;
            self.status_message = "Restoring the production session…".to_owned();
            self.pending_ops_health = Some(self.api.get("/v1/ops/health"));
            self.phase4.begin_reconnect(&mut self.api);
        } else {
            self.begin_guest(false);
        }
        true
    }

    fn clear_session_state(&mut self) -> bool {
        let production_reconnect = self.phase4.has_refresh_session();
        let preserved_account = production_reconnect.then(|| self.account.take()).flatten();
        self.account = None;
        self.api.set_bearer_token(None);
        self.pending_guest = None;
        self.pending_state = None;
        self.pending_ops_health = None;
        self.maintenance_status = None;
        self.readiness_degraded = false;
        self.pending_events = None;
        self.pending_movement = None;
        self.pending_chat = None;
        self.pending_farming = None;
        self.pending_trades = None;
        self.pending_trade = None;
        self.frontier.clear();
        self.movement_queue.clear();
        self.chat_queue.clear();
        self.farming_queue.clear();
        self.trade_queue.clear();
        self.pending_request_type = None;
        self.pending_request_id = None;
        self.action_awaiting_confirmation = false;
        self.pending_trade_action = None;
        self.trades.clear();
        self.had_world = false;
        self.state_reload_pending = false;
        cursor::reset_projection_history(&mut self.projection);
        if production_reconnect {
            self.phase4.clear_for_reconnect();
            self.account = preserved_account;
        } else {
            self.phase4.clear();
        }
        production_reconnect
    }

    fn clear_logged_out_session(&mut self) {
        self.clear_session_state();
        self.client_key = None;
        self.api.set_bearer_token(None);
        self.state = ConnectionState::Degraded;
        self.status_message =
            "Signed out; tap Reconnect to start a fresh guest fixture.".to_owned();
    }

    fn begin_guest(&mut self, reset: bool) {
        self.state = ConnectionState::Connecting;
        self.maintenance_status = None;
        self.readiness_degraded = false;
        self.status_message = "Contacting the development road…".to_owned();
        self.pending_ops_health = Some(self.api.get("/v1/ops/health"));
        self.pending_guest = Some(self.api.post_json(
            "/v1/session/guest",
            &GuestSessionRequest {
                client_key: self.client_key.clone(),
                reset,
            },
        ));
    }

    fn poll_guest(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let result = self
            .pending_guest
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = result else { return };
        self.pending_guest = None;
        match result {
            Ok(response) => {
                self.client_key = Some(response.data.client_key.clone());
                self.api
                    .set_bearer_token(Some(&response.data.account_token));
                self.account = Some(response.data);
                self.status_message = "Guest identity found; loading the shared road…".to_owned();
                self.pending_state = Some(self.api.get("/v1/state"));
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }

    fn poll_state(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let result = self
            .pending_state
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = result else { return };
        self.pending_state = None;
        match result {
            Ok(response) => {
                let first_state = !self.had_world;
                let cursor = response.meta.cursor.unwrap_or(response.data.cursor);
                if state_snapshot_disposition(&self.projection, response.meta.server_tick, cursor)
                    == StateSnapshotDisposition::Reload
                {
                    self.state_reload_pending = true;
                    self.projection.forget_authoritative_player_position();
                    self.state_refresh = 0.0;
                    self.pending_state = Some(self.api.get("/v1/state"));
                    return;
                }
                self.projection
                    .apply_state(response.data, response.meta.server_tick);
                self.had_world = true;
                self.state_reload_pending = false;
                self.state = maintenance::state_after_snapshot(self.readiness_degraded);
                self.retry_count = 0;
                self.status_message = "The persistent settlement is open.".to_owned();
                self.state_refresh = 1.0;
                if first_state {
                    notices.push(NetworkNotice::Success(
                        "The settlement ledger is current.".to_owned(),
                    ));
                }
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }

    fn poll_events(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let result = self
            .pending_events
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = result else { return };
        self.pending_events = None;
        match result {
            Ok(response) => {
                if let Some(account) = &self.account {
                    let cursor = response.meta.cursor.unwrap_or(response.data.cursor);
                    if self
                        .projection
                        .response_is_newer(response.meta.server_tick, cursor)
                    {
                        self.projection.apply_events(
                            response.data,
                            &account.account_id,
                            response.meta.server_tick,
                        );
                    }
                }
            }
            Err(error) if cursor::is_cursor_recovery_error(&error) => {
                cursor::recover_from_cursor_boundary(self, notices)
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }

    fn connection_failed(&mut self, error: String, notices: &mut Vec<NetworkNotice>) {
        let rate_limited = is_rate_limited_error(&error);
        self.state = if self.had_world {
            ConnectionState::Degraded
        } else {
            ConnectionState::Offline
        };
        self.status_message = if rate_limited {
            "The road is rate-limited; wait briefly, then tap Reconnect.".to_owned()
        } else if self.state == ConnectionState::Degraded {
            "The server stopped answering; the last shared road is shown.".to_owned()
        } else {
            "The development server is unavailable.".to_owned()
        };
        self.retry_count = self.retry_count.saturating_add(1);
        self.retry_cooldown = 2.0;
        self.pending_state = None;
        self.pending_events = None;
        self.pending_movement = None;
        self.pending_chat = None;
        self.pending_farming = None;
        self.pending_trades = None;
        self.pending_trade = None;
        self.movement_queue.clear();
        self.chat_queue.clear();
        self.farming_queue.clear();
        self.trade_queue.clear();
        self.pending_request_type = None;
        self.pending_request_id = None;
        self.pending_trade_action = None;
        self.action_awaiting_confirmation = false;
        self.state_reload_pending = false;
        let retry_message = if rate_limited {
            "Wait briefly, then use Reconnect when it is available."
        } else if self.retry_count >= self.max_retry_count {
            "Retry limit reached; use Reconnect when the server is ready."
        } else {
            "Reconnect is available below."
        };
        notices.push(NetworkNotice::Danger(format!(
            "{} {}",
            short_error(&error),
            retry_message
        )));
    }

    fn next_request_id(&mut self, prefix: &str) -> String {
        let id = format!("{prefix}-{}", self.next_request_id);
        self.next_request_id = self.next_request_id.saturating_add(1);
        id
    }

    fn general_mutation_pending(&self) -> bool {
        self.pending_movement.is_some()
            || self.pending_chat.is_some()
            || self.pending_farming.is_some()
            || !self.movement_queue.is_empty()
            || !self.chat_queue.is_empty()
            || !self.farming_queue.is_empty()
    }
}

fn is_rate_limited_error(error: &str) -> bool {
    error.contains("rate_limited") || error.contains("status code 429")
}

#[derive(Debug, PartialEq, Eq)]
enum StateSnapshotDisposition {
    Apply,
    Reload,
}

fn state_snapshot_disposition(
    projection: &WorldProjection,
    server_tick: u64,
    cursor: u64,
) -> StateSnapshotDisposition {
    if projection.response_is_current(server_tick, cursor) {
        StateSnapshotDisposition::Apply
    } else {
        StateSnapshotDisposition::Reload
    }
}

fn remote_player(presence: PlayerPresence) -> RemotePlayer {
    RemotePlayer {
        account_id: presence.account_id,
        character_id: presence.character_id,
        display_name: presence.display_name,
        position: TilePos::new(presence.position.x, presence.position.y),
        last_seen_tick: presence.last_seen_tick,
        online: presence.online,
    }
}

fn from_protocol_tile(tile: tarrowyn_protocol::TileKind) -> TileKind {
    match tile {
        tarrowyn_protocol::TileKind::Meadow => TileKind::Meadow,
        tarrowyn_protocol::TileKind::Path => TileKind::Path,
        tarrowyn_protocol::TileKind::Field => TileKind::Field,
        tarrowyn_protocol::TileKind::Forest => TileKind::Forest,
        tarrowyn_protocol::TileKind::Water => TileKind::Water,
        tarrowyn_protocol::TileKind::Stone => TileKind::Stone,
    }
}

fn short_error(error: &str) -> String {
    error
        .lines()
        .next()
        .unwrap_or(error)
        .chars()
        .take(120)
        .collect()
}

#[cfg(test)]
mod tests;
