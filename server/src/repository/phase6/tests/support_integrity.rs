use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    ClaimLifecycleAction, ClaimLifecycleRequest, GuestSessionRequest, SupportRepairAction,
    SupportRepairRequest,
};

#[test]
fn support_claim_repair_removes_a_stale_free_plot_entry() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("support-claim-repair".to_owned()),
            reset: false,
        })
        .expect("operator session")
        .data;
    let claim = repository
        .claim_lifecycle(
            &operator.account_token,
            ClaimLifecycleRequest {
                request_id: "support-claim-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request")
        .data
        .claim
        .expect("claim record");
    repository
        .claim_lifecycle(
            &operator.account_token,
            ClaimLifecycleRequest {
                request_id: "support-claim-approve".to_owned(),
                action: ClaimLifecycleAction::Approve,
                claim_id: Some(claim.claim_id.clone()),
                target_account_id: None,
            },
        )
        .expect("claim approval");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .claims
            .iter_mut()
            .find(|record| record.claim_id == claim.claim_id)
            .expect("active claim")
            .building_access = false;
        state.phase4.available_plots.push(claim.position);
    }

    let repaired = repository
        .support_repair(
            &operator.account_token,
            SupportRepairRequest {
                request_id: "support-claim-repair-request".to_owned(),
                action: SupportRepairAction::RestoreClaim,
                account_id: Some(operator.account_id),
                target_id: Some(claim.claim_id.clone()),
                note: "Restore access and remove the stale free-plot entry.".to_owned(),
            },
        )
        .expect("claim repair")
        .data;

    assert!(repaired.accepted);
    let state = repository.state.lock().expect("repository lock");
    assert!(
        state
            .phase4
            .claims
            .iter()
            .find(|record| record.claim_id == claim.claim_id)
            .expect("repaired claim")
            .building_access
    );
    assert!(!state.phase4.available_plots.contains(&claim.position));
    drop(state);
    assert!(repository.ops_health().data.integrity_ok);
}

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
