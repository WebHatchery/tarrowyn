use super::{OnlineClient, REQUEST_TIMEOUT_SECONDS};

pub(super) fn poll_ops_health(client: &mut OnlineClient, dt: f32) {
    let result = client
        .pending_ops_health
        .as_mut()
        .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
    let Some(result) = result else { return };
    client.pending_ops_health = None;
    let Ok(response) = result else { return };
    if let Some(message) = maintenance_status_message(
        response.data.ready,
        response.data.maintenance_message.as_deref(),
    ) {
        client.status_message = message;
    }
}

fn maintenance_status_message(ready: bool, maintenance_message: Option<&str>) -> Option<String> {
    maintenance_message
        .filter(|message| !message.trim().is_empty())
        .map(|message| format!("Maintenance: {message}"))
        .or_else(|| {
            (!ready).then(|| {
                "The settlement is in maintenance; tap Reconnect when it is ready.".to_owned()
            })
        })
}

#[cfg(test)]
mod tests;
