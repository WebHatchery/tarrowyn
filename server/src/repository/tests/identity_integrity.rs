use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::GuestSessionRequest;

#[test]
fn malformed_identity_name_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("identity-name-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .identities
            .get_mut(&session.client_key)
            .expect("identity")
            .display_name = "invalid\nname".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn malformed_identity_identifier_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("identity-id-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .identities
            .get_mut(&session.client_key)
            .expect("identity")
            .account_id = "invalid\naccount".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
