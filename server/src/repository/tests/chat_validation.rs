use super::{guest, repo};
use tarrowyn_protocol::ChatRequest;

#[test]
fn chat_rejects_control_characters_before_recording_history() {
    let repository = repo();
    let session = guest(&repository, "chat-input-validation");
    for (request_id, channel, text) in [
        ("chat-control-text", "settlement", "Hello\u{7}there"),
        ("chat-control-channel", "settle\nment", "Hello there"),
    ] {
        let response = repository
            .chat(
                &session.account_token,
                ChatRequest {
                    request_id: request_id.to_owned(),
                    channel: channel.to_owned(),
                    text: text.to_owned(),
                },
            )
            .expect("chat response")
            .data;
        assert!(!response.accepted);
        assert!(response
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("control characters")));
    }
}

#[test]
fn chat_message_id_stays_at_the_numeric_ceiling() {
    let repository = repo();
    let session = guest(&repository, "chat-message-ceiling");
    {
        let mut state = repository.state.lock().expect("world repository lock");
        state.next_message = u64::MAX;
    }

    let response = repository
        .chat(
            &session.account_token,
            ChatRequest {
                request_id: "chat-message-ceiling-request".to_owned(),
                channel: "settlement".to_owned(),
                text: "The ledger keeps this message.".to_owned(),
            },
        )
        .expect("chat response")
        .data;

    assert_eq!(
        response.message.expect("accepted chat message").message_id,
        u64::MAX
    );
}
