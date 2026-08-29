use super::super::models::{RepositoryState, MAX_REPLAY_CACHE};
use super::{Phase6State, MAX_AUDITS, MAX_MODERATION_REPORTS, MODERATION_REPORT_RETENTION_SECONDS};
use std::collections::VecDeque;
use tarrowyn_protocol::AuditRecord;

pub(super) fn trim_auth_link_tokens(phase6: &mut Phase6State) {
    phase6.auth_link_tokens.retain(|_, identity_key| {
        phase6
            .auth_link_results
            .keys()
            .any(|key| key.starts_with(&format!("{identity_key}:")))
    });
    while phase6.auth_link_tokens.len() > MAX_REPLAY_CACHE {
        let Some(token) = phase6.auth_link_tokens.keys().next().cloned() else {
            break;
        };
        phase6.auth_link_tokens.remove(&token);
    }
}

pub(super) fn trim_audits(audits: &mut VecDeque<AuditRecord>) {
    while audits.len() > MAX_AUDITS {
        audits.pop_front();
    }
}

pub(super) fn trim_moderation_reports(phase6: &mut Phase6State, now: u64) {
    for report_id in phase6.reports.keys() {
        phase6
            .report_created_at
            .entry(report_id.clone())
            .or_insert(now);
    }
    phase6
        .reports
        .retain(|report_id, _| phase6.report_created_at.contains_key(report_id));
    phase6.report_created_at.retain(|report_id, created_at| {
        phase6.reports.contains_key(report_id)
            && now.saturating_sub(*created_at) < MODERATION_REPORT_RETENTION_SECONDS
    });
    phase6
        .reports
        .retain(|report_id, _| phase6.report_created_at.contains_key(report_id));
    while phase6.reports.len() > MAX_MODERATION_REPORTS {
        let Some((report_id, _)) = phase6
            .report_created_at
            .iter()
            .min_by_key(|(_, created_at)| *created_at)
            .map(|(report_id, created_at)| (report_id.clone(), *created_at))
        else {
            break;
        };
        phase6.reports.remove(&report_id);
        phase6.report_created_at.remove(&report_id);
    }
}

pub(super) fn prune_moderation_cooldowns(state: &mut RepositoryState) {
    state
        .phase6
        .moderation_last_report_ticks
        .retain(|identity_key, _| state.identities.contains_key(identity_key));
}
