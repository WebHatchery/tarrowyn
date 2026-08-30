use crate::config::ServerConfig;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tarrowyn_protocol::{
    ApiError, ApiMeta, ApiResponse, ChatMessage, ChatRequest, ChatResponse, EventRecord,
    EventsResponse, FarmingRequest, FarmingResponse, GuestSessionRequest, GuestSessionResponse,
    HealthResponse, Inventory, MovementIntent, MovementResponse, PlayerPresence, PlayerProjection,
    Position, StateSnapshot, TavernFeedResponse, TavernNotice, TradeAction, TradeBundle,
    TradeOffer, TradeRequest, TradeResponse, TradeStatus, TradesResponse, WeaponKind, WorldClock,
    WorldEvent, WorldSnapshot, MAX_CHAT_MESSAGE_LENGTH, MAX_TRADE_ITEMS, PROTOCOL_VERSION,
};

pub(super) const STORAGE_VERSION: u32 = 20;
const MAX_EVENTS: usize = 2048;
const MAX_CHAT_HISTORY: usize = 64;
const MAX_NOTICES: usize = 32;
const MAX_TRADES: usize = 128;
const MAX_CLIENT_KEY_CHARS: usize = 128;
pub(super) const FIELD_TOOL_MAX_CONDITION: u8 = 3;

mod adventurer;
mod chat;
mod farming;
mod models;
mod mysql;
mod observability;
mod persistence;
mod phase3;
mod phase3_frontier;
mod phase4;
mod phase5;
mod phase6;
mod recovery;
mod reset;
mod session;
mod skills;
mod trades;
mod world;
pub(crate) use skills::validate_catalog as validate_skill_catalog;

use models::{Identity, RepositoryState, Session};
use persistence::PersistenceBackend;
use session::{authenticate, expire_sessions, presence, sorted_presences};

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
    storage: PersistenceBackend,
    state: Mutex<RepositoryState>,
    persistence_failed: Mutex<bool>,
    backup_failed: Mutex<bool>,
    tick_telemetry: Mutex<TickTelemetry>,
}

#[derive(Debug, Default)]
struct TickTelemetry {
    average_tick_ms: u32,
    last_tick_ms: u32,
    tick_drift_count: u64,
    last_tick_drift: bool,
}

impl WorldRepository {
    pub fn new(config: ServerConfig) -> Self {
        Self::try_new(config).unwrap_or_else(|error| panic!("Tarrowyn storage failed: {error}"))
    }

