use super::*;
use tarrowyn_protocol::{KnowledgeAction, KnowledgeRequest};

#[test]
fn knowledge_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = Phase4Client::new();
    let request = KnowledgeRequest {
        request_id: "knowledge-queued".to_owned(),
        action: KnowledgeAction::Discover,
        knowledge_id: Some("moonberry-tending".to_owned()),
        target_account_id: None,
    };
    client
        .commands
        .push_back(Phase4Command::Knowledge(request.clone()));

    assert!(client.knowledge_command_pending());
    assert!(!client.queue_cycle("knowledge", "knowledge-duplicate".to_owned()));

    client.commands.clear();
    client.in_flight_command = Some(Phase4Command::Knowledge(request));
    assert!(client.knowledge_command_pending());
    assert!(!client.queue_knowledge("knowledge-in-flight".to_owned(), None));
}
