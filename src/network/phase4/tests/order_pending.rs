use super::*;
use tarrowyn_protocol::{ProfessionAction, ProfessionKind, ProfessionRequest};

#[test]
fn order_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = Phase4Client::new();
    let request = ProfessionRequest {
        request_id: "order-queued".to_owned(),
        action: ProfessionAction::CreateOrder,
        order_id: None,
        profession: Some(ProfessionKind::Carpenter),
        capability_id: None,
        service: Some("Repair a field tool".to_owned()),
        timing_score: None,
    };
    client
        .commands
        .push_back(Phase4Command::Profession(request.clone()));

    assert!(client.order_command_pending());
    assert!(!client.queue_cycle("order", "order-duplicate".to_owned()));

    client.commands.clear();
    client.in_flight_command = Some(Phase4Command::Profession(request));
    assert!(client.order_command_pending());
    assert!(!client.queue_cycle("order", "order-in-flight".to_owned()));
}
