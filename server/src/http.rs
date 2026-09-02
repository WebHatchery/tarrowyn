use crate::repository::{RepositoryError, WorldRepository};
use serde::Serialize;
use std::io::Read;
#[cfg(test)]
use std::net::IpAddr;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tarrowyn_protocol::{
    AccountDeletionRequest, ApiErrorResponse, ApiMeta, ApiResponse, AuthLinkRequest,
    AuthRefreshRequest, AuthRevokeRequest, ChatRequest, ClaimLifecycleRequest, ClaimRequest,
    CombatRequest, ContractRequest, ExpeditionRequest, FarmingRequest,
    FoundationInteractionRequest, FoundationResourceRequest, GovernanceAction, GovernanceRequest,
    KnowledgeAction, KnowledgeRequest, LocalCombatRequest, MarketOrderRequest,
    ModerationReportRequest, MovementIntent, OpsHealthResponse, ProfessionRequest, RecoveryRequest,
    RegionalEventRequest, RouteRequest, SkillRequest, SupportRepairRequest, TradeRequest,
    TravelRequest, PROTOCOL_VERSION,
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

type JsonResponse = Response<std::io::Cursor<Vec<u8>>>;
const GUEST_SESSION_RETRY_AFTER_SECONDS: &str = "60";
const DEFAULT_MAINTENANCE_MESSAGE: &str =
    "The settlement is in maintenance; try again once service recovers.";

mod pool;
mod rate_limit;
mod request;
#[cfg(test)]
use crate::config::{MAX_HTTP_REQUEST_QUEUE_CAPACITY, MIN_HTTP_REQUEST_QUEUE_CAPACITY};
use pool::{request_queue_capacity, request_worker_count, RequestPoolTelemetry};
#[cfg(test)]
use pool::{MAX_REQUEST_WORKERS, MIN_REQUEST_WORKERS};
use rate_limit::GuestSessionRateLimiter;
#[cfg(test)]
use rate_limit::{GUEST_SESSION_BURST_LIMIT, GUEST_SESSION_RATE_WINDOW, MAX_TRACKED_GUEST_SOURCES};
#[cfg(test)]
pub(super) use request::MAX_REQUEST_BODY_BYTES;
use request::{
    bearer_token, query_cursor, query_value_result, read_json, read_json_or_default,
    request_url_is_bounded, split_url,
};
#[cfg(test)]
use request::{
    parse_bearer_header, read_bounded_body, MAX_BEARER_TOKEN_CHARS, MAX_REQUEST_URL_BYTES,
};

#[cfg(test)]
mod tests;

pub fn serve(config: crate::config::ServerConfig) -> Result<(), String> {
    crate::content::validate().map_err(|error| format!("content validation failed: {error}"))?;
    config.validate_runtime_content_bounds()?;
    let repository = Arc::new(
        WorldRepository::try_new(config.clone())
            .map_err(|error| format!("repository startup failed: {error}"))?,
    );
    let server = Server::http(&config.bind_addr).map_err(|error| error.to_string())?;
    let ticker_repository = Arc::clone(&repository);
    let tick_interval = config.tick_interval;
    thread::spawn(move || {
        let mut next_tick = Instant::now()
            .checked_add(tick_interval)
            .unwrap_or_else(Instant::now);
        loop {
            monotonic_tick_wait(&mut next_tick, tick_interval);
            ticker_repository.tick();
        }
    });
    eprintln!(
        "Tarrowyn server listening on {} (protocol {}, {}ms tick)",
        config.bind_addr,
        PROTOCOL_VERSION,
        config.tick_interval.as_millis()
    );
    let request_worker_count = request_worker_count(config.http_request_workers);
    let request_queue_capacity = request_queue_capacity(config.http_request_queue_capacity);
    let (request_sender, request_receiver) = mpsc::sync_channel(request_queue_capacity);
    let request_receiver = Arc::new(Mutex::new(request_receiver));
    let request_pool_telemetry = Arc::new(RequestPoolTelemetry::default());
    let guest_session_limiter = Arc::new(Mutex::new(GuestSessionRateLimiter::new(
        config.guest_session_burst_limit,
    )));
    let mut workers = Vec::with_capacity(request_worker_count);
    for _ in 0..request_worker_count {
        let request_receiver = Arc::clone(&request_receiver);
        let repository = Arc::clone(&repository);
        let request_pool_telemetry = Arc::clone(&request_pool_telemetry);
        let guest_session_limiter = Arc::clone(&guest_session_limiter);
        workers.push(thread::spawn(move || loop {
            let request = request_receiver
                .lock()
                .expect("request queue lock poisoned")
                .recv();
            let Ok(request) = request else {
                break;
            };
            request_pool_telemetry.record_dequeue();
            request_pool_telemetry.record_request_start();
            handle_request(
                request,
                Arc::clone(&repository),
                &guest_session_limiter,
                &request_pool_telemetry,
                request_worker_count,
                request_queue_capacity,
            );
            request_pool_telemetry.record_request_finish();
        }));
    }
    for request in server.incoming_requests() {
        request_pool_telemetry.record_enqueue();
        match request_sender.try_send(request) {
            Ok(()) => {}
            Err(mpsc::TrySendError::Full(request)) => {
                request_pool_telemetry.record_queue_full();
                if request_sender.send(request).is_err() {
                    request_pool_telemetry.record_dequeue();
                    break;
                }
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                request_pool_telemetry.record_dequeue();
                break;
            }
        }
    }
    drop(request_sender);
    for worker in workers {
        let _ = worker.join();
    }
    Ok(())
}

fn monotonic_tick_wait(deadline: &mut Instant, interval: Duration) {
    let now = Instant::now();
    if *deadline > now {
        thread::sleep(*deadline - now);
    }
    let now = Instant::now();
    *deadline = next_tick_deadline(*deadline, now, interval);
}

fn next_tick_deadline(deadline: Instant, now: Instant, interval: Duration) -> Instant {
    deadline
        .checked_add(interval)
        .filter(|next| *next > now)
        .unwrap_or_else(|| now.checked_add(interval).unwrap_or(now))
}

fn handle_request(
    mut request: Request,
    repository: Arc<WorldRepository>,
    guest_session_limiter: &Mutex<GuestSessionRateLimiter>,
    request_pool_telemetry: &RequestPoolTelemetry,
    request_worker_count: usize,
    request_queue_capacity: usize,
) {
    if !request_url_is_bounded(request.url()) {
        let _ = request.respond(error_response(
            414,
            "uri_too_long",
            "The request URL exceeds the 8 KiB server boundary.".to_owned(),
            repository.health().meta,
        ));
        return;
    }
    if request.method() == &Method::Options {
        let _ = request.respond(with_cors(Response::empty(StatusCode(204))));
        return;
    }
    let (path, query) = split_url(request.url());
    let path = path.to_owned();
    let query = query.to_owned();
    if readiness_required_for_path(&path) {
        if let Some(response) = readiness_response(&repository) {
            let _ = request.respond(response);
            return;
        }
    }
    let result = match (request.method(), path.as_str()) {
        (Method::Get, "/health") => json_response(StatusCode(200), repository.health()),
        (Method::Get, "/v1/ops/health") => json_response(StatusCode(200), repository.ops_health()),
        (Method::Post, "/v1/session/guest") => {
            if !guest_session_limiter
                .lock()
                .expect("guest session limiter lock poisoned")
                .allow(&request)
            {
                rate_limited_response(429, repository.health().meta)
            } else {
                match read_json_or_default(&mut request) {
                    Ok(body) => match repository.guest_session(body) {
                        Ok(response) => json_response(StatusCode(200), response),
                        Err(error) => json_response(
                            StatusCode(error.status),
                            ApiErrorResponse {
                                meta: repository.health().meta,
                                error: error.error,
                            },
                        ),
                    },
                    Err(error) => {
                        error_response(400, "invalid_json", error, repository.health().meta)
                    }
                }
            }
        }
        (Method::Post, "/v1/auth/link") => match read_json::<AuthLinkRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.auth_link(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Post, "/v1/auth/refresh") => match read_json::<AuthRefreshRequest>(&mut request) {
            Ok(body) => match repository.auth_refresh(body) {
                Ok(response) => json_response(StatusCode(200), response),
                Err(error) => json_response(
                    StatusCode(error.status),
                    ApiErrorResponse {
                        meta: repository.health().meta,
                        error: error.error,
                    },
                ),
            },
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Post, "/v1/auth/revoke") => match read_json::<AuthRevokeRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.auth_revoke(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Get, "/v1/account") => {
            authenticated(&request, &repository, |token| repository.account(token))
        }
        (Method::Post, "/v1/account/delete") => {
            match read_json::<AccountDeletionRequest>(&mut request) {
                Ok(body) => authenticated(&request, &repository, |token| {
                    repository.account_delete(token, body)
                }),
                Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
            }
        }
        (Method::Get, "/v1/ops/metrics") => authenticated(&request, &repository, |token| {
            let mut response = repository.ops_metrics(token)?;
            let telemetry = request_pool_telemetry.snapshot();
            response.data.http_request_workers = request_worker_count as u32;
            response.data.http_request_queue_capacity = request_queue_capacity as u32;
            response.data.http_active_requests = telemetry.active_requests;
            response.data.http_queue_depth = telemetry.queue_depth;
            response.data.http_queue_peak = telemetry.queue_peak;
            response.data.http_queue_full_events = telemetry.queue_full_events;
            Ok(response)
        }),
        (Method::Get, "/v1/support/account") => match query_value_result(&query, "account_id") {
            Ok(account_id) => {
                let account_id = account_id.unwrap_or_default();
                authenticated(&request, &repository, |token| {
                    repository.support_account(token, &account_id)
                })
            }
            Err(error) => error_response(400, "invalid_query", error, repository.health().meta),
        },
        (Method::Get, "/v1/world") => {
            authenticated(&request, &repository, |token| repository.world(token))
        }
        (Method::Get, "/v1/state") => {
            authenticated(&request, &repository, |token| repository.state(token))
        }
        (Method::Get, "/v1/inventory") => {
            authenticated(&request, &repository, |token| repository.inventory(token))
        }
        (Method::Post, "/v1/foundation/interactions") => {
            match read_json::<FoundationInteractionRequest>(&mut request) {
                Ok(body) => authenticated(&request, &repository, |token| {
                    repository.foundation_interaction(token, body)
                }),
                Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
            }
        }
        (Method::Post, "/v1/foundation/resources") => {
            match read_json::<FoundationResourceRequest>(&mut request) {
                Ok(body) => authenticated(&request, &repository, |token| {
                    repository.foundation_resource(token, body)
                }),
                Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
            }
        }
        (Method::Get, "/v1/skills") => {
            authenticated(&request, &repository, |token| repository.skills(token))
        }
        (Method::Post, "/v1/skills") => match read_json::<SkillRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.skill_action(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Post, "/v1/movement") => match read_json::<MovementIntent>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.movement(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Get, "/v1/events") => match query_cursor(&query, "since") {
            Ok(since) => authenticated(&request, &repository, |token| {
                repository.events(token, since)
            }),
            Err(error) => error_response(400, "invalid_cursor", error, repository.health().meta),
        },
        (Method::Get, "/v1/region") => {
            authenticated(&request, &repository, |token| repository.region(token))
        }
        (Method::Get, "/v1/settlements") => {
            authenticated(&request, &repository, |token| repository.settlements(token))
        }
        (Method::Get, "/v1/routes") => {
            authenticated(&request, &repository, |token| repository.routes(token))
        }
        (Method::Post, "/v1/routes") => match read_json::<RouteRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.route_action(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Post, "/v1/travel") => match read_json::<TravelRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.travel(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Get, "/v1/market/orders") => {
            authenticated(&request, &repository, |token| repository.market(token))
        }
        (Method::Post, "/v1/market/orders") => {
            match read_json::<MarketOrderRequest>(&mut request) {
                Ok(body) => authenticated(&request, &repository, |token| {
                    repository.market_order(token, body)
                }),
                Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
            }
        }
        (Method::Get, "/v1/events/region") => match query_cursor(&query, "since") {
            Ok(since) => authenticated(&request, &repository, |token| {
                repository.events_region(token, since)
            }),
            Err(error) => error_response(400, "invalid_cursor", error, repository.health().meta),
        },
        (Method::Post, "/v1/events/region") => {
            match read_json::<RegionalEventRequest>(&mut request) {
                Ok(body) => authenticated(&request, &repository, |token| {
                    repository.event_action(token, body)
                }),
                Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
            }
        }
        (Method::Get, "/v1/households/region") => authenticated(&request, &repository, |token| {
            repository.households_region(token)
        }),
        (Method::Get, "/v1/law") => authenticated(&request, &repository, |token| {
            repository.law_boundary(token)
        }),
        (Method::Post, "/v1/chat") => match read_json::<ChatRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| repository.chat(token, body)),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Post, "/v1/moderation/report") => {
            match read_json::<ModerationReportRequest>(&mut request) {
                Ok(body) => authenticated(&request, &repository, |token| {
                    repository.moderation_report(token, body)
                }),
                Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
            }
        }
        (Method::Post, "/v1/farming/actions") => match read_json::<FarmingRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.farming(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Get, "/v1/trades") => {
            authenticated(&request, &repository, |token| repository.trades(token))
        }
        (Method::Post, "/v1/trades") => match read_json::<TradeRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| repository.trade(token, body)),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Get, "/v1/tavern/feed") => {
            authenticated(&request, &repository, |token| repository.tavern_feed(token))
        }
        (Method::Get, "/v1/contracts") => {
            authenticated(&request, &repository, |token| repository.contracts(token))
        }
        (Method::Post, "/v1/contracts/brambleback-watch") => {
            match read_json::<ContractRequest>(&mut request) {
                Ok(body) => authenticated(&request, &repository, |token| {
                    repository.contract(token, body)
                }),
                Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
            }
        }
        (Method::Post, path) if path.starts_with("/v1/contracts/") => {
            match read_json::<ContractRequest>(&mut request) {
                Ok(mut body) => {
                    if body.contract_id.trim().is_empty() {
                        body.contract_id = path.trim_start_matches("/v1/contracts/").to_owned();
                    }
                    authenticated(&request, &repository, |token| {
                        repository.contract(token, body)
                    })
                }
                Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
            }
        }
        (Method::Post, "/v1/combat/actions") => match read_json::<CombatRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.combat(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Post, "/v1/recovery") => match read_json::<RecoveryRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.recovery(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Get, "/v1/settlement/chronicle") => match query_cursor(&query, "since") {
            Ok(since) => authenticated(&request, &repository, |token| {
                repository.chronicle(token, since)
            }),
            Err(error) => error_response(400, "invalid_cursor", error, repository.health().meta),
        },
        (Method::Get, "/v1/settlement/opportunities") => {
            authenticated(&request, &repository, |token| {
                repository.opportunities(token)
            })
        }
        (Method::Get, "/v1/settlement/governance") => {
            let request_id = format!(
                "inspect-governance-{}",
                repository.health().meta.server_tick
            );
            let repository_for_request = Arc::clone(&repository);
            authenticated(&request, &repository, move |token| {
                repository_for_request.governance(
                    token,
                    GovernanceRequest {
                        request_id,
                        action: GovernanceAction::Inspect,
                        office_id: None,
                        proposal_id: None,
                        public_action: None,
                        target: None,
                        cost: None,
                        tax_rate_percent: None,
                    },
                )
            })
        }
        (Method::Post, "/v1/settlement/governance") => {
            match read_json::<GovernanceRequest>(&mut request) {
                Ok(body) => authenticated(&request, &repository, |token| {
                    repository.governance(token, body)
                }),
                Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
            }
        }
        (Method::Get, "/v1/infrastructure") => authenticated(&request, &repository, |token| {
            repository.infrastructure(token)
        }),
        (Method::Get, "/v1/claims") => {
            authenticated(&request, &repository, |token| repository.claims(token))
        }
        (Method::Post, "/v1/claims/lifecycle") => {
            match read_json::<ClaimLifecycleRequest>(&mut request) {
                Ok(body) => authenticated(&request, &repository, |token| {
                    repository.claim_lifecycle(token, body)
                }),
                Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
            }
        }
        (Method::Post, "/v1/claims") => match read_json::<ClaimRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| repository.claim(token, body)),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Get, "/v1/professions") => {
            authenticated(&request, &repository, |token| repository.professions(token))
        }
        (Method::Post, "/v1/professions/orders") => {
            match read_json::<ProfessionRequest>(&mut request) {
                Ok(body) => authenticated(&request, &repository, |token| {
                    repository.profession_order(token, body)
                }),
                Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
            }
        }
        (Method::Get, "/v1/knowledge") => {
            let request_id = format!("inspect-knowledge-{}", repository.health().meta.server_tick);
            let repository_for_request = Arc::clone(&repository);
            authenticated(&request, &repository, move |token| {
                repository_for_request.knowledge(
                    token,
                    KnowledgeRequest {
                        request_id,
                        action: KnowledgeAction::Inspect,
                        knowledge_id: None,
                        target_account_id: None,
                    },
                )
            })
        }
        (Method::Post, "/v1/knowledge") => match read_json::<KnowledgeRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.knowledge(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Get, "/v1/households") => {
            authenticated(&request, &repository, |token| repository.households(token))
        }
        (Method::Get, "/v1/combat/local") => authenticated(&request, &repository, |token| {
            repository.combat_status(token)
        }),
        (Method::Post, "/v1/combat/local") => match read_json::<LocalCombatRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.local_combat(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Post, "/v1/expeditions") => match read_json::<ExpeditionRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.expedition(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Post, "/v1/support/repair") => {
            match read_json::<SupportRepairRequest>(&mut request) {
                Ok(body) => authenticated(&request, &repository, |token| {
                    repository.support_repair(token, body)
                }),
                Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
            }
        }
        (Method::Get, "/v1/chronicle/search") => match query_cursor(&query, "since") {
            Ok(since) => match query_value_result(&query, "q") {
                Ok(search) => {
                    let search = search.unwrap_or_default();
                    authenticated(&request, &repository, |token| {
                        repository.chronicle_search(token, &search, since)
                    })
                }
                Err(error) => error_response(400, "invalid_query", error, repository.health().meta),
            },
            Err(error) => error_response(400, "invalid_cursor", error, repository.health().meta),
        },
        _ => error_response(
            404,
            "not_found",
            "The requested Tarrowyn endpoint does not exist.".to_owned(),
            repository.health().meta,
        ),
    };
    let _ = request.respond(result);
}

