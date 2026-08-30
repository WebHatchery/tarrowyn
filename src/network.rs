//! Frame-polled online session and the client-side world projection.

use crate::data::GameConfig;
use crate::state::{TileKind, WorldState};
use macroquad_toolkit::grid::{FlatGrid, TilePos};
use macroquad_toolkit::net::{HttpClient, Pending};
use std::collections::VecDeque;
use tarrowyn_protocol::{
    ApiResponse, ChatMessage, ChatRequest, ChronicleEntry, ChronicleSummary, EventsResponse,
    Expedition, FarmAnimal, FarmingAction, FarmingRequest, FrontierEvent, GuestSessionRequest,
    GuestSessionResponse, LandClaim, MovementIntent, OpportunitySignal, OpsHealthResponse,
    PlayerPresence, PlayerProjection, StateSnapshot, TavernFeedResponse, TimeOfDay, TradeAction,
    TradeOffer, TradeRequest, TradesResponse, WildernessZone, WorldClock, WorldEvent,
    WorldSnapshot, MAX_CHAT_MESSAGE_LENGTH,
};

const REQUEST_TIMEOUT_SECONDS: f32 = 6.0;

pub(super) fn is_transient_transport_error(error: &str) -> bool {
    error.contains(" timed out after ")
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
    pub opportunities: Vec<OpportunitySignal>,
    pub claim: Option<LandClaim>,
    pub outpost: Option<macroquad_toolkit::grid::TilePos>,
    pub expedition: Option<Expedition>,
}

impl WorldProjection {
    pub fn new(config: &GameConfig) -> Self {
        Self {
            world: WorldState::new(config),
            player_position: TilePos::new(8, 6),
            players: Vec::new(),
            chat: Vec::new(),
            day: 1,
            day_seconds: 0.0,
            day_length_seconds: config.day_length_seconds,
            server_tick: 0,
            cursor: 0,
            player: None,
            animals: Vec::new(),
            feed: TavernFeedResponse {
                notices: Vec::new(),
                rumours: Vec::new(),
                chat: Vec::new(),
                cursor: 0,
            },
            trades: Vec::new(),
            wilderness: None,
            chronicle: Vec::new(),
            chronicle_summary: None,
            opportunities: Vec::new(),
            claim: None,
            outpost: None,
            expedition: None,
        }
    }

    pub fn clock_minutes(&self) -> u32 {
        let fraction = (self.day_seconds / self.day_length_seconds.max(1.0)).clamp(0.0, 1.0);
        (((6.0 + fraction * 24.0) % 24.0) * 60.0) as u32
    }

    pub fn time_of_day(&self) -> TimeOfDay {
        TimeOfDay::from_clock_minutes(self.clock_minutes())
    }

    pub fn is_night(&self) -> bool {
        self.time_of_day().is_night()
    }

    fn apply_snapshot(&mut self, snapshot: WorldSnapshot, own_account: &str, server_tick: u64) {
        let mut tiles = FlatGrid::new(
            snapshot.width as usize,
            snapshot.height as usize,
            TileKind::Meadow,
        );
        for tile in snapshot.tiles {
            let position = TilePos::new(tile.position.x, tile.position.y);
            if tiles.is_valid(position) {
                tiles.set(position, from_protocol_tile(tile.kind));
            }
        }
        self.world = WorldState {
            tiles,
            crops: FlatGrid::new(snapshot.width as usize, snapshot.height as usize, None),
            reachable: Default::default(),
        };
        self.apply_plots(&snapshot.plots);
        self.animals = snapshot.animals;
        self.apply_clock(snapshot.clock);
        self.server_tick = server_tick;
        self.cursor = snapshot.cursor;
        self.players = snapshot.players.into_iter().map(remote_player).collect();
        self.wilderness = snapshot.wilderness;
        self.outpost = snapshot
            .outpost
            .map(|position| TilePos::new(position.x, position.y));
        self.claim = snapshot.claim;
        self.expedition = snapshot.expedition;
        if let Some(player) = self
            .players
            .iter()
            .find(|player| player.account_id == own_account)
        {
            self.player_position = player.position;
        }
    }

    fn apply_state(&mut self, snapshot: StateSnapshot, server_tick: u64) {
        self.apply_snapshot(snapshot.world, &snapshot.player.account_id, server_tick);
        self.player = Some(snapshot.player);
        self.feed = snapshot.feed;
        self.chat = self.feed.chat.clone();
    }

