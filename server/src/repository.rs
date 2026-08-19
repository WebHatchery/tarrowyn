use crate::config::ServerConfig;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use tarrowyn_protocol::{
    ApiError, ApiMeta, ApiResponse, ChatMessage, ChatRequest, ChatResponse, EventRecord,
    EventsResponse, GuestSessionRequest, GuestSessionResponse, HealthResponse, MovementIntent,
    MovementResponse, PlayerPresence, Position, TileKind, WorldClock, WorldEvent, WorldSnapshot,
    WorldTile, PROTOCOL_VERSION,
};

#[derive(Debug, Clone)]
pub struct RepositoryError {
    pub status: u16,
    pub error: ApiError,
}

impl RepositoryError {
    fn new(status: u16, code: &str, message: impl Into<String>) -> Self {
        Self {
            status,
            error: ApiError {
                code: code.to_owned(),
                message: message.into(),
            },
        }
    }

    fn unauthorized() -> Self {
        Self::new(401, "unauthorized", "A valid guest session is required.")
    }
}

#[derive(Debug)]
struct Identity {
    account_id: String,
    character_id: String,
    display_name: String,
    position: Position,
}

#[derive(Debug, Clone)]
struct Session {
    client_key: String,
    identity_key: String,
    token: String,
    last_seen_tick: u64,
    last_movement_tick: Option<u64>,
    last_chat_tick: Option<u64>,
    movement_results: HashMap<String, MovementResponse>,
    chat_results: HashMap<String, ChatResponse>,
}

#[derive(Debug)]
struct RepositoryState {
    tick: u64,
    clock: WorldClock,
    cursor: u64,
    next_guest: u64,
    next_message: u64,
    next_token: u64,
    identities: HashMap<String, Identity>,
    sessions: HashMap<String, Session>,
    events: VecDeque<EventRecord>,
}

pub struct WorldRepository {
    config: ServerConfig,
    state: Mutex<RepositoryState>,
}

