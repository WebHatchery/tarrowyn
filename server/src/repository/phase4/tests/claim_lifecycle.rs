use super::super::super::{ServerConfig, WorldRepository};
use super::guest;
use tarrowyn_protocol::{ClaimLifecycleAction, ClaimLifecycleRequest, ClaimLifecycleStatus};

#[test]
fn expired_claim_cannot_be_reassigned_or_extend_reclamation_grace() {
    let repository = WorldRepository::new(ServerConfig {
        claim_reclaim_grace_ticks: 10,
        ..ServerConfig::default()
    });
    let owner = guest(&repository, "expired-claim-owner");
    let recipient = guest(&repository, "expired-claim-recipient");
    let requested = repository
        .claim_lifecycle(
            &owner.account_token,
            ClaimLifecycleRequest {
                request_id: "expired-claim-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request")
        .data
        .claim
        .expect("requested claim");
    let claim_id = requested.claim_id.clone();
    repository
        .claim_lifecycle(
            &owner.account_token,
            ClaimLifecycleRequest {
                request_id: "expired-claim-approve".to_owned(),
                action: ClaimLifecycleAction::Approve,
                claim_id: Some(claim_id.clone()),
                target_account_id: None,
            },
        )
        .expect("claim approval");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let claim = state
            .phase4
            .claims
            .iter_mut()
            .find(|claim| claim.claim_id == claim_id)
            .expect("claim remains recorded");
        claim.status = ClaimLifecycleStatus::Expired;
        claim.building_access = false;
        claim.expires_at_unix_seconds = 1;
        claim.last_active_tick = 7;
    }

    let transfer = repository
        .claim_lifecycle(
            &owner.account_token,
            ClaimLifecycleRequest {
                request_id: "expired-claim-transfer".to_owned(),
                action: ClaimLifecycleAction::Transfer,
                claim_id: Some(claim_id.clone()),
                target_account_id: Some(recipient.account_id),
            },
        )
        .expect("expired transfer response")
        .data;
    assert!(!transfer.accepted);
    assert!(transfer
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("active lease")));

    let abandon = repository
        .claim_lifecycle(
            &owner.account_token,
            ClaimLifecycleRequest {
                request_id: "expired-claim-abandon".to_owned(),
                action: ClaimLifecycleAction::Abandon,
                claim_id: Some(claim_id.clone()),
                target_account_id: None,
            },
        )
        .expect("expired abandon response")
        .data;
    assert!(!abandon.accepted);
    assert!(abandon
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("requested or active")));

    let state = repository.state.lock().expect("repository lock");
    let claim = state
        .phase4
        .claims
        .iter()
        .find(|claim| claim.claim_id == claim_id)
        .expect("claim remains recorded");
    assert_eq!(claim.status, ClaimLifecycleStatus::Expired);
    assert_eq!(claim.last_active_tick, 7);
    assert!(!claim.building_access);
}
