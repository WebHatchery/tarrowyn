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

#[test]
fn reclaim_waits_for_the_configured_grace_period() {
    let repository = WorldRepository::new(ServerConfig {
        claim_reclaim_grace_ticks: 10,
        ..ServerConfig::default()
    });
    let owner = guest(&repository, "reclaim-grace-owner");
    let late_player = guest(&repository, "reclaim-grace-late-player");
    let claim = repository
        .claim_lifecycle(
            &owner.account_token,
            ClaimLifecycleRequest {
                request_id: "reclaim-grace-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request")
        .data
        .claim
        .expect("requested claim");
    repository
        .claim_lifecycle(
            &owner.account_token,
            ClaimLifecycleRequest {
                request_id: "reclaim-grace-approve".to_owned(),
                action: ClaimLifecycleAction::Approve,
                claim_id: Some(claim.claim_id.clone()),
                target_account_id: None,
            },
        )
        .expect("claim approval");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let tick = state.tick;
        let claim = state
            .phase4
            .claims
            .iter_mut()
            .find(|candidate| candidate.claim_id == claim.claim_id)
            .expect("claim remains recorded");
        claim.status = ClaimLifecycleStatus::Abandoned;
        claim.building_access = false;
        claim.last_active_tick = tick;
    }

    let response = repository
        .claim_lifecycle(
            &late_player.account_token,
            ClaimLifecycleRequest {
                request_id: "reclaim-grace-too-soon".to_owned(),
                action: ClaimLifecycleAction::Reclaim,
                claim_id: Some(claim.claim_id.clone()),
                target_account_id: None,
            },
        )
        .expect("reclaim response")
        .data;
    assert!(!response.accepted);
    assert!(response
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("grace period")));

    let state = repository.state.lock().expect("repository lock");
    let claim = state
        .phase4
        .claims
        .iter()
        .find(|candidate| candidate.claim_id == claim.claim_id)
        .expect("claim remains recorded");
    assert_eq!(claim.status, ClaimLifecycleStatus::Abandoned);
    assert!(!state.phase4.available_plots.contains(&claim.position));
}

#[test]
fn unknown_claim_inspection_does_not_fall_back_to_the_latest_record() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "unknown-claim-selector");
    repository
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "known-claim-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request");

    let response = repository
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "unknown-claim-inspect".to_owned(),
                action: ClaimLifecycleAction::Inspect,
                claim_id: Some("missing-claim".to_owned()),
                target_account_id: None,
            },
        )
        .expect("unknown claim inspection")
        .data;

    assert!(!response.accepted);
    assert_eq!(response.claim, None);
    assert_eq!(
        response.reason.as_deref(),
        Some("That claim is not in the land registry.")
    );
}
