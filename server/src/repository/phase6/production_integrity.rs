use super::super::models::RepositoryState;
use super::{MAX_AUDITS, MAX_MODERATION_REPORTS, MAX_PENDING_DELETIONS};
use std::collections::HashSet;
use tarrowyn_protocol::{AuthSession, ModerationReportResponse, SupportRepairResponse};

const DELETED_ACCOUNT: &str = "former-resident";
const MAX_ACCOUNT_ID_CHARS: usize = 160;
const MAX_CACHE_KEY_CHARS: usize = 320;
const MAX_TOKEN_CHARS: usize = 512;
const MAX_AUDIT_ACTION_CHARS: usize = 80;
const MAX_AUDIT_TARGET_CHARS: usize = 240;
const MAX_BACKUP_PATH_CHARS: usize = 1_024;

pub(super) fn ok(state: &RepositoryState) -> bool {
    let identity_accounts: HashSet<&str> = state
        .identities
        .values()
        .map(|identity| identity.account_id.as_str())
        .collect();
    accounts_ok(state, &identity_accounts)
        && sessions_ok(state)
        && audits_ok(state, &identity_accounts)
        && moderation_ok(state)
        && replay_caches_ok(state, &identity_accounts)
        && deletion_queue_ok(state)
        && backup_metadata_ok(state)
}

fn accounts_ok(state: &RepositoryState, identity_accounts: &HashSet<&str>) -> bool {
    let mut subjects = HashSet::new();
    let mut identities = HashSet::new();
    let mut maximum_generated_id = 0;
    for (key, account) in &state.phase6.accounts {
        if !bounded(key, MAX_ACCOUNT_ID_CHARS)
            || key != &account.account_id
            || !bounded(&account.account_id, MAX_ACCOUNT_ID_CHARS)
            || !bounded(&account.provider, 80)
            || !bounded(&account.subject, 160)
            || !identity_accounts.contains(account.account_id.as_str())
            || !state.identities.contains_key(&account.identity_key)
            || state
                .identities
                .get(&account.identity_key)
                .is_none_or(|identity| identity.account_id != account.account_id)
            || !identities.insert(account.identity_key.as_str())
            || !subjects.insert((account.provider.as_str(), account.subject.as_str()))
        {
            return false;
        }
        if let Some(number) = generated_number(key, "account-") {
            maximum_generated_id = maximum_generated_id.max(number);
        }
    }
    state.phase6.next_account_id > maximum_generated_id
}

fn sessions_ok(state: &RepositoryState) -> bool {
    let mut refresh_tokens = HashSet::new();
    let mut maximum_generated_id = 0;
    for (token, session) in &state.phase6.sessions {
        if !bounded(token, MAX_TOKEN_CHARS)
            || !bounded(&session.identity_key, MAX_ACCOUNT_ID_CHARS)
            || !bounded(&session.account_id, MAX_ACCOUNT_ID_CHARS)
            || !bounded(&session.refresh_token, MAX_TOKEN_CHARS)
            || !refresh_tokens.insert(session.refresh_token.as_str())
            || session.expires_at_tick == 0
            || session.refresh_expires_at_tick == 0
            || session.refresh_expires_at_tick < session.expires_at_tick
            || !state
                .phase6
                .accounts
                .get(&session.account_id)
                .is_some_and(|account| account.identity_key == session.identity_key)
        {
            return false;
        }
        let current = state.sessions.get(token);
        if session.revoked || session.expires_at_tick <= state.tick {
            if current.is_some() {
                return false;
            }
        } else if current.is_none_or(|current| {
            current.identity_key != session.identity_key
                || current.client_key != session.identity_key
        }) {
            return false;
        }
        if let Some(number) = generated_number(token, "prod-session-") {
            maximum_generated_id = maximum_generated_id.max(number);
        }
    }
    state.phase6.next_session_id > maximum_generated_id
}

fn audits_ok(state: &RepositoryState, identity_accounts: &HashSet<&str>) -> bool {
    if state.phase6.audits.len() > MAX_AUDITS {
        return false;
    }
    let mut audit_ids = HashSet::new();
    state.phase6.audits.iter().all(|audit| {
        bounded(&audit.audit_id, MAX_CACHE_KEY_CHARS)
            && audit_ids.insert(audit.audit_id.as_str())
            && account_or_deleted(&audit.actor_account_id, identity_accounts)
            && bounded(&audit.action, MAX_AUDIT_ACTION_CHARS)
            && bounded(&audit.target, MAX_AUDIT_TARGET_CHARS)
            && matches!(audit.outcome.as_str(), "accepted" | "rejected")
            && audit.tick <= state.tick
            && bounded(&audit.note, 240)
    })
}

