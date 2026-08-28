use crate::repository::{RepositoryError, WorldRepository};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::Read;
use std::sync::Arc;
use std::thread;
use tarrowyn_protocol::{
    ApiErrorResponse, ApiMeta, ApiResponse, AuthLinkRequest, AuthRefreshRequest, AuthRevokeRequest,
    ChatRequest, ClaimLifecycleRequest, ClaimRequest, CombatRequest, ContractRequest,
    ExpeditionRequest, FarmingRequest, GovernanceAction, GovernanceRequest, KnowledgeAction,
    KnowledgeRequest, LocalCombatRequest, MarketOrderRequest, ModerationReportRequest,
    MovementIntent, ProfessionRequest, RecoveryRequest, RegionalEventRequest, RouteRequest,
    SkillRequest, SupportRepairRequest, TradeRequest, TravelRequest, PROTOCOL_VERSION,
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

type JsonResponse = Response<std::io::Cursor<Vec<u8>>>;
const MAX_REQUEST_BODY_BYTES: usize = 64 * 1024;

#[cfg(test)]
mod tests;

pub fn serve(config: crate::config::ServerConfig) -> Result<(), String> {
    crate::content::validate().map_err(|error| format!("content validation failed: {error}"))?;
    let repository = Arc::new(
        WorldRepository::try_new(config.clone())
            .map_err(|error| format!("repository startup failed: {error}"))?,
    );
    let server = Server::http(&config.bind_addr).map_err(|error| error.to_string())?;
    let ticker_repository = Arc::clone(&repository);
    thread::spawn(move || loop {
        thread::sleep(config.tick_interval);
        ticker_repository.tick();
    });
    eprintln!(
        "Tarrowyn server listening on {} (protocol {}, {}ms tick)",
        config.bind_addr,
        PROTOCOL_VERSION,
        config.tick_interval.as_millis()
    );
    for request in server.incoming_requests() {
        handle_request(request, Arc::clone(&repository));
    }
    Ok(())
}

fn handle_request(mut request: Request, repository: Arc<WorldRepository>) {
    if request.method() == &Method::Options {
        let _ = request.respond(with_cors(Response::empty(StatusCode(204))));
        return;
    }
    let (path, query) = split_url(request.url());
    let path = path.to_owned();
    let query = query.to_owned();
    let result = match (request.method(), path.as_str()) {
        (Method::Get, "/health") => json_response(StatusCode(200), repository.health()),
        (Method::Get, "/v1/ops/health") => json_response(StatusCode(200), repository.ops_health()),
        (Method::Post, "/v1/session/guest") => match read_json_or_default(&mut request) {
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
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
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
        (Method::Get, "/v1/ops/metrics") => {
            authenticated(&request, &repository, |token| repository.ops_metrics(token))
        }
        (Method::Get, "/v1/world") => {
            authenticated(&request, &repository, |token| repository.world(token))
        }
        (Method::Get, "/v1/state") => {
            authenticated(&request, &repository, |token| repository.state(token))
        }
        (Method::Get, "/v1/inventory") => {
            authenticated(&request, &repository, |token| repository.inventory(token))
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
        (Method::Get, "/v1/events") => {
            let since = query_value(&query, "since")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            authenticated(&request, &repository, |token| {
                repository.events(token, since)
            })
        }
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
        (Method::Get, "/v1/events/region") => {
            let since = query_value(&query, "since")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            authenticated(&request, &repository, |token| {
                repository.events_region(token, since)
            })
        }
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
        (Method::Get, "/v1/settlement/chronicle") => {
            let since = query_value(&query, "since")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            authenticated(&request, &repository, |token| {
                repository.chronicle(token, since)
            })
        }
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
        (Method::Get, "/v1/chronicle/search") => {
            let since = query_value(&query, "since")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let search = query_value(&query, "q").unwrap_or("");
            authenticated(&request, &repository, |token| {
                repository.chronicle_search(token, search, since)
            })
        }
        _ => error_response(
            404,
            "not_found",
            "The requested Tarrowyn endpoint does not exist.".to_owned(),
            repository.health().meta,
        ),
    };
    let _ = request.respond(with_cors(result));
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

fn read_json<T: DeserializeOwned>(request: &mut Request) -> Result<T, String> {
    let mut reader = request.as_reader();
    let body = read_bounded_body(&mut reader)?;
    serde_json::from_str(&body).map_err(|error| format!("Could not decode request JSON: {error}"))
}

fn read_json_or_default<T: DeserializeOwned + Default>(request: &mut Request) -> Result<T, String> {
    let mut reader = request.as_reader();
    let body = read_bounded_body(&mut reader)?;
    if body.trim().is_empty() {
        Ok(T::default())
    } else {
        serde_json::from_str(&body)
            .map_err(|error| format!("Could not decode request JSON: {error}"))
    }
}

fn read_bounded_body<R: Read>(reader: &mut R) -> Result<String, String> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_REQUEST_BODY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read request body: {error}"))?;
    if bytes.len() > MAX_REQUEST_BODY_BYTES {
        return Err(format!(
            "Request body exceeds {MAX_REQUEST_BODY_BYTES} bytes."
        ));
    }
    String::from_utf8(bytes).map_err(|_| "Request body must be valid UTF-8.".to_owned())
}

fn bearer_token(request: &Request) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Authorization"))
        .and_then(|header| header.value.as_str().strip_prefix("Bearer "))
        .map(str::to_owned)
}

fn split_url(url: &str) -> (&str, &str) {
    url.split_once('?').unwrap_or((url, ""))
}

fn query_value<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn json_response<T: Serialize>(status: StatusCode, value: T) -> JsonResponse {
    let body = serde_json::to_string(&value).unwrap_or_else(|error| {
        format!("{{\"error\":{{\"code\":\"serialization\",\"message\":\"{error}\"}}}}")
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

fn content_type() -> Header {
    Header::from_bytes("Content-Type", "application/json; charset=utf-8").expect("valid header")
}

fn with_cors<R: Read>(response: Response<R>) -> Response<R> {
    response
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
