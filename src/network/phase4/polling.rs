use super::*;

pub(super) fn accept_projection_cursor(current: &mut u64, incoming: Option<u64>) -> bool {
    let Some(incoming) = incoming else {
        return true;
    };
    if incoming < *current {
        return false;
    }
    *current = incoming;
    true
}

pub(super) fn poll_projection<T, F>(
    pending: &mut Option<Pending<ApiResponse<T>>>,
    dt: f32,
    apply: F,
    notices: &mut Vec<NetworkNotice>,
    label: &str,
) where
    T: serde::de::DeserializeOwned,
    F: FnOnce(ApiResponse<T>),
{
    if let Some(result) = pending
        .as_mut()
        .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
    {
        *pending = None;
        match result {
            Ok(response) => apply(response),
            Err(error) => notices.push(NetworkNotice::Warning(format!(
                "The Phase 4 {label} could not be refreshed: {}",
                short_error(&error)
            ))),
        }
    }
}

pub(super) fn phase4_notice(
    accepted: bool,
    reason: Option<String>,
    success: &str,
    notices: &mut Vec<NetworkNotice>,
) {
    if accepted {
        notices.push(NetworkNotice::Success(success.to_owned()));
    } else if let Some(reason) = reason {
        notices.push(NetworkNotice::Warning(reason));
    }
}

pub(super) fn short_error(error: &str) -> String {
    error
        .lines()
        .next()
        .unwrap_or(error)
        .chars()
        .take(100)
        .collect()
}
