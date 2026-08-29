use super::super::models::{RepositoryState, MAX_REPLAY_CACHE};
use super::super::phase5::Phase5Response;

const MAX_CACHE_KEY_CHARS: usize = 512;
const MAX_REQUEST_ID_CHARS: usize = 64;

pub(super) fn ok(state: &RepositoryState) -> bool {
    let cache = &state.phase5.request_results;
    cache.len() <= MAX_REPLAY_CACHE
        && cache.iter().all(|(key, response)| {
            let request_id = response_request_id(response);
            bounded(key, MAX_CACHE_KEY_CHARS)
                && bounded(request_id, MAX_REQUEST_ID_CHARS)
                && state.identities.keys().any(|identity_key| {
                    let prefix = format!("phase5:{identity_key}:");
                    key.strip_prefix(&prefix) == Some(request_id)
                })
        })
}

fn response_request_id(response: &Phase5Response) -> &str {
    match response {
        Phase5Response::Travel(response) => &response.request_id,
        Phase5Response::Route(response) => &response.request_id,
        Phase5Response::Market(response) => &response.request_id,
        Phase5Response::Event(response) => &response.request_id,
    }
}

fn bounded(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}
