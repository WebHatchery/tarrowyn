use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::GuestSessionRequest;

fn seeded_profession(repository: &WorldRepository) -> String {
    let player = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-profession-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .professions(&player.account_token)
        .expect("profession view");
    player.client_key
}

#[test]
fn malformed_phase4_credential_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let identity_key = seeded_profession(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .credentials
            .get_mut(&identity_key)
            .expect("credentials")
            .push("credential\nwith-control".to_owned());
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn excessive_phase4_profession_reputation_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let identity_key = seeded_profession(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .profiles
            .get_mut(&identity_key)
            .expect("profiles")
            .first_mut()
            .expect("profile")
            .reputation = 101;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
