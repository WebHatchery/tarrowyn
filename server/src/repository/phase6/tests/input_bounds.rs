use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{GuestSessionRequest, SupportRepairAction, SupportRepairRequest};

#[test]
fn support_account_rejects_unbounded_or_controlled_target_ids() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("support-input-operator".to_owned()),
            reset: false,
        })
        .expect("operator session")
        .data;

    for target_account_id in ["x".repeat(161), "account\nwith-control".to_owned()] {
        let error = repository
            .support_account(&operator.account_token, &target_account_id)
            .expect_err("invalid support target should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, "invalid_account_id");
    }
}

#[test]
fn support_repair_rejects_unbounded_or_controlled_selector_ids() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("support-repair-input-operator".to_owned()),
            reset: false,
        })
        .expect("operator session")
        .data;

    let cases = [
        (Some("x".repeat(161)), None, "invalid_repair_account"),
        (
            None,
            Some("target\nwith-control".to_owned()),
            "invalid_repair_target",
        ),
    ];
    for (account_id, target_id, expected_code) in cases {
        let error = repository
            .support_repair(
                &operator.account_token,
                SupportRepairRequest {
                    request_id: format!("repair-input-{expected_code}"),
                    action: SupportRepairAction::NormalizeInventory,
                    account_id,
                    target_id,
                    note: "Validate support selector input.".to_owned(),
                },
            )
            .expect_err("invalid repair selector should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, expected_code);
    }
}
