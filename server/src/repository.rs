use crate::config::ServerConfig;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tarrowyn_protocol::{
    ApiError, ApiMeta, ApiResponse, ChatMessage, ChatRequest, ChatResponse, CropKind, CropState,
    EventRecord, EventsResponse, FarmPlot, FarmingAction, FarmingRequest, FarmingResponse,
    GuestSessionRequest, GuestSessionResponse, HealthResponse, Inventory, MovementIntent,
    MovementResponse, PlayerPresence, PlayerProjection, Position, StateSnapshot,
    TavernFeedResponse, TavernNotice, TileKind, TradeAction, TradeBundle, TradeOffer, TradeRequest,
    TradeResponse, TradeStatus, TradesResponse, WeaponKind, WorldClock, WorldEvent, WorldSnapshot,
    WorldTile, MAX_CHAT_MESSAGE_LENGTH, MAX_TRADE_ITEMS, PROTOCOL_VERSION,
};

pub(super) const STORAGE_VERSION: u32 = 7;
const MAX_EVENTS: usize = 2048;
const MAX_CHAT_HISTORY: usize = 64;
const MAX_NOTICES: usize = 32;
const MAX_TRADES: usize = 128;

mod farming;
mod models;
mod persistence;
mod phase3;
mod phase3_frontier;
mod phase4;
mod phase5;
mod phase6;
mod skills;
mod trades;
pub(crate) use skills::validate_catalog as validate_skill_catalog;

use models::{Identity, RepositoryState, Session};
use persistence::{load_state, replace_file};

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

pub struct WorldRepository {
    config: ServerConfig,
    state: Mutex<RepositoryState>,
}

