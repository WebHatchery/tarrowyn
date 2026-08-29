use super::*;
use tarrowyn_protocol::{ModerationReportRequest, SupportRepairAction, SupportRepairRequest};

#[test]
fn moderation_reports_reject_unbounded_or_controlled_persisted_inputs() {
    let fixtures = [
        (
            "moderation-long-category",
            None,
            "x".repeat(41),
            "A useful note".to_owned(),
            "invalid_report",
        ),
        (
            "moderation-control-note",
            None,
            "harassment".to_owned(),
            "A note\nwith a boundary".to_owned(),
            "invalid_report",
        ),
        (
            "moderation-control-target",
            Some("target\naccount".to_owned()),
            "harassment".to_owned(),
            "A useful note".to_owned(),
            "invalid_report_target",
        ),
    ];
    for (request_id, target_account_id, category, note, expected_code) in fixtures {
        let repository = WorldRepository::new(ServerConfig::default());
        let session = repository
            .guest_session(GuestSessionRequest {
                client_key: Some(request_id.to_owned()),
                reset: false,
            })
            .expect("guest session")
            .data;
        let error = repository
            .moderation_report(
                &session.account_token,
                ModerationReportRequest {
                    request_id: request_id.to_owned(),
                    target_account_id,
                    message_id: None,
                    category,
                    note,
                },
            )
            .expect_err("invalid moderation input should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, expected_code);
    }
}

#[test]
fn support_repairs_reject_unbounded_or_controlled_operator_notes() {
    for (request_id, note) in [
        ("repair-long-note", "x".repeat(241)),
        (
            "repair-control-note",
            "Repair note\twith a boundary".to_owned(),
        ),
    ] {
        let repository = WorldRepository::new(ServerConfig {
            support_operator_accounts: vec!["dev-account-1".to_owned()],
            ..ServerConfig::default()
        });
        let operator = repository
            .guest_session(GuestSessionRequest {
                client_key: Some(request_id.to_owned()),
                reset: false,
            })
            .expect("operator session")
            .data;
        let error = repository
            .support_repair(
                &operator.account_token,
                SupportRepairRequest {
                    request_id: request_id.to_owned(),
                    action: SupportRepairAction::NormalizeInventory,
                    account_id: None,
                    target_id: None,
                    note,
                },
            )
            .expect_err("invalid operator note should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, "invalid_repair_note");
    }
}