fn moderation_ok(state: &RepositoryState) -> bool {
    let phase6 = &state.phase6;
    if phase6.reports.len() > MAX_MODERATION_REPORTS
        || phase6.reports.len() != phase6.report_created_at.len()
        || !phase6
            .reports
            .keys()
            .all(|report_id| phase6.report_created_at.contains_key(report_id))
    {
        return false;
    }
    let reports_ok = phase6.reports.iter().all(|(report_id, report)| {
        report_id == &report.report_id
            && moderation_response_ok(report)
            && report.accepted
            && phase6
                .report_created_at
                .get(report_id)
                .is_some_and(|created_at| *created_at > 0)
    });
    let cooldowns_ok = phase6
        .moderation_last_report_ticks
        .iter()
        .all(|(key, tick)| state.identities.contains_key(key) && *tick <= state.tick);
    reports_ok && cooldowns_ok
}

fn replay_caches_ok(state: &RepositoryState, identity_accounts: &HashSet<&str>) -> bool {
    let phase6 = &state.phase6;
    let link_results_ok = phase6.auth_link_results.iter().all(|(key, response)| {
        bounded(key, MAX_CACHE_KEY_CHARS)
            && bounded(&response.request_id, 64)
            && bounded(&response.account_id, MAX_ACCOUNT_ID_CHARS)
            && bounded(&response.character_id, MAX_ACCOUNT_ID_CHARS)
            && bounded(&response.display_name, 80)
            && phase6
                .accounts
                .get(&response.account_id)
                .is_some_and(|account| {
                    account.provider == response.provider
                        && link_cache_key_matches(key, &response.request_id, &account.identity_key)
                        && state
                            .identities
                            .get(&account.identity_key)
                            .is_some_and(|identity| {
                                identity.character_id == response.character_id
                                    && identity.display_name == response.display_name
                            })
                })
            && auth_session_ok(&response.session)
            && response.linked_guest
    });
    let link_tokens_ok = phase6.auth_link_tokens.iter().all(|(token, identity_key)| {
        bounded(token, MAX_TOKEN_CHARS)
            && bounded(identity_key, MAX_ACCOUNT_ID_CHARS)
            && state.identities.contains_key(identity_key)
            && phase6
                .auth_link_results
                .keys()
                .any(|key| key.starts_with(&format!("{identity_key}:")))
    });
    let refresh_results_ok = phase6.auth_refresh_results.iter().all(|(key, response)| {
        bounded(key, MAX_CACHE_KEY_CHARS)
            && bounded(&response.request_id, 64)
            && refresh_cache_key_matches(key, &response.request_id)
            && auth_session_ok(&response.session)
            && phase6
                .auth_refresh_accounts
                .get(key)
                .is_some_and(|account_id| {
                    state.phase6.accounts.contains_key(account_id)
                        && cached_session_matches_account(state, &response.session, account_id)
                })
    });
    let refresh_accounts_ok = phase6
        .auth_refresh_accounts
        .iter()
        .all(|(key, account_id)| {
            bounded(key, MAX_CACHE_KEY_CHARS)
                && bounded(account_id, MAX_ACCOUNT_ID_CHARS)
                && phase6.auth_refresh_results.contains_key(key)
                && state.phase6.accounts.contains_key(account_id)
        });
    let revoke_results_ok = phase6.auth_revoke_results.iter().all(|(key, response)| {
        bounded(key, MAX_CACHE_KEY_CHARS) && bounded(&response.request_id, 64)
    });
    let support_results_ok = phase6
        .request_results
        .iter()
        .all(|(key, response)| bounded(key, MAX_CACHE_KEY_CHARS) && support_response_ok(response));
    let moderation_results_ok = phase6.moderation_results.iter().all(|(key, response)| {
        bounded(key, MAX_CACHE_KEY_CHARS)
            && moderation_response_ok(response)
            && identity_cache_key_matches(key, "moderation:", &response.request_id, state)
    });
    let account_ids_ok = phase6
        .accounts
        .keys()
        .all(|account_id| identity_accounts.contains(account_id.as_str()));
    link_results_ok
        && link_tokens_ok
        && refresh_results_ok
        && refresh_accounts_ok
        && revoke_results_ok
        && support_results_ok
        && moderation_results_ok
        && account_ids_ok
}

