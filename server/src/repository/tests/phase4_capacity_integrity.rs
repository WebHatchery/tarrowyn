use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{Capability, GuestSessionRequest, ProfessionKind, ProfessionProfile};

fn guest(repository: &WorldRepository, client_key: &str) {
    repository
        .guest_session(GuestSessionRequest {
            client_key: Some(client_key.to_owned()),
            reset: false,
        })
        .expect("guest session");
}

fn capability(index: usize) -> Capability {
    Capability {
        capability_id: format!("phase4-capability-{index}"),
        name: "A settlement capability".to_owned(),
        profession: ProfessionKind::Farmer,
        level: 1,
        description: "A bounded capability fixture".to_owned(),
        effect: "It keeps the registry honest".to_owned(),
    }
}

fn profile(capabilities: Vec<Capability>) -> ProfessionProfile {
    ProfessionProfile {
        profession: ProfessionKind::Farmer,
        level: 1,
        reputation: 0,
        credential: Some("farmer credential".to_owned()),
        capabilities,
    }
}

#[test]
fn over_capacity_phase4_identity_keyed_records_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    guest(&repository, "phase4-profile-capacity");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.profiles.insert(
            "phase4-profile-capacity".to_owned(),
            vec![profile(vec![capability(0)]); 7],
        );
    }
    assert!(!repository.ops_health().data.integrity_ok);

    let repository = WorldRepository::new(ServerConfig::default());
    guest(&repository, "phase4-capability-capacity");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.profiles.insert(
            "phase4-capability-capacity".to_owned(),
            vec![profile((0..17).map(capability).collect())],
        );
    }
    assert!(!repository.ops_health().data.integrity_ok);

    let repository = WorldRepository::new(ServerConfig::default());
    guest(&repository, "phase4-credential-capacity");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.credentials.insert(
            "phase4-credential-capacity".to_owned(),
            (0..17)
                .map(|index| format!("phase4-credential-{index}"))
                .collect(),
        );
    }
    assert!(!repository.ops_health().data.integrity_ok);
}
