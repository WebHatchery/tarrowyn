use super::models::{Identity, RepositoryState};
use super::{RepositoryError, ServerConfig};
use std::collections::{HashMap, HashSet};
use tarrowyn_protocol::PlayerPresence;

pub(super) fn authenticate(
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
            state.tick.saturating_sub(session.last_seen_tick) >= config.session_ttl_ticks()
        });
    if expired {
        state.sessions.remove(token);
        return Err(RepositoryError::unauthorized());
    }
    let key = session.identity_key.clone();
    if !state.identities.contains_key(&key) {
        state.sessions.remove(token);
        return Err(RepositoryError::unauthorized());
    }
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

pub(super) fn expire_sessions(state: &mut RepositoryState, config: &ServerConfig) -> bool {
    let sessions_before = state.sessions.len();
    let phase6_sessions_before = state.phase6.sessions.len();
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
                    state.tick.saturating_sub(session.last_seen_tick) >= config.session_ttl_ticks()
                })
        })
        .map(|(token, _)| token.clone())
        .collect();
    let mut departed_identities = HashSet::new();
    for token in expired {
        if let Some(session) = state.sessions.remove(&token) {
            departed_identities.insert(session.identity_key);
        }
    }
    for identity_key in departed_identities {
        record_offline_presence_if_last_session(state, &identity_key);
    }
    state.phase6.sessions.retain(|token, session| {
        state.sessions.contains_key(token)
            || (!session.revoked && session.refresh_expires_at_tick > state.tick)
    });
    state.sessions.len() != sessions_before || state.phase6.sessions.len() != phase6_sessions_before
}

pub(super) fn sorted_presences(state: &RepositoryState) -> Vec<PlayerPresence> {
    let mut latest_seen_by_identity = HashMap::new();
    for session in state.sessions.values() {
        latest_seen_by_identity
            .entry(session.identity_key.clone())
            .and_modify(|last_seen: &mut u64| *last_seen = (*last_seen).max(session.last_seen_tick))
            .or_insert(session.last_seen_tick);
    }
    let mut players: Vec<_> = latest_seen_by_identity
        .into_iter()
        .filter_map(|(identity_key, last_seen_tick)| {
            state
                .identities
                .get(&identity_key)
                .map(|identity| presence(identity, last_seen_tick, true))
        })
        .collect();
    players.sort_by(|left, right| left.character_id.cmp(&right.character_id));
    players
}

pub(super) fn record_offline_presence_if_last_session(
    state: &mut RepositoryState,
    identity_key: &str,
) {
    if state
        .sessions
        .values()
        .any(|session| session.identity_key == identity_key)
    {
        return;
    }
    if let Some(identity) = state.identities.get(identity_key) {
        let event = super::WorldEvent::Presence(presence(identity, state.tick, false));
        super::push_event(state, event);
    }
}

pub(super) fn presence(identity: &Identity, last_seen_tick: u64, online: bool) -> PlayerPresence {
    PlayerPresence {
        account_id: identity.account_id.clone(),
        character_id: identity.character_id.clone(),
        display_name: identity.display_name.clone(),
        position: identity.position,
        last_seen_tick,
        online,
    }
}

#[cfg(test)]
mod tests;