fn authenticated<T, F>(
    request: &Request,
    repository: &WorldRepository,
    operation: F,
) -> JsonResponse
where
    T: Serialize,
    F: FnOnce(&str) -> Result<ApiResponse<T>, RepositoryError>,
{
    let Some(token) = bearer_token(request) else {
        return error_response(
            401,
            "unauthorized",
            "A Bearer guest session token is required.".to_owned(),
            repository.health().meta,
        );
    };
    match operation(&token) {
        Ok(response) => json_response(StatusCode(200), response),
        Err(error) => json_response(
            StatusCode(error.status),
            ApiErrorResponse {
                meta: repository.health().meta,
                error: error.error,
            },
        ),
    }
}

fn readiness_required_for_path(path: &str) -> bool {
    path.starts_with("/v1/")
        && !matches!(
            path,
            "/v1/ops/health" | "/v1/ops/metrics" | "/v1/support/account" | "/v1/support/repair"
        )
}

fn readiness_response(repository: &WorldRepository) -> Option<JsonResponse> {
    readiness_error_response(repository.ops_health())
}

fn readiness_error_response(health: ApiResponse<OpsHealthResponse>) -> Option<JsonResponse> {
    if health.data.ready {
        return None;
    }
    let message = health
        .data
        .maintenance_message
        .as_deref()
        .filter(|message| !message.trim().is_empty())
        .unwrap_or(DEFAULT_MAINTENANCE_MESSAGE)
        .to_owned();
    Some(error_response(503, "maintenance", message, health.meta))
}

