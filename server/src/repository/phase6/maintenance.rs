use super::super::models::{trim_replay_cache, RepositoryState};
use super::{
    deletion, prune_moderation_cooldowns, trim_audits, trim_auth_link_tokens,
    trim_moderation_reports,
};
use crate::config::ServerConfig;

pub(super) fn run(state: &mut RepositoryState) {
    deletion::process(state);
    trim_replay_cache(&mut state.phase6.auth_link_results);
    trim_auth_link_tokens(&mut state.phase6);
    trim_replay_cache(&mut state.phase6.auth_refresh_results);
    state
        .phase6
        .auth_refresh_accounts
        .retain(|key, _| state.phase6.auth_refresh_results.contains_key(key));
    trim_replay_cache(&mut state.phase6.auth_revoke_results);
    trim_replay_cache(&mut state.phase6.auth_revoke_guest_tokens);
    trim_replay_cache(&mut state.phase6.moderation_results);
    prune_moderation_cooldowns(state);
    trim_replay_cache(&mut state.phase3.request_results);
    trim_replay_cache(&mut state.phase4.request_results);
    trim_replay_cache(&mut state.phase5.request_results);
    trim_replay_cache(&mut state.phase6.request_results);
    trim_replay_cache(&mut state.phase6.deletion_results);
    trim_moderation_reports(&mut state.phase6, super::super::phase4::unix_time_seconds());
    for identity in state.identities.values_mut() {
        trim_replay_cache(&mut identity.farming_results);
        trim_replay_cache(&mut identity.trade_results);
        trim_replay_cache(&mut identity.movement_results);
        trim_replay_cache(&mut identity.chat_results);
    }
    trim_audits(&mut state.phase6.audits);
}

pub(super) fn backup_due(state: &RepositoryState, config: &ServerConfig) -> bool {
    config.backup_interval_ticks > 0 && state.tick.is_multiple_of(config.backup_interval_ticks)
}