    pub fn try_new(config: ServerConfig) -> Result<Self, String> {
        let (storage, stored_state) =
            PersistenceBackend::open(&config).map_err(|error| error.to_string())?;
        let state = stored_state.unwrap_or_else(|| RepositoryState::fresh(&config));
        let repository = Self {
            config,
            storage,
            state: Mutex::new(state),
            persistence_failed: Mutex::new(false),
            backup_failed: Mutex::new(false),
            tick_telemetry: Mutex::new(TickTelemetry::default()),
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
        Ok(repository)
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

    pub fn guest_session(
        &self,
        request: GuestSessionRequest,
    ) -> Result<ApiResponse<GuestSessionResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        if request.client_key.as_deref().is_some_and(|key| {
            let trimmed = key.trim();
            !trimmed.is_empty()
                && (trimmed.chars().count() > MAX_CLIENT_KEY_CHARS
                    || trimmed.chars().any(char::is_control))
        }) {
            return Err(RepositoryError::new(
                400,
                "invalid_client_key",
                "The client key must be at most 128 characters and contain no control characters.",
            ));
        }
        let client_key = request
            .client_key
            .map(|key| key.trim().to_owned())
            .filter(|key| !key.trim().is_empty())
            .unwrap_or_else(|| format!("guest-client-{}", state.next_guest));
        if state
            .phase6
            .accounts
            .values()
            .any(|account| account.identity_key == client_key)
        {
            return Err(RepositoryError::new(
                409,
                "production_identity_required",
                "This character is linked to a production identity; provider sign-in is required.",
            ));
        }
        if request.reset {
            reset::reset_guest(&mut state, &client_key);
        }
        if request.reset || !state.identities.contains_key(&client_key) {
            let number = state.next_guest;
            state.next_guest = state.next_guest.saturating_add(1);
            let current_day = state.clock.day;
            state.identities.insert(
                client_key.clone(),
                Identity {
                    account_id: format!("dev-account-{number}"),
                    character_id: format!("dev-character-{number}"),
                    display_name: format!("Guest {number}"),
                    position: crate::content::region_location_profile("hearth").position,
                    gold: self.config.starting_gold,
                    field_tool_condition: FIELD_TOOL_MAX_CONDITION,
                    skill: crate::content::starting_skill(),
                    reputation: 0,
                    inventory: Inventory {
                        seeds: self.config.starting_seeds,
                        bandages: 1,
                        ..Inventory::default()
                    },
                    seeds_planted: 0,
                    last_seen_tick: 0,
                    last_tax_day: current_day,
                    farming_results: HashMap::new(),
                    trade_results: HashMap::new(),
                    movement_results: HashMap::new(),
                    chat_results: HashMap::new(),
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
            .filter(|(token, session)| {
                session.client_key == client_key && !state.phase6.sessions.contains_key(*token)
            })
            .map(|(token, _)| token.clone())
            .collect();
        for token in old_tokens {
            state.sessions.remove(&token);
        }
        let token = format!("dev-session-{}", state.next_token);
        state.next_token = state.next_token.saturating_add(1);
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
        Ok(response)
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
        let player = player_projection(&state, &key);
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
            data: player_projection(&state, &key),
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
        validate_request_id(&intent.request_id)?;
        if let Some(previous) = state
            .identities
            .get(&key)
            .and_then(|identity| identity.movement_results.get(&intent.request_id))
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
        let travel_locked = state.phase5.travel.get(&key).is_some_and(|travel| {
            matches!(
                travel.status,
                tarrowyn_protocol::TravelStatus::Travelling
                    | tarrowyn_protocol::TravelStatus::Interrupted
                    | tarrowyn_protocol::TravelStatus::Recovering
            )
        });
        let mut response = MovementResponse {
            request_id: intent.request_id.clone(),
            accepted: false,
            position: current,
            reason: None,
        };
        if intent
            .dx
            .unsigned_abs()
            .saturating_add(intent.dy.unsigned_abs())
            != 1
        {
            response.reason = Some("Movement must be one cardinal step.".to_owned());
        } else if limited {
            response.reason = Some("Movement is arriving too quickly.".to_owned());
        } else if travel_locked {
            response.reason = Some(
                "Your journey is on the regional ledger; tap Recover or wait for arrival before walking."
                    .to_owned(),
            );
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
            } else if !world::tile_at(next, self.config.world_width, self.config.world_height)
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
                    .identities
                    .get_mut(&key)
                    .expect("identity exists")
                    .movement_results
                    .insert(intent.request_id.clone(), response.clone());
                record_command_outcome(&mut state, response.accepted);
                self.persist(&state);
                return Ok(ApiResponse {
                    meta: meta(state.tick, Some(intent.request_id), Some(cursor)),
                    data: response,
                });
            }
        }
        state
            .identities
            .get_mut(&key)
            .expect("identity exists")
            .movement_results
            .insert(intent.request_id.clone(), response.clone());
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
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
        validate_event_cursor(&state, since, "requested")?;
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
        let started = Instant::now();
        let mut state = self.state.lock().expect("world repository lock poisoned");
        state.tick = state.tick.saturating_add(1);
        let day_length =
            if state.clock.day_length_seconds.is_finite() && state.clock.day_length_seconds > 0.0 {
                state.clock.day_length_seconds
            } else {
                1.0
            };
        let current_seconds = if state.clock.seconds.is_finite() && state.clock.seconds >= 0.0 {
            state.clock.seconds
        } else {
            0.0
        };
        let tick_seconds = if self.config.world_seconds_per_tick.is_finite()
            && self.config.world_seconds_per_tick > 0.0
        {
            self.config.world_seconds_per_tick
        } else {
            0.0
        };
        let elapsed_seconds = (current_seconds + tick_seconds).min(f32::MAX);
        let elapsed_days = (elapsed_seconds / day_length).floor();
        let advanced_days = elapsed_days.min(u32::MAX as f32) as u32;
        if elapsed_days > 0.0 {
            state.clock.seconds = elapsed_seconds % day_length;
            state.clock.day = state.clock.day.saturating_add(advanced_days);
        } else {
            state.clock.seconds = elapsed_seconds;
        }
        if advanced_days > 0 {
            phase4::day_rollover(&mut state, advanced_days);
        }
        world::grow_plots(&mut state, &self.config);
        trades::expire_trades(&mut state);
        phase3::tick(&mut state, &self.config);
        if let Some(backup_ok) = phase4::phase4_tick(&mut state, &self.config) {
            *self
                .backup_failed
                .lock()
                .expect("backup status lock poisoned") = !backup_ok;
        }
        let clock = state.clock.clone();
        push_event(&mut state, WorldEvent::Clock(clock));
        expire_sessions(&mut state, &self.config);
        self.persist(&state);
        drop(state);
        self.record_tick_duration(started.elapsed());
    }

    pub fn server_tick(&self) -> u64 {
        self.state
            .lock()
            .expect("world repository lock poisoned")
            .tick
    }

    fn persist(&self, state: &RepositoryState) {
        match self.storage.persist(state, &self.config) {
            Ok(()) => {
                *self
                    .persistence_failed
                    .lock()
                    .expect("persistence status lock poisoned") = false;
            }
            Err(error) => {
                eprintln!("Tarrowyn persistence write failed: {error}");
                *self
                    .persistence_failed
                    .lock()
                    .expect("persistence status lock poisoned") = true;
            }
        }
    }

    fn record_tick_duration(&self, elapsed: Duration) {
        let elapsed_ms = elapsed.as_millis().min(u128::from(u32::MAX)) as u32;
        let budget_ms = self
            .config
            .tick_interval
            .as_millis()
            .max(1)
            .min(u128::from(u32::MAX)) as u32;
        let mut telemetry = self
            .tick_telemetry
            .lock()
            .expect("tick telemetry lock poisoned");
        telemetry.last_tick_ms = elapsed_ms;
        telemetry.average_tick_ms = if telemetry.average_tick_ms == 0 {
            elapsed_ms
        } else {
            (u64::from(telemetry.average_tick_ms)
                .saturating_mul(7)
                .saturating_add(u64::from(elapsed_ms))
                / 8) as u32
        };
        telemetry.last_tick_drift = elapsed_ms > budget_ms;
        if telemetry.last_tick_drift {
            telemetry.tick_drift_count = telemetry.tick_drift_count.saturating_add(1);
        }
    }
}

pub(super) fn record_command_outcome(state: &mut RepositoryState, accepted: bool) {
    let counter = if accepted {
        &mut state.phase6.completed_commands
    } else {
        &mut state.phase6.rejected_commands
    };
    *counter = counter.saturating_add(1);
}

pub(super) fn player_projection(state: &RepositoryState, key: &str) -> PlayerProjection {
    let identity = state.identities.get(key).expect("identity exists");
    let (adventurer_rank, adventurer_credentials) = adventurer::profile(state, key);
    PlayerProjection {
        account_id: identity.account_id.clone(),
        character_id: identity.character_id.clone(),
        display_name: identity.display_name.clone(),
        position: identity.position,
        gold: identity.gold,
        field_tool_condition: identity.field_tool_condition,
        field_weather: world::field_weather_for_day(state.clock.day),
        field_pest_pressure: world::field_pest_pressure_for_day(state.clock.day),
        animal_condition: state
            .phase4
            .animals
            .first()
            .map(|animal| animal.condition)
            .unwrap_or(0),
        animal_max_condition: state
            .phase4
            .animals
            .first()
            .map(|animal| animal.max_condition)
            .unwrap_or(0),
        skill: identity.skill,
        reputation: identity.reputation,
        adventurer_rank,
        adventurer_credentials,
        inventory: identity.inventory,
        weapon: identity.weapon,
        knocked_out: identity.knocked_out,
        injuries: identity.injuries,
        recovery_cost: identity.recovery_cost,
    }
}
pub(super) fn validate_request_id(request_id: &str) -> Result<(), RepositoryError> {
    if request_id.trim().is_empty()
        || request_id.chars().count() > 64
        || request_id.chars().any(char::is_control)
    {
        Err(RepositoryError::new(
            400,
            "invalid_request_id",
            "Request IDs must contain 1 to 64 characters and no control characters.",
        ))
    } else {
        Ok(())
    }
}

pub(super) fn validate_bounded_text(
    value: &str,
    max_chars: usize,
    code: &'static str,
    message: &'static str,
) -> Result<String, RepositoryError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > max_chars
        || trimmed.chars().any(char::is_control)
    {
        Err(RepositoryError::new(400, code, message))
    } else {
        Ok(trimmed.to_owned())
    }
}

pub(super) fn validate_optional_identifier(
    value: Option<&str>,
    code: &'static str,
    message: &'static str,
) -> Result<Option<String>, RepositoryError> {
    value
        .map(|value| validate_bounded_text(value, 160, code, message))
        .transpose()
}

pub(super) fn validate_event_cursor(
    state: &RepositoryState,
    since: u64,
    stream: &str,
) -> Result<(), RepositoryError> {
    if since > state.cursor {
        return Err(RepositoryError::new(
            409,
            "cursor_ahead",
            format!("The {stream} event cursor is ahead of the settlement."),
        ));
    }
    if state
        .events
        .front()
        .is_some_and(|record| since.saturating_add(1) < record.cursor)
    {
        return Err(RepositoryError::new(
            409,
            "cursor_stale",
            format!(
                "The {stream} event history is no longer retained; reload authoritative state."
            ),
        ));
    }
    Ok(())
}

fn snapshot(
    state: &RepositoryState,
    config: &ServerConfig,
    players: Vec<PlayerPresence>,
) -> WorldSnapshot {
    observability::snapshot(state, config, players)
}

fn feed(state: &RepositoryState) -> TavernFeedResponse {
    observability::feed(state)
}

fn meta(tick: u64, request_id: Option<String>, cursor: Option<u64>) -> ApiMeta {
    observability::meta(tick, request_id, cursor)
}

fn push_event(state: &mut RepositoryState, event: WorldEvent) -> u64 {
    observability::push_event(state, event)
}

fn add_notice(state: &mut RepositoryState, kind: &str, text: &str) {
    observability::add_notice(state, kind, text);
}

fn trim_back<T>(queue: &mut VecDeque<T>, max: usize) {
    observability::trim_back(queue, max);
}

fn trim_queue<T>(queue: VecDeque<T>, max: usize) -> VecDeque<T> {
    observability::trim_queue(queue, max)
}

#[cfg(test)]
mod tests;
