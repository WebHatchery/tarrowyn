use crate::config::ServerConfig;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use tarrowyn_protocol::{
    ApiMeta, ApiResponse, ChatMessage, ChatRequest, ChatResponse, EventRecord, EventsResponse,
    FarmingRequest, FarmingResponse, FoundationCacheResponse, FoundationFieldToolKind,
    FoundationForgeResponse, FoundationInteractionRequest, FoundationInteractionResponse,
    FoundationResourceResponse, GuestSessionRequest, GuestSessionResponse, HealthResponse,
    Inventory, MovementIntent, MovementResponse, PlayerPresence, PlayerProjection, Position,
    StateSnapshot, TavernFeedResponse, TavernNotice, TradeAction, TradeBundle, TradeOffer,
    TradeRequest, TradeResponse, TradeStatus, TradesResponse, WeaponKind, WorldClock, WorldEvent,
    WorldSnapshot, MAX_CHAT_MESSAGE_LENGTH, MAX_TRADE_ITEMS, PROTOCOL_VERSION,
};

pub(super) const STORAGE_VERSION: u32 = 26;
const MAX_EVENTS: usize = 2048;
const MAX_CHAT_HISTORY: usize = 64;
const MAX_NOTICES: usize = 32;
const MAX_TRADES: usize = 128;
const MAX_CLIENT_KEY_CHARS: usize = 128;
pub(super) const FIELD_TOOL_MAX_CONDITION: u8 = 3;

fn checked_step(current: Position, intent: &MovementIntent) -> Option<Position> {
    Some(Position {
        x: current.x.checked_add(intent.dx)?,
        y: current.y.checked_add(intent.dy)?,
    })
}

mod adventurer;
mod chat;
mod errors;
mod farming;
mod foundation;
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
mod tick;
mod trades;
mod world;
pub(crate) use skills::validate_catalog as validate_skill_catalog;

pub use errors::RepositoryError;
pub(super) use errors::{
    validate_bounded_text, validate_event_cursor, validate_optional_identifier, validate_request_id,
};

use models::{Identity, RepositoryState, Session};
use persistence::PersistenceBackend;
use session::{
    authenticate, expire_sessions, presence, record_offline_presence_if_last_session,
    sorted_presences,
};

