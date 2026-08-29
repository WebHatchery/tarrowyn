use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{ChatRequest, WorldEvent};

#[test]
fn malformed_retained_notice_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.notices.back_mut().expect("startup notice").text = "malformed\nnotice".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn malformed_retained_chat_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(tarrowyn_protocol::GuestSessionRequest {
            client_key: Some("core-history-chat-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .chat(
            &session.account_token,
            ChatRequest {
                request_id: "core-history-chat".to_owned(),
                channel: "settlement".to_owned(),
                text: "A retained message.".to_owned(),
            },
        )
        .expect("chat message");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .chat_history
            .back_mut()
            .expect("retained chat")
            .channel = "settlement\nchannel".to_owned();
        assert!(state
            .events
            .iter()
            .any(|record| matches!(record.event, WorldEvent::Chat(_))));
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
