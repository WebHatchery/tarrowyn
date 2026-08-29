use super::{guest, repo};
use tarrowyn_protocol::{MovementIntent, Position};

#[test]
fn movement_rejects_extreme_deltas_without_overflow() {
    let repository = repo();
    let session = guest(&repository, "movement-overflow");
    let response = repository
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "movement-overflow-request".to_owned(),
                dx: i32::MIN,
                dy: i32::MAX,
            },
        )
        .expect("extreme movement should return a response")
        .data;
    assert!(!response.accepted);
    assert_eq!(response.position, Position { x: 8, y: 6 });
    assert!(response
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("cardinal step")));
}
