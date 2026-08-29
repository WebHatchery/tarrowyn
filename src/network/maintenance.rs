use super::{OnlineClient, REQUEST_TIMEOUT_SECONDS};

pub(super) fn poll_ops_health(client: &mut OnlineClient, dt: f32) {
    let result = client
        .pending_ops_health
        .as_mut()
        .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
    let Some(result) = result else { return };
    client.pending_ops_health = None;
    let Ok(response) = result else { return };
    apply_readiness(
        client,
        response.data.ready,
        response.data.maintenance_message.as_deref(),
    );
}

fn apply_readiness(client: &mut OnlineClient, ready: bool, maintenance_message: Option<&str>) {
    client.readiness_degraded = !ready;
    client.maintenance_status = maintenance_status_message(ready, maintenance_message);
    if let Some(message) = client.maintenance_status.clone() {
        client.status_message = message;
    }
    if !ready {
        client.state = super::ConnectionState::Degraded;
    }
}

pub(super) fn restore_status(client: &mut OnlineClient) {
    if let Some(message) = client.maintenance_status.clone() {
        client.status_message = message;
    }
    if client.readiness_degraded {
        client.state = super::ConnectionState::Degraded;
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