pub struct WorldRepository {
    config: ServerConfig,
    storage: PersistenceBackend,
    state: Mutex<RepositoryState>,
    last_persisted_state: Mutex<RepositoryState>,
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
            last_persisted_state: Mutex::new(state.clone()),
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
            let _ = repository.persist(&mut state);
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
        self.expire_and_persist_sessions(&mut state)?;
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
                    field_tool_kind: FoundationFieldToolKind::Crude,
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
                    foundation_resource_results: HashMap::new(),
                    foundation_cache_results: HashMap::new(),
                    foundation_forge_results: HashMap::new(),
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
        self.persist(&mut state)?;
        Ok(response)
    }

    pub fn world(&self, token: &str) -> Result<ApiResponse<WorldSnapshot>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        authenticate(&mut state, token, &self.config)?;
        self.validate_snapshot_configuration()?;
        let players = sorted_presences(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: snapshot(&state, &self.config, players),
        })
    }

    pub fn state(&self, token: &str) -> Result<ApiResponse<StateSnapshot>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        let key = authenticate(&mut state, token, &self.config)?;
        self.validate_snapshot_configuration()?;
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
        self.expire_and_persist_sessions(&mut state)?;
        let key = authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: player_projection(&state, &key),
        })
    }

    pub fn foundation_interaction(
        &self,
        token: &str,
        request: FoundationInteractionRequest,
    ) -> Result<ApiResponse<FoundationInteractionResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let interaction_id = validate_bounded_text(
            &request.interaction_id,
            160,
            "invalid_foundation_interaction",
            "The First Beacon interaction ID must contain 1 to 160 characters and no control characters.",
        )?;

        let baseline = crate::content::foundation_baseline();
        let interaction = baseline
            .interactions
            .iter()
            .find(|interaction| interaction.id == interaction_id)
            .ok_or_else(|| {
                RepositoryError::new(
                    404,
                    "foundation_interaction_not_found",
                    "That First Beacon interaction is not part of the authoritative fixture.",
                )
            })?;
        let landmark = baseline
            .landmarks
            .iter()
            .find(|landmark| landmark.id == interaction.landmark_id)
            .expect("validated foundation interaction references a landmark");
        let position = state
            .identities
            .get(&key)
            .expect("identity exists")
            .position;
        let nearby = position.manhattan_distance(landmark.position) <= 1;
        let (supported, title, message) =
            foundation_interaction_copy(&interaction.id, &landmark.name);
        let accepted = nearby && supported;
        let message = if !nearby {
            format!("Walk beside {} before using this action.", landmark.name)
        } else {
            message
        };

        Ok(ApiResponse {
            meta: meta(
                state.tick,
                Some(request.request_id.clone()),
                Some(state.cursor),
            ),
            data: FoundationInteractionResponse {
                request_id: request.request_id,
                interaction_id,
                landmark_id: landmark.id.clone(),
                accepted,
                title,
                message,
            },
        })
    }

    pub fn movement(
        &self,
        token: &str,
        intent: MovementIntent,
    ) -> Result<ApiResponse<MovementResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
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
        } else if travel_locked {
            response.reason = Some(
                "Your journey is on the regional ledger; tap Recover or wait for arrival before walking."
                    .to_owned(),
            );
        } else if let Some(next) = checked_step(current, &intent) {
            if world::position_in_world(next, self.config.world_width, self.config.world_height)
                .is_none()
            {
                response.reason = Some("The settlement edge blocks that step.".to_owned());
            } else if state
                .identities
                .get(&key)
                .expect("identity exists")
                .knocked_out
            {
                response.reason =
                    Some("You are knocked out; tap Self, Rescuer, or Healer below.".to_owned());
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
                return Ok(ApiResponse {
                    meta: meta(state.tick, Some(intent.request_id), Some(cursor)),
                    data: response,
                });
            }
        } else {
            response.reason = Some("The settlement edge blocks that step.".to_owned());
        }
        state
            .identities
            .get_mut(&key)
            .expect("identity exists")
            .movement_results
            .insert(intent.request_id.clone(), response.clone());
        record_command_outcome(&mut state, response.accepted);
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
        self.expire_and_persist_sessions(&mut state)?;
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
        self.expire_and_persist_sessions(&mut state)?;
        authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: feed(&state),
        })
    }

    fn validate_snapshot_configuration(&self) -> Result<(), RepositoryError> {
        self.config
            .validate_runtime_world_bounds()
            .map_err(|error| {
                RepositoryError::new(
                    500,
                    "invalid_world_configuration",
                    format!("The world snapshot configuration is invalid: {error}"),
                )
            })
    }
}

fn foundation_interaction_copy(
    interaction_id: &str,
    landmark_name: &str,
) -> (bool, String, String) {
    let title = landmark_name.to_owned();
    let message = match interaction_id {
        "arrive-first-beacon" => "The First Beacon is the permanent heart of arrival. Every newcomer begins here, and its light will not fail.".to_owned(),
        "inspect-tent-settlement" => "Canvas shelters ring the beacon. The camp is young, shared, and waiting for players to shape what comes next.".to_owned(),
        "gather-at-beacon-fire" => "The communal fire is open to everyone. Travellers meet here before choosing work of their own.".to_owned(),
        "speak-with-builder" => "Mara: Welcome to the First Beacon. I am setting out the camp's first storehouse. Read the noticeboard beside me to see what the settlement needs.".to_owned(),
        "read-local-needs" => "LOCAL NEED — First storehouse: timber for the frame and stone for a dry foundation. Mara can explain what this shared shelter will become.".to_owned(),
        "borrow-crude-tools" => "The shared rack holds a hand axe and stone pick. Every traveller may use these crude tools for nearby logging and mining without choosing a profession.".to_owned(),
        _ => return (false, title, "That place can be inspected, but its work belongs to a later foundational milestone.".to_owned()),
    };
    (true, title, message)
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
        field_tool_kind: identity.field_tool_kind,
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
