use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{GuestSessionRequest, RegionalEventAction, RegionalEventRequest};

fn seeded_event_cache(repository: &WorldRepository) -> (String, String) {
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase5-replay-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "phase5-replay-request".to_owned(),
                action: RegionalEventAction::Seed,
                event_id: None,
                intervention: None,
            },
        )
        .expect("regional event seed");
    (session.client_key, "phase5-replay-request".to_owned())
}

#[test]
fn orphaned_phase5_replay_key_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let (identity_key, request_id) = seeded_event_cache(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let response = state
            .phase5
            .request_results
            .remove(&format!("phase5:{identity_key}:{request_id}"))
            .expect("seeded replay result");
        state
            .phase5
            .request_results
            .insert(format!("phase5:missing-identity:{request_id}"), response);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn phase5_replay_response_must_match_its_request_key() {
    let repository = WorldRepository::new(ServerConfig::default());
    let (identity_key, request_id) = seeded_event_cache(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let key = format!("phase5:{identity_key}:{request_id}");
        let response = state
            .phase5
            .request_results
            .get_mut(&key)
            .expect("seeded replay result");
        if let super::super::phase5::Phase5Response::Event(response) = response {
            response.request_id = "different-request".to_owned();
        }
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
