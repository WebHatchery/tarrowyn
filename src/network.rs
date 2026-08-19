//! Frame-polled online session and the client-side world projection.

use crate::data::GameConfig;
use crate::state::{TileKind, WorldState};
use macroquad_toolkit::grid::{FlatGrid, TilePos};
use macroquad_toolkit::net::{HttpClient, Pending};
use serde::Serialize;
use std::collections::VecDeque;
use tarrowyn_protocol::{
    ApiResponse, ChatMessage, ChatRequest, EventsResponse, GuestSessionRequest,
    GuestSessionResponse, MovementIntent, MovementResponse, PlayerPresence, WorldClock, WorldEvent,
    WorldSnapshot, MAX_CHAT_MESSAGE_LENGTH,
};

const REQUEST_TIMEOUT_SECONDS: f32 = 6.0;
const STALE_TICKS: u64 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Online,
    Degraded,
    Offline,
}

impl ConnectionState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Connecting => "CONNECTING",
            Self::Online => "ONLINE",
            Self::Degraded => "DEGRADED",
            Self::Offline => "OFFLINE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkNotice {
    Info(String),
    Success(String),
    Warning(String),
    Danger(String),
}

pub struct RemotePlayer {
    pub account_id: String,
    pub character_id: String,
    pub display_name: String,
    pub position: TilePos,
    pub last_seen_tick: u64,
    pub online: bool,
}

impl RemotePlayer {
    pub fn stale(&self, server_tick: u64) -> bool {
        !self.online || server_tick.saturating_sub(self.last_seen_tick) > STALE_TICKS
    }
}

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
        }
    }

    pub fn clock_minutes(&self) -> u32 {
        let fraction = (self.day_seconds / self.day_length_seconds.max(1.0)).clamp(0.0, 1.0);
        (((6.0 + fraction * 24.0) % 24.0) * 60.0) as u32
    }

    pub fn is_night(&self) -> bool {
        let hour = self.clock_minutes() / 60;
        !(7..=18).contains(&hour)
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
        self.apply_clock(snapshot.clock);
        self.server_tick = server_tick;
        self.cursor = snapshot.cursor;
        self.players = snapshot.players.into_iter().map(remote_player).collect();
        if let Some(player) = self
            .players
            .iter()
            .find(|player| player.account_id == own_account)
        {
            self.player_position = player.position;
        }
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
            }
        }
        self.cursor = self.cursor.max(response.cursor);
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

struct PendingMovement {
    pending: Pending<ApiResponse<MovementResponse>>,
}

