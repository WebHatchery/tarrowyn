use crate::repository::{RepositoryError, WorldRepository};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::Read;
use std::sync::Arc;
use std::thread;
use tarrowyn_protocol::{
    ApiErrorResponse, ApiMeta, ApiResponse, ChatRequest, MovementIntent, PROTOCOL_VERSION,
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

type JsonResponse = Response<std::io::Cursor<Vec<u8>>>;

pub fn serve(config: crate::config::ServerConfig) -> Result<(), String> {
    let server = Server::http(&config.bind_addr).map_err(|error| error.to_string())?;
    let repository = Arc::new(WorldRepository::new(config.clone()));
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
    let result = match (request.method(), path) {
        (Method::Get, "/health") => json_response(StatusCode(200), repository.health()),
        (Method::Post, "/v1/session/guest") => match read_json_or_default(&mut request) {
            Ok(body) => json_response(StatusCode(200), repository.guest_session(body)),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Get, "/v1/world") => {
            authenticated(&request, &repository, |token| repository.world(token))
        }
        (Method::Post, "/v1/movement") => match read_json::<MovementIntent>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| {
                repository.movement(token, body)
            }),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
        (Method::Get, "/v1/events") => {
            let since = query_value(query, "since")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            authenticated(&request, &repository, |token| {
                repository.events(token, since)
            })
        }
        (Method::Post, "/v1/chat") => match read_json::<ChatRequest>(&mut request) {
            Ok(body) => authenticated(&request, &repository, |token| repository.chat(token, body)),
            Err(error) => error_response(400, "invalid_json", error, repository.health().meta),
        },
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
    let mut body = String::new();
    request
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|error| format!("Could not read request body: {error}"))?;
    serde_json::from_str(&body).map_err(|error| format!("Could not decode request JSON: {error}"))
}

fn read_json_or_default<T: DeserializeOwned + Default>(request: &mut Request) -> Result<T, String> {
    let mut body = String::new();
    request
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|error| format!("Could not read request body: {error}"))?;
    if body.trim().is_empty() {
        Ok(T::default())
    } else {
        serde_json::from_str(&body)
            .map_err(|error| format!("Could not decode request JSON: {error}"))
    }
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
