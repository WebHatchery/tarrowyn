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
fn moderation_reports_reject_missing_or_mismatched_chat_evidence() {
    let repository = WorldRepository::new(ServerConfig::default());
    let reporter = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("moderation-evidence-reporter".to_owned()),
            reset: false,
        })
        .expect("reporter session")
        .data;
    let other_player = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("moderation-evidence-other".to_owned()),
            reset: false,
        })
        .expect("other player session")
        .data;
    let message_id = repository
        .chat(
            &other_player.account_token,
            ChatRequest {
                request_id: "moderation-evidence-chat".to_owned(),
                channel: "settlement".to_owned(),
                text: "Evidence remains in the chat ledger.".to_owned(),
            },
        )
        .expect("chat evidence")
        .data
        .message
        .expect("accepted chat evidence")
        .message_id;
    let error = repository
        .moderation_report(
            &reporter.account_token,
            ModerationReportRequest {
                request_id: "missing-evidence".to_owned(),
                target_account_id: Some("account-2".to_owned()),
                message_id: Some(999),
                category: "player_report".to_owned(),
                note: "The evidence should be retained by the chat ledger.".to_owned(),
            },
        )
        .expect_err("missing chat evidence should be rejected");

    assert_eq!(error.error.code, "invalid_report_evidence");

    let error = repository
        .moderation_report(
            &reporter.account_token,
            ModerationReportRequest {
                request_id: "mismatched-evidence".to_owned(),
                target_account_id: Some(reporter.account_id),
                message_id: Some(message_id),
                category: "player_report".to_owned(),
                note: "The selected message belongs to another account.".to_owned(),
            },
        )
        .expect_err("mismatched chat evidence should be rejected");

    assert_eq!(error.error.code, "invalid_report_evidence");

    let accepted = repository
        .moderation_report(
            &reporter.account_token,
            ModerationReportRequest {
                request_id: "valid-evidence".to_owned(),
                target_account_id: Some(other_player.account_id.clone()),
                message_id: Some(message_id),
                category: "harassment".to_owned(),
                note: "The audit keeps the evidence reference, not chat text.".to_owned(),
            },
        )
        .expect("valid chat evidence should be accepted")
        .data;
    assert!(accepted.accepted);
    let state = repository.state.lock().expect("repository state lock");
    let audit = state.phase6.audits.back().expect("report audit");
    assert_eq!(audit.action, "moderation.report:harassment");
    assert_eq!(
        audit.target,
        format!("{} (message {message_id})", other_player.account_id)
    );
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
