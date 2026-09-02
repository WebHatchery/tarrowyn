use super::*;
use tarrowyn_protocol::FoundationInteractionRequest;

#[test]
fn builder_and_noticeboard_are_authoritative_proximity_interactions() {
    let repository = repo();
    let session = guest(&repository, "foundation-context");
    repository
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "step-to-camp".to_owned(),
                dx: 0,
                dy: -1,
            },
        )
        .expect("movement beside builder and board");

    let builder = repository
        .foundation_interaction(
            &session.account_token,
            FoundationInteractionRequest {
                request_id: "meet-mara".to_owned(),
                interaction_id: "speak-with-builder".to_owned(),
            },
        )
        .expect("builder interaction");
    assert!(builder.data.accepted);
    assert_eq!(builder.data.landmark_id, "builder-mara");
    assert!(builder.data.message.contains("noticeboard"));

    let board = repository
        .foundation_interaction(
            &session.account_token,
            FoundationInteractionRequest {
                request_id: "read-needs".to_owned(),
                interaction_id: "read-local-needs".to_owned(),
            },
        )
        .expect("noticeboard interaction");
    assert!(board.data.accepted);
    assert!(board.data.message.contains("timber"));
    assert!(board.data.message.contains("stone"));
}

#[test]
fn interaction_is_rejected_when_player_is_not_near_the_landmark() {
    let repository = repo();
    let session = guest(&repository, "foundation-distance");
    repository
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "walk-south".to_owned(),
                dx: 0,
                dy: 1,
            },
        )
        .expect("movement");

    let response = repository
        .foundation_interaction(
            &session.account_token,
            FoundationInteractionRequest {
                request_id: "too-far".to_owned(),
                interaction_id: "read-local-needs".to_owned(),
            },
        )
        .expect("bounded rejection");

    assert!(!response.data.accepted);
    assert!(response.data.message.contains("Walk beside"));
}

#[test]
fn later_foundation_work_is_visible_but_not_enabled_early() {
    let repository = repo();
    let session = guest(&repository, "foundation-deferral");
    let response = repository
        .foundation_interaction(
            &session.account_token,
            FoundationInteractionRequest {
                request_id: "cache-before-f2".to_owned(),
                interaction_id: "use-shared-cache".to_owned(),
            },
        )
        .expect("explicit deferral");

    assert!(!response.data.accepted);
    assert!(response
        .data
        .message
        .contains("later foundational milestone"));
}
