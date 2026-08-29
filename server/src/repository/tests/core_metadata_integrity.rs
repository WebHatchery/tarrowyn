use super::super::{ServerConfig, WorldRepository};

#[test]
fn invalid_core_sequence_metadata_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.next_notice = 0;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