fn json_response<T: Serialize>(status: StatusCode, value: T) -> JsonResponse {
    let body = serde_json::to_string(&value).unwrap_or_else(|_| {
        "{\"error\":{\"code\":\"serialization\",\"message\":\"The server could not encode this response.\"}}"
            .to_owned()
    });
    with_cors(
        Response::from_string(body)
            .with_status_code(status)
            .with_header(content_type()),
    )
}

fn error_response(status: u16, code: &str, message: String, meta: ApiMeta) -> JsonResponse {
    json_response(
        StatusCode(status),
        ApiErrorResponse {
            meta,
            error: tarrowyn_protocol::ApiError {
                code: code.to_owned(),
                message,
            },
        },
    )
}

fn rate_limited_response(status: u16, meta: ApiMeta) -> JsonResponse {
    error_response(
        status,
        "rate_limited",
        "Too many guest-session attempts from this source; try again shortly.".to_owned(),
        meta,
    )
    .with_header(
        Header::from_bytes("Retry-After", GUEST_SESSION_RETRY_AFTER_SECONDS).expect("valid header"),
    )
}

fn content_type() -> Header {
    Header::from_bytes("Content-Type", "application/json; charset=utf-8").expect("valid header")
}

fn with_cors<R: Read>(response: Response<R>) -> Response<R> {
    response
        .with_header(Header::from_bytes("Cache-Control", "no-store").expect("valid header"))
        .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").expect("valid header"))
        .with_header(
            Header::from_bytes(
                "Access-Control-Allow-Headers",
                "Authorization, Content-Type",
            )
            .expect("valid header"),
        )
        .with_header(
            Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                .expect("valid header"),
        )
}