impl WorldRepository {
    pub fn new(config: ServerConfig) -> Self {
        let state = load_state(&config).unwrap_or_else(|| RepositoryState::fresh(&config));
        let repository = Self {
            config,
            state: Mutex::new(state),
        };
        let mut state = repository
            .state
            .lock()
            .expect("world repository lock poisoned");
        if state.notices.is_empty() {
            add_notice(
                &mut state,
                "settlement",
                "The Hearth notice board is open; bring useful things to one another.",
            );
            repository.persist(&state);
        }
        drop(state);
        repository
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
                    gold: self.config.starting_gold,
                    skill: 1,
                    reputation: 0,
                    inventory: Inventory {
                        seeds: self.config.starting_seeds,
                        ..Inventory::default()
                    },
                    seeds_planted: 0,
                    last_seen_tick: 0,
                    farming_results: HashMap::new(),
                    trade_results: HashMap::new(),
                    weapon: WeaponKind::IronSword,
                    knocked_out: false,
                    injuries: 0,
                    recovery_cost: 0,
                    skills: models::SkillLedger::default(),
                },
            );
        }
        let old_tokens: Vec<String> = state
            .sessions
            .iter()
            .filter(|(_, session)| session.client_key == client_key)
            .map(|(token, _)| token.clone())
            .collect();
        for token in old_tokens {
            state.sessions.remove(&token);
        }
        let token = format!("dev-session-{}", state.next_token);
        state.next_token += 1;
        let tick = state.tick;
        state
            .identities
            .get_mut(&client_key)
            .expect("identity created")
            .last_seen_tick = tick;
        state.sessions.insert(
            token.clone(),
            Session {
                client_key: client_key.clone(),
                identity_key: client_key.clone(),
                last_seen_tick: tick,
                last_movement_tick: None,
                last_chat_tick: None,
                movement_results: HashMap::new(),
                chat_results: HashMap::new(),
            },
        );
        let event = {
            let identity = state.identities.get(&client_key).expect("identity created");
            WorldEvent::Presence(presence(identity, tick, true))
        };
        let cursor = push_event(&mut state, event);
        let identity = state.identities.get(&client_key).expect("identity created");
        let response = ApiResponse {
            meta: meta(state.tick, None, Some(cursor)),
            data: GuestSessionResponse {
                client_key,
                account_id: identity.account_id.clone(),
                character_id: identity.character_id.clone(),
                display_name: identity.display_name.clone(),
                account_token: token,
                expires_in_seconds: self.config.session_ttl_seconds,
            },
        };
        self.persist(&state);
        response
    }

    pub fn world(&self, token: &str) -> Result<ApiResponse<WorldSnapshot>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        authenticate(&mut state, token, &self.config)?;
        let players = sorted_presences(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: snapshot(&state, &self.config, players),
        })
    }

    pub fn state(&self, token: &str) -> Result<ApiResponse<StateSnapshot>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
        let player = player_projection(state.identities.get(&key).expect("identity exists"));
        let world = snapshot(&state, &self.config, sorted_presences(&state));
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: StateSnapshot {
                world,
                player,
                feed: feed(&state),
                cursor: state.cursor,
            },
        })
    }

    pub fn inventory(&self, token: &str) -> Result<ApiResponse<PlayerProjection>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: player_projection(state.identities.get(&key).expect("identity exists")),
        })
    }

    pub fn movement(
        &self,
        token: &str,
        intent: MovementIntent,
    ) -> Result<ApiResponse<MovementResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
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
        let current = state
            .identities
            .get(&key)
            .expect("identity exists")
            .position;
        let limited = state
            .sessions
            .get(token)
            .and_then(|session| session.last_movement_tick)
            .is_some_and(|last| {
                state.tick.saturating_sub(last) < self.config.movement_cooldown_ticks
            });
        let mut response = MovementResponse {
            request_id: intent.request_id.clone(),
            accepted: false,
            position: current,
            reason: None,
        };
        if intent.dx.abs() + intent.dy.abs() != 1 {
            response.reason = Some("Movement must be one cardinal step.".to_owned());
        } else if limited {
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
            } else if state
                .identities
                .get(&key)
                .expect("identity exists")
                .knocked_out
            {
                response.reason =
                    Some("You are knocked out; choose a recovery prompt first.".to_owned());
            } else if phase3::movement_blocked(&state.phase3, next) {
                response.reason = Some(
                    "The Brambleback has closed the north road; the tavern has posted a contract."
                        .to_owned(),
                );
            } else if !tile_at(next, self.config.world_width, self.config.world_height)
                .is_walkable()
            {
                response.reason = Some("Water blocks that step.".to_owned());
            } else {
                response.accepted = true;
                response.position = next;
                state
                    .identities
                    .get_mut(&key)
                    .expect("identity exists")
                    .position = next;
                state
                    .sessions
                    .get_mut(token)
                    .expect("session exists")
                    .last_movement_tick = Some(state.tick);
                let event = {
                    let identity = state.identities.get(&key).expect("identity exists");
                    WorldEvent::Presence(presence(identity, state.tick, true))
                };
                let cursor = push_event(&mut state, event);
                state
                    .sessions
                    .get_mut(token)
                    .expect("session exists")
                    .movement_results
                    .insert(intent.request_id.clone(), response.clone());
                self.persist(&state);
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
        if since > state.cursor {
            return Err(RepositoryError::new(
                409,
                "cursor_ahead",
                "The requested event cursor is ahead of the settlement.",
            ));
        }
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
        let key = authenticate(&mut state, token, &self.config)?;
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
        let channel = if request.channel.trim().is_empty() {
            "settlement"
        } else {
            request.channel.trim()
        };
        let mut response = ChatResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            message: None,
            reason: None,
        };
        let fast = state
            .sessions
            .get(token)
            .and_then(|session| session.last_chat_tick)
            .is_some_and(|last| last == state.tick);
        if request.request_id.trim().is_empty() || request.request_id.len() > 64 {
            response.reason = Some("Chat request IDs must contain 1 to 64 characters.".to_owned());
        } else if text.is_empty() {
            response.reason = Some("A message cannot be empty.".to_owned());
        } else if text.chars().count() > self.config.chat_max_length.min(MAX_CHAT_MESSAGE_LENGTH) {
            response.reason = Some(format!(
                "Messages are limited to {} characters.",
                self.config.chat_max_length
            ));
        } else if channel.len() > 24 {
            response.reason = Some("That channel name is too long.".to_owned());
        } else if fast {
            response.reason = Some("Give the channel a moment before sending again.".to_owned());
        } else {
            let identity = state.identities.get(&key).expect("identity exists");
            let mut message = ChatMessage {
                message_id: state.next_message,
                account_id: identity.account_id.clone(),
                display_name: identity.display_name.clone(),
                channel: channel.to_owned(),
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
            state.chat_history.push_back(message.clone());
            trim_back(&mut state.chat_history, MAX_CHAT_HISTORY);
            response.accepted = true;
            response.message = Some(message);
            state
                .sessions
                .get_mut(token)
                .expect("session exists")
                .last_chat_tick = Some(state.tick);
            self.persist(&state);
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

    pub fn tavern_feed(
        &self,
        token: &str,
    ) -> Result<ApiResponse<TavernFeedResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: feed(&state),
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
        grow_plots(&mut state, &self.config);
        trades::expire_trades(&mut state);
        phase3::tick(&mut state, &self.config);
        phase4::phase4_tick(&mut state, &self.config);
        let clock = state.clock.clone();
        push_event(&mut state, WorldEvent::Clock(clock));
        expire_sessions(&mut state, &self.config);
        self.persist(&state);
    }

    pub fn server_tick(&self) -> u64 {
        self.state
            .lock()
            .expect("world repository lock poisoned")
            .tick
    }

    fn persist(&self, state: &RepositoryState) {
        let Some(path) = self.config.persistence_path.as_deref() else {
            return;
        };
        let Ok(data) = serde_json::to_vec_pretty(&state.to_stored()) else {
            return;
        };
        let path = Path::new(path);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let temporary_path = path.with_extension(format!(
            "{}-{}",
            path.extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("state"),
            std::process::id()
        ));
        if fs::write(&temporary_path, data).is_ok() && replace_file(&temporary_path, path).is_err()
        {
            let _ = fs::remove_file(temporary_path);
        }
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
    let expired = state
        .phase6
        .sessions
        .get(token)
        .map(|production| production.revoked || production.expires_at_tick <= state.tick)
        .unwrap_or_else(|| {
            state.tick.saturating_sub(session.last_seen_tick) > config.session_ttl_ticks()
        });
    if expired {
        state.sessions.remove(token);
        return Err(RepositoryError::unauthorized());
    }
    let key = session.identity_key.clone();
    state
        .identities
        .get_mut(&key)
        .expect("identity exists")
        .last_seen_tick = state.tick;
    state
        .sessions
        .get_mut(token)
        .expect("session exists")
        .last_seen_tick = state.tick;
    Ok(key)
}

fn expire_sessions(state: &mut RepositoryState, config: &ServerConfig) {
    let expired: Vec<String> = state
        .sessions
        .iter()
        .filter(|(token, session)| {
            state
                .phase6
                .sessions
                .get(*token)
                .map(|production| production.revoked || production.expires_at_tick <= state.tick)
                .unwrap_or_else(|| {
                    state.tick.saturating_sub(session.last_seen_tick) > config.session_ttl_ticks()
                })
        })
        .map(|(token, _)| token.clone())
        .collect();
    for token in expired {
        if let Some(session) = state.sessions.remove(&token) {
            if let Some(identity) = state.identities.get(&session.identity_key) {
                let event = WorldEvent::Presence(presence(identity, state.tick, false));
                push_event(state, event);
            }
        }
    }
}

fn sorted_presences(state: &RepositoryState) -> Vec<PlayerPresence> {
    let mut players: Vec<_> = state
        .sessions
        .values()
        .filter_map(|session| {
            state
                .identities
                .get(&session.identity_key)
                .map(|identity| presence(identity, session.last_seen_tick, true))
        })
        .collect();
    players.sort_by(|left, right| left.character_id.cmp(&right.character_id));
    players
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
fn player_projection(identity: &Identity) -> PlayerProjection {
    PlayerProjection {
        account_id: identity.account_id.clone(),
        character_id: identity.character_id.clone(),
        display_name: identity.display_name.clone(),
        position: identity.position,
        gold: identity.gold,
        skill: identity.skill,
        reputation: identity.reputation,
        inventory: identity.inventory,
        weapon: identity.weapon,
        knocked_out: identity.knocked_out,
        injuries: identity.injuries,
        recovery_cost: identity.recovery_cost,
    }
}
fn snapshot(
    state: &RepositoryState,
    config: &ServerConfig,
    players: Vec<PlayerPresence>,
) -> WorldSnapshot {
    WorldSnapshot {
        width: config.world_width,
        height: config.world_height,
        tiles: world_tiles(config.world_width, config.world_height),
        clock: state.clock.clone(),
        players,
        plots: state.plots.clone(),
        tavern_position: Position { x: 8, y: 5 },
        cursor: state.cursor,
        wilderness: Some(state.phase3.zone.clone()),
        outpost: state.phase3.outpost,
        claim: state.phase3.claim.clone(),
        expedition: state.phase3.expedition.clone(),
    }
}
fn feed(state: &RepositoryState) -> TavernFeedResponse {
    TavernFeedResponse {
        notices: state.notices.iter().cloned().collect(),
        rumours: phase3::rumours(&state.phase3),
        chat: state.chat_history.iter().cloned().collect(),
        cursor: state.cursor,
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
    trim_back(&mut state.events, MAX_EVENTS);
    state.cursor
}
fn add_notice(state: &mut RepositoryState, kind: &str, text: &str) {
    let id = state.next_notice;
    state.next_notice += 1;
    let mut notice = TavernNotice {
        notice_id: id,
        kind: kind.to_owned(),
        text: text.to_owned(),
        created_tick: state.tick,
        cursor: 0,
    };
    let cursor = push_event(state, WorldEvent::TavernNotice(notice.clone()));
    notice.cursor = cursor;
    if let Some(EventRecord {
        event: WorldEvent::TavernNotice(stored),
        ..
    }) = state.events.back_mut()
    {
        *stored = notice.clone();
    }
    state.notices.push_back(notice);
    trim_back(&mut state.notices, MAX_NOTICES);
}
fn farming_notice(action: FarmingAction) -> &'static str {
    match action {
        FarmingAction::Plant => "A new promise is planted in the shared fields.",
        FarmingAction::Tend => "Someone has tended the fields; the next harvest looks steadier.",
        FarmingAction::Harvest => "A fresh crop reaches the Hearth's stores.",
    }
}

fn grow_plots(state: &mut RepositoryState, config: &ServerConfig) {
    let mut changed = Vec::new();
    for plot in &mut state.plots {
        let Some(mut crop) = plot.crop else { continue };
        let age = state.tick.saturating_sub(crop.planted_tick) as f32
            * config.world_seconds_per_tick.max(0.0);
        let stage =
            ((age / config.crop_stage_seconds.max(1.0)).floor() as u8).min(CropState::MATURE_STAGE);
        if stage > crop.stage {
            crop.stage = stage;
            plot.crop = Some(crop);
            changed.push(*plot);
        }
    }
    for plot in changed {
        push_event(state, WorldEvent::Farming(plot));
    }
}

fn farm_plots() -> Vec<FarmPlot> {
    (3..6)
        .flat_map(|x| {
            (4..6).map(move |y| FarmPlot {
                position: Position { x, y },
                crop: None,
            })
        })
        .collect()
}
fn world_tiles(width: u32, height: u32) -> Vec<WorldTile> {
    (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| WorldTile {
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
            })
        })
        .collect()
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
fn trim_back<T>(queue: &mut VecDeque<T>, max: usize) {
    while queue.len() > max {
        queue.pop_front();
    }
}
fn trim_queue<T>(mut queue: VecDeque<T>, max: usize) -> VecDeque<T> {
    trim_back(&mut queue, max);
    queue
}

#[cfg(test)]
mod tests;
