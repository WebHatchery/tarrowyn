use super::super::models::{Identity, RepositoryState, MAX_REPLAY_CACHE};

const MAX_REQUEST_ID_CHARS: usize = 64;

pub(super) fn ok(state: &RepositoryState) -> bool {
    state.identities.values().all(identity_ok)
}

fn identity_ok(identity: &Identity) -> bool {
    cache_ok(&identity.farming_results, |response| &response.request_id)
        && cache_ok(&identity.trade_results, |response| &response.request_id)
        && cache_ok(&identity.movement_results, |response| &response.request_id)
        && cache_ok(&identity.chat_results, |response| &response.request_id)
        && cache_ok(&identity.foundation_resource_results, |response| {
            &response.request_id
        })
        && cache_ok(&identity.foundation_cache_results, |response| {
            &response.request_id
        })
        && cache_ok(&identity.foundation_forge_results, |response| {
            &response.request_id
        })
        && cache_ok(&identity.foundation_storehouse_results, |response| {
            &response.request_id
        })
        && cache_ok(&identity.foundation_property_results, |response| {
            &response.request_id
        })
}

fn cache_ok<T>(
    cache: &std::collections::HashMap<String, T>,
    request_id: impl Fn(&T) -> &str,
) -> bool {
    cache.len() <= MAX_REPLAY_CACHE
        && cache.iter().all(|(key, response)| {
            bounded(key) && bounded(request_id(response)) && key == request_id(response)
        })
}

fn bounded(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= MAX_REQUEST_ID_CHARS
        && !value.chars().any(char::is_control)
}
