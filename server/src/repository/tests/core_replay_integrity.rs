use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{ChatRequest, GuestSessionRequest};

#[test]
fn core_replay_response_must_match_its_request_key() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("core-replay-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .chat(
            &session.account_token,
            ChatRequest {
                request_id: "core-replay-request".to_owned(),
                channel: "settlement".to_owned(),
                text: "A replay boundary check.".to_owned(),
            },
        )
        .expect("chat message");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let response = state
            .identities
            .get_mut(&session.client_key)
            .expect("identity")
            .chat_results
            .get_mut("core-replay-request")
            .expect("chat replay result");
        response.request_id = "different-request".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
