use super::*;

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

pub fn stable_fingerprint(value: &str) -> u64 {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211_u64);
    }
    hash
}

pub fn issue_session(
    state: &mut RepositoryState,
    config: &ServerConfig,
    identity_key: &str,
    account_id: &str,
) -> AuthSession {
    let id = state.phase6.next_session_id;
    state.phase6.next_session_id = state.phase6.next_session_id.saturating_add(1);
    let access = format!("prod-session-{id}");
    let refresh = format!("prod-refresh-{id}");
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
