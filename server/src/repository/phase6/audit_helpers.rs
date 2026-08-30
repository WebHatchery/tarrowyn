use super::*;
use sha2::{Digest, Sha256};

const SESSION_TOKEN_BYTES: usize = 32;

pub fn audit_command(
    state: &mut RepositoryState,
    actor: &str,
    action: &str,
    target: &str,
    accepted: bool,
    note: &str,
) {
    audit(
        state,
        actor,
        action,
        target,
        if accepted { "accepted" } else { "rejected" },
        note,
    );
}

pub fn stable_fingerprint(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut fingerprint = String::with_capacity(digest.len() * 2);
    for byte in digest {
        fingerprint.push(HEX[(byte >> 4) as usize] as char);
        fingerprint.push(HEX[(byte & 0x0f) as usize] as char);
    }
    fingerprint
}

pub fn new_session_tokens() -> Result<(String, String), ()> {
    let mut access_bytes = [0_u8; SESSION_TOKEN_BYTES];
    let mut refresh_bytes = [0_u8; SESSION_TOKEN_BYTES];
    getrandom::fill(&mut access_bytes).map_err(|_| ())?;
    getrandom::fill(&mut refresh_bytes).map_err(|_| ())?;
    Ok((
        format!("prod-session-{}", hex_token(&access_bytes)),
        format!("prod-refresh-{}", hex_token(&refresh_bytes)),
    ))
}

fn hex_token(bytes: &[u8; SESSION_TOKEN_BYTES]) -> String {
    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        token.push(HEX[(byte >> 4) as usize] as char);
        token.push(HEX[(byte & 0x0f) as usize] as char);
    }
    token
}

const HEX: &[u8; 16] = b"0123456789abcdef";

pub fn issue_session(
    state: &mut RepositoryState,
    config: &ServerConfig,
    identity_key: &str,
    account_id: &str,
    tokens: (String, String),
) -> AuthSession {
    state.phase6.next_session_id = state.phase6.next_session_id.saturating_add(1);
    let (access, refresh) = tokens;
    let expires = state
        .tick
        .saturating_add(config.production_session_ttl_ticks());
    let refresh_expires = state.tick.saturating_add(config.refresh_ttl_ticks());
    state.phase6.sessions.insert(
        access.clone(),
        ProductionSession {
            identity_key: identity_key.to_owned(),
            account_id: account_id.to_owned(),
            refresh_token: refresh.clone(),
            expires_at_tick: expires,
            refresh_expires_at_tick: refresh_expires,
            revoked: false,
        },
    );
    state.sessions.insert(
        access.clone(),
        Session {
            client_key: identity_key.to_owned(),
            identity_key: identity_key.to_owned(),
            last_seen_tick: state.tick,
            last_movement_tick: None,
            last_chat_tick: None,
        },
    );
    AuthSession {
        account_token: access,
        refresh_token: refresh,
        expires_in_seconds: config.production_session_ttl_seconds,
        expires_at_tick: expires,
    }
}

pub fn audit(
    state: &mut RepositoryState,
    actor: &str,
    action: &str,
    target: &str,
    outcome: &str,
    note: &str,
) -> String {
    let audit_id = format!("audit-{}", state.phase6.next_audit_id);
    state.phase6.next_audit_id = state.phase6.next_audit_id.saturating_add(1);
    state.phase6.audits.push_back(AuditRecord {
        audit_id: audit_id.clone(),
        actor_account_id: actor.to_owned(),
        action: action.to_owned(),
        target: target.to_owned(),
        outcome: outcome.to_owned(),
        tick: state.tick,
        note: note.chars().take(240).collect(),
    });
    trim_audits(&mut state.phase6.audits);
    audit_id
}
