use super::*;
use tarrowyn_protocol::{ApiResponse, ModerationReportRequest, ModerationReportResponse};

impl WorldRepository {
    pub fn moderation_report(
        &self,
        token: &str,
        request: ModerationReportRequest,
    ) -> Result<ApiResponse<ModerationReportResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        if request.category.trim().is_empty() || request.note.trim().is_empty() {
            return Err(RepositoryError::new(
                400,
                "invalid_report",
                "A moderation category and note are required.",
            ));
        }
        let cache = format!("moderation:{}:{}", key, request.request_id);
        if let Some(previous) = state.phase6.moderation_results.get(&cache) {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous.clone(),
            });
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
            request.target_account_id.as_deref().unwrap_or("message"),
            "accepted",
            &request.note,
        );
        state
            .phase6
            .moderation_results
            .insert(cache, response.clone());
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
