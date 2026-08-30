use super::*;

#[test]
fn maintenance_status_prefers_the_deployment_message_and_has_a_safe_fallback() {
    assert_eq!(
        maintenance_status_message(true, Some("The road opens after dawn.")),
        Some("Maintenance: The road opens after dawn.".to_owned())
    );
    assert_eq!(
        maintenance_status_message(false, None),
        Some("The settlement is in maintenance; tap Reconnect when it is ready.".to_owned())
    );
    assert_eq!(maintenance_status_message(true, None), None);
}

#[test]
fn degraded_readiness_survives_a_later_connection_status_update() {
    let data = crate::data::GameData::load().expect("embedded client data");
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &data.config);
    client.state = super::super::ConnectionState::Online;

    apply_readiness(&mut client, false, None);
    client.status_message = "The persistent settlement is open.".to_owned();
    restore_status(&mut client);

    assert_eq!(
        client.status_message,
        "The settlement is in maintenance; tap Reconnect when it is ready."
    );
    assert_eq!(client.state, super::super::ConnectionState::Degraded);
}

#[test]
fn state_snapshot_does_not_reopen_a_maintenance_gate() {
    assert_eq!(
        state_after_snapshot(true),
        super::super::ConnectionState::Degraded
    );
    assert_eq!(
        state_after_snapshot(false),
        super::super::ConnectionState::Online
    );
}

#[test]
fn healthy_readiness_reopens_a_loaded_world_after_maintenance() {
    let data = crate::data::GameData::load().expect("embedded client data");
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &data.config);
    client.had_world = true;
    client.state = super::super::ConnectionState::Online;

    apply_readiness(&mut client, false, None);
    client.state_refresh = 12.0;
    apply_readiness(&mut client, true, None);

    assert!(!client.readiness_degraded);
    assert_eq!(client.state, super::super::ConnectionState::Online);
    assert_eq!(client.state_refresh, 0.0);
}

#[test]
fn healthy_readiness_does_not_reopen_a_transport_failure() {
    let data = crate::data::GameData::load().expect("embedded client data");
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &data.config);
    client.had_world = true;
    client.state = super::super::ConnectionState::Degraded;
    client.state_refresh = 12.0;

    apply_readiness(&mut client, true, None);

    assert!(!client.readiness_degraded);
    assert_eq!(client.state, super::super::ConnectionState::Degraded);
    assert_eq!(client.state_refresh, 12.0);
}
