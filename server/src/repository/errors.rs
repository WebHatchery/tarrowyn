use super::models::RepositoryState;
use tarrowyn_protocol::ApiError;

#[derive(Debug, Clone)]
pub struct RepositoryError {
    pub status: u16,
    pub error: ApiError,
}

impl RepositoryError {
    pub(super) fn new(status: u16, code: &str, message: impl Into<String>) -> Self {
        Self {
            status,
            error: ApiError {
                code: code.to_owned(),
                message: message.into(),
            },
        }
    }

    pub(super) fn unauthorized() -> Self {
        Self::new(401, "unauthorized", "A valid guest session is required.")
    }
}

pub fn validate_request_id(request_id: &str) -> Result<(), RepositoryError> {
    if request_id.trim().is_empty()
        || request_id.chars().count() > 64
        || request_id.chars().any(char::is_control)
    {
        Err(RepositoryError::new(
            400,
            "invalid_request_id",
            "Request IDs must contain 1 to 64 characters and no control characters.",
        ))
    } else {
        Ok(())
    }
}

pub fn validate_bounded_text(
    value: &str,
    max_chars: usize,
    code: &'static str,
    message: &'static str,
) -> Result<String, RepositoryError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > max_chars
        || trimmed.chars().any(char::is_control)
    {
        Err(RepositoryError::new(400, code, message))
    } else {
        Ok(trimmed.to_owned())
    }
}

pub fn validate_optional_identifier(
    value: Option<&str>,
    code: &'static str,
    message: &'static str,
) -> Result<Option<String>, RepositoryError> {
    value
        .map(|value| validate_bounded_text(value, 160, code, message))
        .transpose()
}

pub fn validate_event_cursor(
    state: &RepositoryState,
    since: u64,
    stream: &str,
) -> Result<(), RepositoryError> {
    if since > state.cursor {
        return Err(RepositoryError::new(
            409,
            "cursor_ahead",
            format!("The {stream} event cursor is ahead of the settlement."),
        ));
    }
    if state
        .events
        .front()
        .is_some_and(|record| since.saturating_add(1) < record.cursor)
    {
        return Err(RepositoryError::new(
            409,
            "cursor_stale",
            format!(
                "The {stream} event history is no longer retained; reload authoritative state."
            ),
        ));
    }
    Ok(())
}
