use super::{guest, repo};
use tarrowyn_protocol::MovementIntent;

#[test]
fn mutation_request_ids_reject_control_characters_before_replay_lookup() {
    let repository = repo();
    let session = guest(&repository, "request-id-validation");
    let error = repository
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "movement\u{7}request".to_owned(),
                dx: 0,
                dy: 1,
            },
        )
        .expect_err("control characters must not enter mutation replay keys");
    assert_eq!(error.status, 400);
    assert_eq!(error.error.code, "invalid_request_id");
}
