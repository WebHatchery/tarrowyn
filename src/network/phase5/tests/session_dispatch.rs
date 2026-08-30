use super::super::*;

#[test]
fn refresh_is_scheduled_before_a_production_session_expires() {
    assert_eq!(refresh_delay(0), 1.0);
    assert_eq!(refresh_delay(20), 15.0);
}

#[test]
fn session_only_dispatch_sends_logout_without_releasing_gameplay_queue() {
    let mut client = Phase5Client::new();
    client
        .commands
        .push_back(Phase5Command::Travel(tarrowyn_protocol::TravelRequest {
            request_id: "travel-during-reload".to_owned(),
            action: tarrowyn_protocol::TravelAction::Start,
            route_id: Some("north-pack-road".to_owned()),
            travel_id: None,
        }));
    client.commands.push_back(Phase5Command::Revoke(
        tarrowyn_protocol::AuthRevokeRequest {
            request_id: "logout-during-reload".to_owned(),
            revoke_all: true,
        },
    ));

    let mut api = HttpClient::new("https://example.test");
    client.dispatch_with_mode(&mut api, false, true);

    assert!(client.pending_command.is_some());
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Travel(_))
    ));
    assert!(matches!(
        client.in_flight_command,
        Some(Phase5Command::Revoke(_))
    ));
}
