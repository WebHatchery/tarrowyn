use super::*;

#[test]
fn foundation_baseline_is_deterministic_and_fresh_world_has_no_leak() {
    let first_repository = repo();
    let first_guest = guest(&first_repository, "foundation-first-run");
    let first_state = first_repository
        .state(&first_guest.account_token)
        .expect("first authoritative state")
        .data;
    first_repository
        .chat(
            &first_guest.account_token,
            ChatRequest {
                request_id: "foundation-leak-probe".to_owned(),
                channel: "settlement".to_owned(),
                text: "This belongs only to the first run.".to_owned(),
            },
        )
        .expect("first run mutation");

    let reset_repository = repo();
    let reset_guest = guest(&reset_repository, "foundation-second-run");
    let reset_state = reset_repository
        .state(&reset_guest.account_token)
        .expect("reset authoritative state")
        .data;

    assert_eq!(first_state.world.foundation, reset_state.world.foundation);
    assert_eq!(
        reset_state.world.foundation.fixture_id,
        "first-beacon-baseline-v1"
    );
    assert_eq!(reset_state.world.foundation.landmarks.len(), 12);
    assert_eq!(reset_state.world.foundation.interactions.len(), 12);
    assert!(reset_state
        .feed
        .chat
        .iter()
        .all(|message| message.text != "This belongs only to the first run."));
    assert_eq!(reset_state.world.players.len(), 1);
}

#[test]
fn foundation_baseline_survives_a_development_identity_reset() {
    let repository = repo();
    let original = guest(&repository, "foundation-reset");
    let before = repository
        .world(&original.account_token)
        .expect("world before reset")
        .data
        .foundation;
    let replacement = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("foundation-reset".to_owned()),
            reset: true,
        })
        .expect("reset guest")
        .data;
    let after = repository
        .world(&replacement.account_token)
        .expect("world after reset")
        .data
        .foundation;

    assert_ne!(original.character_id, replacement.character_id);
    assert_eq!(before, after);
}