struct PendingChat {
    pending: Pending<ApiResponse<tarrowyn_protocol::ChatResponse>>,
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
    pending_events: Option<Pending<ApiResponse<EventsResponse>>>,
    pending_movement: Option<PendingMovement>,
    pending_chat: Option<PendingChat>,
    movement_queue: VecDeque<MovementIntent>,
    chat_queue: VecDeque<ChatRequest>,
    next_request_id: u64,
    retry_cooldown: f32,
    had_world: bool,
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
            pending_events: None,
            pending_movement: None,
            pending_chat: None,
            movement_queue: VecDeque::new(),
            chat_queue: VecDeque::new(),
            next_request_id: 1,
            retry_cooldown: 0.0,
            had_world: false,
        };
        client.begin_guest(false);
        client
    }

    pub fn update(&mut self, dt: f32) -> Vec<NetworkNotice> {
        self.retry_cooldown = (self.retry_cooldown - dt.max(0.0)).max(0.0);
        let mut notices = Vec::new();
        self.poll_guest(dt, &mut notices);
        self.poll_world(dt, &mut notices);
        self.poll_events(dt, &mut notices);
        self.poll_movement(dt, &mut notices);
        self.poll_chat(dt, &mut notices);
        self.dispatch_requests();
        notices
    }

    pub fn queue_movement(&mut self, dx: i32, dy: i32) {
        if self.state != ConnectionState::Online {
            return;
        }
        let request_id = self.next_request_id("move");
        self.movement_queue
            .push_back(MovementIntent { request_id, dx, dy });
    }

    pub fn queue_move_toward(&mut self, target: TilePos) {
        let dx = target.x - self.projection.player_position.x;
        let dy = target.y - self.projection.player_position.y;
        if dx.abs() >= dy.abs() && dx != 0 {
            self.queue_movement(dx.signum(), 0);
        } else if dy != 0 {
            self.queue_movement(0, dy.signum());
        }
    }

    pub fn queue_chat(&mut self, text: &str) {
        if self.state != ConnectionState::Online {
            return;
        }
        let text: String = text.chars().take(MAX_CHAT_MESSAGE_LENGTH).collect();
        if text.trim().is_empty() {
            return;
        }
        let request_id = self.next_request_id("chat");
        self.chat_queue.push_back(ChatRequest {
            request_id,
            channel: "settlement".to_owned(),
            text,
        });
    }

    pub fn reconnect(&mut self) -> bool {
        if self.retry_cooldown > 0.0 {
            return false;
        }
        self.pending_world = None;
        self.pending_events = None;
        self.pending_movement = None;
        self.pending_chat = None;
        self.movement_queue.clear();
        self.chat_queue.clear();
        self.begin_guest(false);
        true
    }

    fn begin_guest(&mut self, reset: bool) {
        self.state = ConnectionState::Connecting;
        self.status_message = "Contacting the development road…".to_owned();
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
                self.pending_world = Some(self.api.get("/v1/world"));
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
                    self.projection.apply_snapshot(
                        response.data,
                        &account.account_id,
                        response.meta.server_tick,
                    );
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
                    self.projection.apply_events(
                        response.data,
                        &account.account_id,
                        response.meta.server_tick,
                    );
                }
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }

    fn poll_movement(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let result = self
            .pending_movement
            .as_mut()
            .and_then(|pending| pending.pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = result else { return };
        self.pending_movement = None;
        match result {
            Ok(response) => {
                let position = response.data.position;
                self.projection.player_position = TilePos::new(position.x, position.y);
                if response.data.accepted {
                    notices.push(NetworkNotice::Info(
                        "The server accepted that step.".to_owned(),
                    ));
                } else {
                    notices.push(NetworkNotice::Warning(
                        response
                            .data
                            .reason
                            .unwrap_or_else(|| "The server rejected that step.".to_owned()),
                    ));
                }
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }

    fn poll_chat(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let result = self
            .pending_chat
            .as_mut()
            .and_then(|pending| pending.pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = result else { return };
        self.pending_chat = None;
        match result {
            Ok(response) => {
                if response.data.accepted {
                    if let Some(message) = response.data.message {
                        self.projection.push_chat(message);
                    }
                    notices.push(NetworkNotice::Success(
                        "Message sent to the settlement.".to_owned(),
                    ));
                } else {
                    notices.push(NetworkNotice::Warning(
                        response
                            .data
                            .reason
                            .unwrap_or_else(|| "The server rejected that message.".to_owned()),
                    ));
                }
            }
            Err(error) => self.connection_failed(error, notices),
        }
    }

    fn dispatch_requests(&mut self) {
        if self.state != ConnectionState::Online {
            return;
        }
        if self.pending_events.is_none() {
            self.pending_events = Some(
                self.api
                    .get(&format!("/v1/events?since={}", self.projection.cursor)),
            );
        }
        if self.pending_movement.is_none() {
            if let Some(request) = self.movement_queue.pop_front() {
                let pending = self.api.post_json("/v1/movement", &request);
                self.pending_movement = Some(PendingMovement { pending });
            }
        }
        if self.pending_chat.is_none() {
            if let Some(request) = self.chat_queue.pop_front() {
                let pending = self.api.post_json("/v1/chat", &request);
                self.pending_chat = Some(PendingChat { pending });
            }
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
        self.retry_cooldown = 2.0;
        self.movement_queue.clear();
        self.chat_queue.clear();
        notices.push(NetworkNotice::Danger(format!(
            "{} Reconnect is available below.",
            short_error(&error)
        )));
    }

    fn next_request_id(&mut self, prefix: &str) -> String {
        let id = format!("{prefix}-{}", self.next_request_id);
        self.next_request_id += 1;
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

#[allow(dead_code)]
fn _assert_request_serializable<T: Serialize>() {}

#[cfg(test)]
mod tests;
