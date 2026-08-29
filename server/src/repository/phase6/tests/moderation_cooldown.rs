use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::GuestSessionRequest;

#[test]
fn moderation_cooldowns_follow_identity_lifetime_not_replay_eviction() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("moderation-cooldown-player".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let mut state = repository.state.lock().expect("repository lock");
    state
        .phase6
        .moderation_last_report_ticks
        .insert(session.client_key.clone(), 12);
    state
        .phase6
        .moderation_last_report_ticks
        .insert("deleted-identity".to_owned(), 13);

    super::super::prune_moderation_cooldowns(&mut state);

    assert_eq!(
        state
            .phase6
            .moderation_last_report_ticks
            .get(&session.client_key),
        Some(&12)
    );
    assert!(!state
        .phase6
        .moderation_last_report_ticks
        .contains_key("deleted-identity"));
}