    fn apply_events(&mut self, response: EventsResponse, own_account: &str, server_tick: u64) {
        self.server_tick = self.server_tick.max(server_tick);
        self.apply_clock(response.clock);
        for record in response.events {
            self.cursor = self.cursor.max(record.cursor);
            match record.event {
                WorldEvent::Presence(presence) => self.apply_presence(presence, own_account),
                WorldEvent::Clock(clock) => self.apply_clock(clock),
                WorldEvent::Chat(message) => self.push_chat(message),
                WorldEvent::Farming(plot) => self.apply_plot(plot),
                WorldEvent::Trade(_) => {}
                WorldEvent::TavernNotice(notice) => {
                    self.feed.notices.push(notice);
                    if self.feed.notices.len() > 8 {
                        self.feed.notices.remove(0);
                    }
                }
                WorldEvent::Chronicle(entry) => {
                    merge_chronicle_entry(&mut self.chronicle, entry);
                }
                WorldEvent::Frontier(event) => match event {
                    FrontierEvent::Threat(zone) => self.wilderness = Some(zone),
                    FrontierEvent::Opportunity(opportunity) => {
                        self.opportunities
                            .retain(|existing| existing.household_id != opportunity.household_id);
                        self.opportunities.push(opportunity);
                    }
                    FrontierEvent::Claim(claim) => self.claim = Some(claim),
                    FrontierEvent::Expedition(expedition) => {
                        self.expedition = Some(expedition.clone());
                        self.outpost = (expedition.status
                            == tarrowyn_protocol::ExpeditionStatus::Succeeded)
                            .then_some(TilePos::new(
                                expedition.outpost_position.x,
                                expedition.outpost_position.y,
                            ));
                    }
                },
            }
        }
        self.cursor = self.cursor.max(response.cursor);
    }

    pub(super) fn response_is_current(&self, server_tick: u64, cursor: u64) -> bool {
        server_tick >= self.server_tick && cursor >= self.cursor
    }

    pub(super) fn response_is_newer(&self, server_tick: u64, cursor: u64) -> bool {
        server_tick >= self.server_tick && cursor > self.cursor
    }

    pub(super) fn record_response_version(&mut self, server_tick: u64, cursor: Option<u64>) {
        self.server_tick = self.server_tick.max(server_tick);
        if let Some(cursor) = cursor {
            self.cursor = self.cursor.max(cursor);
        }
    }

    fn apply_presence(&mut self, presence: PlayerPresence, own_account: &str) {
        let remote = remote_player(presence);
        if remote.account_id == own_account {
            self.player_position = remote.position;
        }
        if let Some(existing) = self
            .players
            .iter_mut()
            .find(|player| player.character_id == remote.character_id)
        {
            *existing = remote;
        } else {
            self.players.push(remote);
        }
    }

    fn apply_clock(&mut self, clock: WorldClock) {
        self.day = clock.day;
        self.day_seconds = clock.seconds;
        self.day_length_seconds = clock.day_length_seconds;
    }

    fn push_chat(&mut self, message: ChatMessage) {
        if self
            .chat
            .iter()
            .any(|existing| existing.message_id == message.message_id)
        {
            return;
        }
        self.chat.push(message);
        self.chat.sort_by_key(|message| message.cursor);
        if self.chat.len() > 8 {
            let keep_from = self.chat.len() - 8;
            self.chat.drain(0..keep_from);
        }
    }
}