impl WorldRepository {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            state: Mutex::new(RepositoryState {
                tick: 0,
                clock: WorldClock {
                    day: 1,
                    seconds: 0.0,
                    day_length_seconds: config.day_length_seconds,
                },
                cursor: 0,
                next_guest: 1,
                next_message: 1,
                next_token: 1,
                identities: HashMap::new(),
                sessions: HashMap::new(),
                events: VecDeque::new(),
            }),
            config,
        }
    }

    pub fn health(&self) -> ApiResponse<HealthResponse> {
        let state = self.state.lock().expect("world repository lock poisoned");
        ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: HealthResponse {
                status: "ok".to_owned(),
                service: "tarrowyn-server".to_owned(),
                protocol_version: PROTOCOL_VERSION.to_owned(),
            },
        }
    }

    pub fn guest_session(&self, request: GuestSessionRequest) -> ApiResponse<GuestSessionResponse> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        let client_key = request
            .client_key
            .filter(|key| !key.trim().is_empty())
            .unwrap_or_else(|| format!("guest-client-{}", state.next_guest));
        if request.reset || !state.identities.contains_key(&client_key) {
            let number = state.next_guest;
            state.next_guest += 1;
            state.identities.insert(
                client_key.clone(),
                Identity {
                    account_id: format!("dev-account-{number}"),
                    character_id: format!("dev-character-{number}"),
                    display_name: format!("Guest {number}"),
                    position: Position { x: 8, y: 6 },
                },
            );
        }

        let old_tokens: Vec<String> = state
            .sessions
            .values()
            .filter(|session| session.client_key == client_key)
            .map(|session| session.token.clone())
            .collect();
        for token in old_tokens {
            state.sessions.remove(&token);
        }

        let token = format!("dev-session-{}", state.next_token);
        state.next_token += 1;
        let tick = state.tick;
        state.sessions.insert(
            token.clone(),
            Session {
                client_key: client_key.clone(),
                identity_key: client_key.clone(),
                token: token.clone(),
                last_seen_tick: tick,
                last_movement_tick: None,
                last_chat_tick: None,
                movement_results: HashMap::new(),
                chat_results: HashMap::new(),
            },
        );
        let identity = state.identities.get(&client_key).expect("identity created");
        let presence = presence(identity, state.tick, true);
        let cursor = push_event(&mut state, WorldEvent::Presence(presence));
        let identity = state.identities.get(&client_key).expect("identity created");
        ApiResponse {
            meta: meta(state.tick, None, Some(cursor)),
            data: GuestSessionResponse {
                client_key,
                account_id: identity.account_id.clone(),
                character_id: identity.character_id.clone(),
                display_name: identity.display_name.clone(),
                account_token: token,
                expires_in_seconds: self.config.session_ttl_seconds,
            },
        }
    }

    pub fn world(&self, token: &str) -> Result<ApiResponse<WorldSnapshot>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let identity_key = authenticate(&mut state, token, &self.config)?;
        let mut players = active_presences(&state);
        players.sort_by(|left, right| left.character_id.cmp(&right.character_id));
        let snapshot = WorldSnapshot {
            width: self.config.world_width,
            height: self.config.world_height,
            tiles: world_tiles(self.config.world_width, self.config.world_height),
            clock: state.clock.clone(),
            players,
            cursor: state.cursor,
        };
        let _ = identity_key;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: snapshot,
        })
    }

    pub fn movement(
        &self,
        token: &str,
        intent: MovementIntent,
    ) -> Result<ApiResponse<MovementResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let identity_key = authenticate(&mut state, token, &self.config)?;
        if intent.request_id.trim().is_empty() || intent.request_id.len() > 64 {
            return Err(RepositoryError::new(
                400,
                "invalid_request_id",
                "Movement request IDs must contain 1 to 64 characters.",
            ));
        }
        if let Some(previous) = state
            .sessions
            .get(token)
            .and_then(|session| session.movement_results.get(&intent.request_id))
            .cloned()
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(intent.request_id), Some(state.cursor)),
                data: previous,
            });
        }

        let identity = state
            .identities
            .get(&identity_key)
            .expect("identity exists");
        let current = identity.position;
        let mut response = MovementResponse {
            request_id: intent.request_id.clone(),
            accepted: false,
            position: current,
            reason: None,
        };
        let cardinal = intent.dx.abs() + intent.dy.abs() == 1;
        let rate_limited = state
            .sessions
            .get(token)
            .and_then(|session| session.last_movement_tick)
            .is_some_and(|last| {
                state.tick.saturating_sub(last) < self.config.movement_cooldown_ticks
            });
        if !cardinal {
            response.reason = Some("Movement must be one cardinal step.".to_owned());
        } else if rate_limited {
            response.reason = Some("Movement is arriving too quickly.".to_owned());
        } else {
            let next = Position {
                x: current.x + intent.dx,
                y: current.y + intent.dy,
            };
            if next.x < 0
                || next.y < 0
                || next.x >= self.config.world_width as i32
                || next.y >= self.config.world_height as i32
            {
                response.reason = Some("The settlement edge blocks that step.".to_owned());
            } else if !tile_at(next, self.config.world_width, self.config.world_height)
                .is_walkable()
            {
                response.reason = Some("Water blocks that step.".to_owned());
            } else {
                response.accepted = true;
                response.position = next;
                state
                    .identities
                    .get_mut(&identity_key)
                    .expect("identity exists")
                    .position = next;
                state
                    .sessions
                    .get_mut(token)
                    .expect("session exists")
                    .last_movement_tick = Some(state.tick);
                let presence = {
                    let identity = state
                        .identities
                        .get(&identity_key)
                        .expect("identity exists");
                    presence(identity, state.tick, true)
                };
                let cursor = push_event(&mut state, WorldEvent::Presence(presence));
                state
                    .sessions
                    .get_mut(token)
                    .expect("session exists")
                    .movement_results
                    .insert(intent.request_id.clone(), response.clone());
                return Ok(ApiResponse {
                    meta: meta(state.tick, Some(intent.request_id), Some(cursor)),
                    data: response,
                });
            }
        }
        state
            .sessions
            .get_mut(token)
            .expect("session exists")
            .movement_results
            .insert(intent.request_id.clone(), response.clone());
        Ok(ApiResponse {
            meta: meta(state.tick, Some(intent.request_id), Some(state.cursor)),
            data: response,
        })
    }

    pub fn events(
        &self,
        token: &str,
        since: u64,
    ) -> Result<ApiResponse<EventsResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        authenticate(&mut state, token, &self.config)?;
        let events = state
            .events
            .iter()
            .filter(|record| record.cursor > since)
            .cloned()
            .collect();
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: EventsResponse {
                cursor: state.cursor,
                clock: state.clock.clone(),
                events,
            },
        })
    }

    pub fn chat(
        &self,
        token: &str,
        request: ChatRequest,
    ) -> Result<ApiResponse<ChatResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let identity_key = authenticate(&mut state, token, &self.config)?;
        if let Some(previous) = state
            .sessions
            .get(token)
            .and_then(|session| session.chat_results.get(&request.request_id))
            .cloned()
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous,
            });
        }

        let text = request.text.trim().to_owned();
        let mut response = ChatResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            message: None,
            reason: None,
        };
        let too_long = text.chars().count() > self.config.chat_max_length;
        let too_fast = state
            .sessions
            .get(token)
            .and_then(|session| session.last_chat_tick)
            .is_some_and(|last| state.tick == last);
        if request.request_id.trim().is_empty() {
            response.reason = Some("Chat request IDs are required.".to_owned());
        } else if text.is_empty() {
            response.reason = Some("A message cannot be empty.".to_owned());
        } else if too_long {
            response.reason = Some(format!(
                "Messages are limited to {} characters.",
                self.config.chat_max_length
            ));
        } else if too_fast {
            response.reason = Some("Give the channel a moment before sending again.".to_owned());
        } else {
            let identity = state
                .identities
                .get(&identity_key)
                .expect("identity exists");
            let mut message = ChatMessage {
                message_id: state.next_message,
                account_id: identity.account_id.clone(),
                display_name: identity.display_name.clone(),
                channel: if request.channel.trim().is_empty() {
                    "settlement".to_owned()
                } else {
                    request.channel.trim().to_owned()
                },
                text,
                cursor: 0,
            };
            state.next_message += 1;
            let cursor = push_event(&mut state, WorldEvent::Chat(message.clone()));
            message.cursor = cursor;
            if let Some(EventRecord {
                event: WorldEvent::Chat(stored),
                ..
            }) = state.events.back_mut()
            {
                *stored = message.clone();
            }
            response.accepted = true;
            response.message = Some(message);
            state
                .sessions
                .get_mut(token)
                .expect("session exists")
                .last_chat_tick = Some(state.tick);
        }
        state
            .sessions
            .get_mut(token)
            .expect("session exists")
            .chat_results
            .insert(request.request_id.clone(), response.clone());
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }

    pub fn tick(&self) {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        state.tick += 1;
        state.clock.seconds += self.config.world_seconds_per_tick.max(0.0);
        while state.clock.seconds >= state.clock.day_length_seconds.max(1.0) {
            state.clock.seconds -= state.clock.day_length_seconds.max(1.0);
            state.clock.day += 1;
        }
        let clock = state.clock.clone();
        push_event(&mut state, WorldEvent::Clock(clock));
        expire_sessions(&mut state, &self.config);
    }

    pub fn server_tick(&self) -> u64 {
        self.state
            .lock()
            .expect("world repository lock poisoned")
            .tick
    }
}

