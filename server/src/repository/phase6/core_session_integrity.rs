use super::super::models::RepositoryState;
use crate::config::ServerConfig;

const MAX_SESSION_TOKEN_CHARS: usize = 512;

pub(super) fn ok(state: &RepositoryState, _config: &ServerConfig) -> bool {
    state.sessions.iter().all(|(token, session)| {
        bounded(token, MAX_SESSION_TOKEN_CHARS)
            && bounded(&session.client_key, super::super::MAX_CLIENT_KEY_CHARS)
            && bounded(&session.identity_key, super::super::MAX_CLIENT_KEY_CHARS)
            && session.client_key == session.identity_key
            && state.identities.contains_key(&session.identity_key)
            && session.last_seen_tick <= state.tick
            && session
                .last_movement_tick
                .is_none_or(|tick| tick <= state.tick)
            && session.last_chat_tick.is_none_or(|tick| tick <= state.tick)
    })
}

fn bounded(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}