pub struct OnlineClient {
    api: HttpClient,
    pub projection: WorldProjection,
    pub state: ConnectionState,
    pub status_message: String,
    pub client_key: Option<String>,
    pub account: Option<GuestSessionResponse>,
    pending_guest: Option<Pending<ApiResponse<GuestSessionResponse>>>,
    pending_world: Option<Pending<ApiResponse<WorldSnapshot>>>,
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
            pending_world: None,
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
        };
        client.begin_guest(false);
        client
    }

    pub fn update(&mut self, dt: f32) -> Vec<NetworkNotice> {
        self.retry_cooldown = (self.retry_cooldown - dt.max(0.0)).max(0.0);
        self.state_refresh = (self.state_refresh - dt.max(0.0)).max(0.0);
        let mut notices = Vec::new();
        self.poll_guest(dt, &mut notices);
        self.poll_world(dt, &mut notices);
        self.poll_state(dt, &mut notices);
        maintenance::poll_ops_health(self, dt);
        self.poll_events(dt, &mut notices);
        self.poll_movement(dt, &mut notices);
        self.poll_chat(dt, &mut notices);
        self.poll_farming(dt, &mut notices);
        self.poll_trade_requests(dt, &mut notices);
        let frontier_cursor_boundary = self.frontier.update(
            &mut self.projection,
            dt,
            self.state == ConnectionState::Online,
            &mut notices,
        );
        if frontier_cursor_boundary {
            cursor::recover_from_cursor_boundary(self, &mut notices);
        }
        self.phase4.set_account(
            self.account
                .as_ref()
                .map(|account| account.account_id.as_str()),
        );
        let other_mutation_pending = self.frontier.has_pending_command()
            || self.pending_movement.is_some()
            || self.pending_chat.is_some()
            || self.pending_farming.is_some()
            || self.pending_trade.is_some();
        self.phase4.update(
            dt,
            &mut self.api,
            self.state == ConnectionState::Online,
            other_mutation_pending,
            &mut notices,
        );
        if let Some(account) = self.phase4.take_linked_account(self.client_key.as_deref()) {
            self.account = Some(account);
        }
        if let Some(session) = self.phase4.take_refreshed_session() {
            if let Some(account) = self.account.as_mut() {
                account.account_token = session.account_token;
                account.expires_in_seconds = session.expires_in_seconds;
            }
        }
        if self.phase4.take_logged_out() {
            self.clear_logged_out_session();
        }
        self.dispatch_requests();
        self.dispatch_trade_requests();
        self.frontier.dispatch(
            &mut self.api,
            self.state == ConnectionState::Online,
            self.projection.cursor,
            self.phase4.auth_refresh_pending(),
        );
        maintenance::restore_status(self);
        notices
    }

    pub fn refresh_tavern(&mut self) {
        if self.state == ConnectionState::Online {
            self.state_refresh = 0.0;
        }
    }

    pub fn reconnect(&mut self) -> bool {
        if self.retry_cooldown > 0.0 {
            return false;
        }
        self.clear_session_state();
        self.retry_count = 0;
        self.begin_guest(false);
        true
    }

    fn clear_session_state(&mut self) {
        self.account = None;
        self.api.set_bearer_token(None);
        self.pending_guest = None;
        self.pending_world = None;
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
        self.phase4.clear();
        self.movement_queue.clear();
        self.chat_queue.clear();
        self.farming_queue.clear();
        self.trade_queue.clear();
        self.pending_request_type = None;
        self.pending_request_id = None;
        self.action_awaiting_confirmation = false;
        self.pending_trade_action = None;
        self.trades.clear();
        cursor::reset_projection_history(&mut self.projection);
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
                if self
                    .projection
                    .response_is_current(response.meta.server_tick, cursor)
                {
                    self.projection
                        .apply_state(response.data, response.meta.server_tick);
                }
                self.had_world = true;
                self.state = ConnectionState::Online;
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

    fn poll_world(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let result = self
            .pending_world
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = result else { return };
        self.pending_world = None;
        match result {
            Ok(response) => {
                if let Some(account) = &self.account {
                    let cursor = response.meta.cursor.unwrap_or(response.data.cursor);
                    if self
                        .projection
                        .response_is_current(response.meta.server_tick, cursor)
                    {
                        self.projection.apply_snapshot(
                            response.data,
                            &account.account_id,
                            response.meta.server_tick,
                        );
                    }
                }
                self.had_world = true;
                self.state = ConnectionState::Online;
                self.status_message = "The shared road is open.".to_owned();
                notices.push(NetworkNotice::Success(
                    "Connected to the shared road.".to_owned(),
                ));
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
        self.state = if self.had_world {
            ConnectionState::Degraded
        } else {
            ConnectionState::Offline
        };
        self.status_message = if self.state == ConnectionState::Degraded {
            "The server stopped answering; the last shared road is shown.".to_owned()
        } else {
            "The development server is unavailable.".to_owned()
        };
        self.retry_count = self.retry_count.saturating_add(1);
        self.retry_cooldown = 2.0;
        self.movement_queue.clear();
        self.chat_queue.clear();
        self.farming_queue.clear();
        self.action_awaiting_confirmation = false;
        let retry_message = if self.retry_count >= self.max_retry_count {
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
