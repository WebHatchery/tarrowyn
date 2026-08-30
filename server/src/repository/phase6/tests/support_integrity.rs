use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{GuestSessionRequest, SupportRepairAction, SupportRepairRequest};

#[test]
fn malformed_support_replay_key_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("support-replay-key-integrity".to_owned()),
            reset: false,
        })
        .expect("operator session")
        .data;
    repository
        .support_repair(
            &operator.account_token,
            SupportRepairRequest {
                request_id: "support-replay-key".to_owned(),
                action: SupportRepairAction::NormalizeInventory,
                account_id: None,
                target_id: None,
                note: "The support note records why this repair is safe.".to_owned(),
            },
        )
        .expect("support repair");

    {
        let mut state = repository.state.lock().expect("repository lock");
        let key = state
            .phase6
            .request_results
            .keys()
            .next()
            .cloned()
            .expect("support cache");
        let response = state
            .phase6
            .request_results
            .remove(&key)
            .expect("support cache response");
        state.phase6.request_results.insert(
            "repair:missing-account:support-replay-key".to_owned(),
            response,
        );
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn support_replay_cleanup_requires_complete_account_and_request_boundaries() {
    let response = tarrowyn_protocol::SupportRepairResponse {
        request_id: "support-boundary-request".to_owned(),
        audit_id: "audit-1".to_owned(),
        accepted: true,
        summary: "The bounded repair completed.".to_owned(),
        reason: None,
    };

    assert!(super::super::is_support_replay_key_for_account(
        "repair:account-owner:support-boundary-request",
        "account-owner",
        &response,
    ));
    assert!(!super::super::is_support_replay_key_for_account(
        "repair:account-owner:observer:support-boundary-request",
        "account-owner",
        &response,
    ));
    assert!(super::super::is_support_replay_key_for_account(
        "repair:account-owner:observer:support-boundary-request",
        "account-owner:observer",
        &response,
    ));
}
