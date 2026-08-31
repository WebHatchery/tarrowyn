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

pub(super) fn poll<T, F>(
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
                "The regional {label} could not be refreshed: {}",
                short_error(&error)
            ))),
        }
    }
}
