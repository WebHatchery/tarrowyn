use super::models::{Identity, RepositoryState};
use super::{RepositoryError, ServerConfig};
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
            state.tick.saturating_sub(session.last_seen_tick) > config.session_ttl_ticks()
        });
    if expired {
        state.sessions.remove(token);
        return Err(RepositoryError::unauthorized());
    }
    let key = session.identity_key.clone();
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

pub(super) fn expire_sessions(state: &mut RepositoryState, config: &ServerConfig) {
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
                    state.tick.saturating_sub(session.last_seen_tick) > config.session_ttl_ticks()
                })
        })
        .map(|(token, _)| token.clone())
        .collect();
    for token in expired {
        if let Some(session) = state.sessions.remove(&token) {
            if let Some(identity) = state.identities.get(&session.identity_key) {
                let event = super::WorldEvent::Presence(presence(identity, state.tick, false));
                super::push_event(state, event);
            }
        }
    }
}

pub(super) fn sorted_presences(state: &RepositoryState) -> Vec<PlayerPresence> {
    let mut players: Vec<_> = state
        .sessions
        .values()
        .filter_map(|session| {
            state
                .identities
                .get(&session.identity_key)
                .map(|identity| presence(identity, session.last_seen_tick, true))
        })
        .collect();
    players.sort_by(|left, right| left.character_id.cmp(&right.character_id));
    players
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
