use super::*;
use tarrowyn_protocol::{ApiResponse, ModerationReportRequest, ModerationReportResponse};

const MAX_MODERATION_CATEGORY_CHARS: usize = 40;
const MAX_MODERATION_NOTE_CHARS: usize = 240;
const MAX_ACCOUNT_ID_CHARS: usize = 160;

impl WorldRepository {
    pub fn moderation_report(
        &self,
        token: &str,
        request: ModerationReportRequest,
    ) -> Result<ApiResponse<ModerationReportResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        validate_bounded_text(
            &request.category,
            MAX_MODERATION_CATEGORY_CHARS,
            "invalid_report",
            "A moderation category is required, bounded, and must contain no control characters.",
        )?;
        let note = validate_bounded_text(
            &request.note,
            MAX_MODERATION_NOTE_CHARS,
            "invalid_report",
            "A moderation note is required, bounded, and must contain no control characters.",
        )?;
        let target_account_id = request
            .target_account_id
            .as_deref()
            .map(|target| {
                validate_bounded_text(
                    target,
                    MAX_ACCOUNT_ID_CHARS,
                    "invalid_report_target",
                    "A moderation target account ID must be bounded and contain no control characters.",
                )
            })
            .transpose()?;
        let cache = format!("moderation:{}:{}", key, request.request_id);
        if let Some(previous) = state.phase6.moderation_results.get(&cache) {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous.clone(),
            });
        }
        if state
            .phase6
            .moderation_last_report_ticks
            .get(&key)
            .is_some_and(|last| {
                state.tick.saturating_sub(*last) < self.config.moderation_cooldown_ticks
            })
        {
            return Err(RepositoryError::new(
                429,
                "moderation_rate_limited",
                "Wait before submitting another moderation report.",
            ));
        }
        let report_id = format!("report-{}", state.phase6.next_audit_id);
        state.phase6.next_audit_id = state.phase6.next_audit_id.saturating_add(1);
        let response = ModerationReportResponse {
            request_id: request.request_id,
            accepted: true,
            report_id: report_id.clone(),
            status: "queued".to_owned(),
            reason: None,
        };
        state.phase6.reports.insert(report_id, response.clone());
        state.phase6.report_created_at.insert(
            response.report_id.clone(),
            super::super::phase4::unix_time_seconds(),
        );
        let actor = state
            .identities
            .get(&key)
            .expect("identity exists")
            .account_id
            .clone();
        audit(
            &mut state,
            &actor,
            "moderation.report",
            target_account_id.as_deref().unwrap_or("message"),
            "accepted",
            &note,
        );
        state
            .phase6
            .moderation_results
            .insert(cache, response.clone());
        let report_tick = state.tick;
        state
            .phase6
            .moderation_last_report_ticks
            .insert(key, report_tick);
        super::super::record_command_outcome(&mut state, true);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(
                state.tick,
                Some(response.request_id.clone()),
                Some(state.cursor),
            ),
            data: response,
        })
    }
}
