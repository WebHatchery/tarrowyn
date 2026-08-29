use super::*;
use tarrowyn_protocol::ModerationReportRequest;

#[test]
fn moderation_report_retries_return_the_original_queued_report() {
    let repository = WorldRepository::new(ServerConfig {
        moderation_cooldown_ticks: 2,
        ..ServerConfig::default()
    });
    let session = guest(&repository, "phase6-moderation-replay");
    let request = ModerationReportRequest {
        request_id: "moderation-replay".to_owned(),
        target_account_id: None,
        message_id: None,
        category: "player_report".to_owned(),
        note: "The same report should not be queued twice.".to_owned(),
    };
    let first = repository
        .moderation_report(&session.account_token, request.clone())
        .unwrap()
        .data;
    let retry = repository
        .moderation_report(&session.account_token, request.clone())
        .unwrap()
        .data;
    assert_eq!(retry, first);
    assert_eq!(first.status, "queued");

    let limited = repository
        .moderation_report(
            &session.account_token,
            ModerationReportRequest {
                request_id: "moderation-too-soon".to_owned(),
                ..request
            },
        )
        .unwrap_err();
    assert_eq!(limited.status, 429);
    assert_eq!(limited.error.code, "moderation_rate_limited");
}
