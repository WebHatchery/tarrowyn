use super::*;
use tarrowyn_protocol::{
    RegionalEventAction, RegionalEventRequest, RegionalEventResponse, RegionalEventsResponse,
};

impl WorldRepository {
    pub fn events_region(
        &self,
        token: &str,
        since: u64,
    ) -> Result<ApiResponse<RegionalEventsResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        authenticate(&mut state, token, &self.config)?;
        super::validate_event_cursor(&state, since, "regional")?;
        if state.phase5.event_history_floor > since {
            return Err(RepositoryError::new(
                409,
                "cursor_stale",
                "The regional event history is no longer retained; reload authoritative state.",
            ));
        }
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: RegionalEventsResponse {
                events: state
                    .phase5
                    .events
                    .iter()
                    .filter(|event| event.cursor > since)
                    .cloned()
                    .collect(),
                cursor: state.cursor,
            },
        })
    }

    pub fn event_action(
        &self,
        token: &str,
        request: RegionalEventRequest,
    ) -> Result<ApiResponse<RegionalEventResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let event_id = super::validate_optional_identifier(
            request.event_id.as_deref(),
            "invalid_event_id",
            "A regional event selector must be bounded and contain no control characters.",
        )?;
        let intervention = super::validate_optional_identifier(
            request.intervention.as_deref(),
            "invalid_intervention",
            "A regional intervention must be bounded and contain no control characters.",
        )?;
        let cache_key = cache_key(&key, &request.request_id);
        if let Some(Phase5Response::Event(response)) = state.phase5.request_results.get(&cache_key)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let (accepted, event, reason) = match request.action {
            RegionalEventAction::Seed => seed_event(&mut state),
            RegionalEventAction::Intervene => {
                intervene_event(&mut state, event_id.as_deref(), intervention.as_deref())
            }
            RegionalEventAction::Resolve => resolve_event(&mut state, event_id.as_deref()),
        };
        let response = RegionalEventResponse {
            request_id: request.request_id.clone(),
            accepted,
            event,
            reason,
        };
        state
            .phase5
            .request_results
            .insert(cache_key, Phase5Response::Event(response.clone()));
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }
}
