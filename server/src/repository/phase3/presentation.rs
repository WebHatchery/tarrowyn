use super::*;
use tarrowyn_protocol::{ChronicleResponse, OpportunitiesResponse};

impl WorldRepository {
    pub fn chronicle(
        &self,
        token: &str,
        since: u64,
    ) -> Result<ApiResponse<ChronicleResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        authenticate(&mut state, token, &self.config)?;
        super::super::validate_event_cursor(&state, since, "chronicle")?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: ChronicleResponse {
                entries: state
                    .phase3
                    .chronicle
                    .iter()
                    .filter(|entry| entry.cursor > since)
                    .cloned()
                    .collect(),
                summary: chronicle_summary(&state.phase3.chronicle_archive, since),
                cursor: state.cursor,
            },
        })
    }

    pub fn opportunities(
        &self,
        token: &str,
    ) -> Result<ApiResponse<OpportunitiesResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: OpportunitiesResponse {
                opportunities: state.phase3.households.clone(),
                cursor: state.cursor,
            },
        })
    }
}