fn deletion_queue_ok(state: &RepositoryState) -> bool {
    if state.phase6.deletion_requests.len() > MAX_PENDING_DELETIONS {
        return false;
    }
    state.phase6.deletion_requests.iter().all(|(key, request)| {
        bounded(key, MAX_CACHE_KEY_CHARS)
            && key == &format!("delete:{}:{}", request.account_id, request.request_id)
            && bounded(&request.request_id, 64)
            && bounded(&request.account_id, MAX_ACCOUNT_ID_CHARS)
            && bounded(&request.identity_key, MAX_ACCOUNT_ID_CHARS)
            && bounded(&request.character_id, MAX_ACCOUNT_ID_CHARS)
            && state.phase6.accounts.contains_key(&request.account_id)
            && state
                .identities
                .get(&request.identity_key)
                .is_some_and(|identity| {
                    identity.account_id == request.account_id
                        && identity.character_id == request.character_id
                })
    })
}

fn backup_metadata_ok(state: &RepositoryState) -> bool {
    state.phase6.next_account_id > 0
        && state.phase6.next_session_id > 0
        && state.phase6.next_audit_id > 0
        && state
            .phase6
            .last_backup_tick
            .is_none_or(|tick| tick <= state.tick)
        && state
            .phase6
            .last_backup_path
            .as_deref()
            .is_none_or(|path| bounded(path, MAX_BACKUP_PATH_CHARS))
        && state.phase6.last_backup_tick.is_some() == state.phase6.last_backup_path.is_some()
}

fn moderation_response_ok(response: &ModerationReportResponse) -> bool {
    bounded(&response.request_id, 64)
        && bounded(&response.report_id, MAX_CACHE_KEY_CHARS)
        && matches!(response.status.as_str(), "queued" | "resolved")
        && response
            .reason
            .as_deref()
            .is_none_or(|reason| bounded(reason, 240))
}

fn support_response_ok(response: &SupportRepairResponse) -> bool {
    bounded(&response.request_id, 64)
        && bounded(&response.audit_id, MAX_CACHE_KEY_CHARS)
        && bounded(&response.summary, 512)
        && response
            .reason
            .as_deref()
            .is_none_or(|reason| bounded(reason, 240))
}

fn auth_session_ok(session: &AuthSession) -> bool {
    bounded(&session.account_token, MAX_TOKEN_CHARS)
        && bounded(&session.refresh_token, MAX_TOKEN_CHARS)
        && session.expires_in_seconds > 0
        && session.expires_at_tick > 0
}

fn link_cache_key_matches(key: &str, request_id: &str, identity_key: &str) -> bool {
    key.strip_prefix(&format!("{identity_key}:")) == Some(request_id)
}

fn refresh_cache_key_matches(key: &str, request_id: &str) -> bool {
    let Some(fingerprint) = key.strip_prefix(&format!("{request_id}:")) else {
        return false;
    };
    fingerprint.len() == 16 && u64::from_str_radix(fingerprint, 16).is_ok()
}

fn cached_session_matches_account(
    state: &RepositoryState,
    session: &AuthSession,
    account_id: &str,
) -> bool {
    state
        .phase6
        .sessions
        .get(&session.account_token)
        .is_none_or(|stored| {
            stored.account_id == account_id
                && stored.refresh_token == session.refresh_token
                && stored.expires_at_tick == session.expires_at_tick
        })
}

fn identity_cache_key_matches(
    key: &str,
    prefix: &str,
    request_id: &str,
    state: &RepositoryState,
) -> bool {
    state
        .identities
        .keys()
        .any(|identity_key| key == format!("{prefix}{identity_key}:{request_id}"))
}

fn account_or_deleted(account_id: &str, identity_accounts: &HashSet<&str>) -> bool {
    bounded(account_id, MAX_ACCOUNT_ID_CHARS)
        && (identity_accounts.contains(account_id) || account_id == DELETED_ACCOUNT)
}

fn bounded(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}

fn generated_number(value: &str, prefix: &str) -> Option<u64> {
    value.strip_prefix(prefix)?.parse::<u64>().ok()
}