fn authenticate(
    state: &mut RepositoryState,
    token: &str,
    config: &ServerConfig,
) -> Result<String, RepositoryError> {
    let Some(session) = state.sessions.get(token) else {
        return Err(RepositoryError::unauthorized());
    };
    if state.tick.saturating_sub(session.last_seen_tick) > config.session_ttl_ticks() {
        state.sessions.remove(token);
        return Err(RepositoryError::unauthorized());
    }
    let identity_key = session.identity_key.clone();
    state
        .sessions
        .get_mut(token)
        .expect("session exists")
        .last_seen_tick = state.tick;
    Ok(identity_key)
}

fn expire_sessions(state: &mut RepositoryState, config: &ServerConfig) {
    let expired: Vec<String> = state
        .sessions
        .iter()
        .filter(|(_, session)| {
            state.tick.saturating_sub(session.last_seen_tick) > config.session_ttl_ticks()
        })
        .map(|(token, _)| token.clone())
        .collect();
    for token in expired {
        if let Some(session) = state.sessions.remove(&token) {
            if let Some(identity) = state.identities.get(&session.identity_key) {
                push_event(
                    state,
                    WorldEvent::Presence(presence(identity, state.tick, false)),
                );
            }
        }
    }
}

fn active_presences(state: &RepositoryState) -> Vec<PlayerPresence> {
    state
        .sessions
        .values()
        .filter_map(|session| {
            state
                .identities
                .get(&session.identity_key)
                .map(|identity| presence(identity, session.last_seen_tick, true))
        })
        .collect()
}

fn presence(identity: &Identity, last_seen_tick: u64, online: bool) -> PlayerPresence {
    PlayerPresence {
        account_id: identity.account_id.clone(),
        character_id: identity.character_id.clone(),
        display_name: identity.display_name.clone(),
        position: identity.position,
        last_seen_tick,
        online,
    }
}

fn meta(tick: u64, request_id: Option<String>, cursor: Option<u64>) -> ApiMeta {
    let mut meta = ApiMeta::at(tick);
    meta.request_id = request_id;
    meta.cursor = cursor;
    meta
}

fn push_event(state: &mut RepositoryState, event: WorldEvent) -> u64 {
    state.cursor += 1;
    state.events.push_back(EventRecord {
        cursor: state.cursor,
        event,
    });
    while state.events.len() > 2048 {
        state.events.pop_front();
    }
    state.cursor
}

fn world_tiles(width: u32, height: u32) -> Vec<WorldTile> {
    let mut tiles = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            tiles.push(WorldTile {
                position: Position {
                    x: x as i32,
                    y: y as i32,
                },
                kind: tile_at(
                    Position {
                        x: x as i32,
                        y: y as i32,
                    },
                    width,
                    height,
                ),
            });
        }
    }
    tiles
}

fn tile_at(position: Position, width: u32, height: u32) -> TileKind {
    let x = position.x as u32;
    let y = position.y as u32;
    if x >= width || y >= height {
        return TileKind::Water;
    }
    if (x <= 1 && y <= 4) || (x == 16 && (2..=8).contains(&y)) {
        TileKind::Water
    } else if ((12..=15).contains(&x) && y <= 4) || ((13..=16).contains(&x) && y >= 8) {
        TileKind::Forest
    } else if ((2..16).contains(&x) && y == 6) || (x == 8 && (4..7).contains(&y)) {
        TileKind::Path
    } else if (3..6).contains(&x) && (4..6).contains(&y) {
        TileKind::Field
    } else if x == 10 && y == 3 {
        TileKind::Stone
    } else {
        TileKind::Meadow
    }
}

#[allow(dead_code)]
fn _assert_serializable<T: Serialize>() {}

#[cfg(test)]
mod tests;
