use super::super::models::{RepositoryState, MAX_REPLAY_CACHE};
use super::super::phase4::Phase4Response;

const MAX_CACHE_KEY_CHARS: usize = 512;
const MAX_REQUEST_ID_CHARS: usize = 64;
const CACHE_PREFIXES: [&str; 4] = [
    "phase4:",
    "skill-practice:",
    "skill-lesson-begin:",
    "skill-lesson-complete:",
];

pub(super) fn ok(state: &RepositoryState) -> bool {
    let cache = &state.phase4.request_results;
    cache.len() <= MAX_REPLAY_CACHE
        && cache.iter().all(|(key, response)| {
            let request_id = response_request_id(response);
            bounded(key, MAX_CACHE_KEY_CHARS)
                && bounded(request_id, MAX_REQUEST_ID_CHARS)
                && state.identities.values().any(|identity| {
                    CACHE_PREFIXES.iter().any(|prefix| {
                        let prefix = format!("{prefix}{}:", identity.account_id);
                        key.strip_prefix(&prefix) == Some(request_id)
                    })
                })
        })
}

fn response_request_id(response: &Phase4Response) -> &str {
    match response {
        Phase4Response::Governance(response) => &response.request_id,
        Phase4Response::Claim(response) => &response.request_id,
        Phase4Response::Profession(response) => &response.request_id,
        Phase4Response::Knowledge(response) => &response.request_id,
        Phase4Response::Combat(response) => &response.request_id,
        Phase4Response::Skill(response) => &response.request_id,
    }
}

fn bounded(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}
