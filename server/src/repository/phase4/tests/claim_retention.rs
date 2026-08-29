use super::super::super::{ServerConfig, WorldRepository};
use super::guest;
use tarrowyn_protocol::{
    ClaimLifecycleAction, ClaimLifecycleRequest, ClaimLifecycleStatus, ClaimRecord, Position,
};

fn reclaimed_claim(index: usize) -> ClaimRecord {
    ClaimRecord {
        claim_id: format!("reclaimed-claim-{index}"),
        plot_id: format!("plot-{index}"),
        owner_account_id: None,
        owner_name: None,
        position: Position {
            x: (index % 18) as i32,
            y: (index / 18) as i32,
        },
        lease_days: 90,
        started_tick: index as u64,
        expires_tick: index as u64,
        started_at_unix_seconds: 0,
        expires_at_unix_seconds: 0,
        last_active_tick: index as u64,
        status: ClaimLifecycleStatus::Reclaimed,
        approved_by: None,
        building_access: false,
        protected_goods_policy: "Stored goods remain protected.".to_owned(),
        inspection_note: "This history row is no longer active.".to_owned(),
    }
}

#[test]
fn claim_history_evicts_reclaimed_rows_before_requesting_new_land() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-claim-retention");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.claims = (0..super::super::MAX_CLAIMS).map(reclaimed_claim).collect();
        state.phase4.available_plots.push(Position { x: 17, y: 10 });
    }

    let created = repository
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "claim-after-history".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request")
        .data;
    assert!(created.accepted);
    let state = repository.state.lock().expect("repository lock");
    assert_eq!(state.phase4.claims.len(), super::super::MAX_CLAIMS);
    assert!(!state
        .phase4
        .claims
        .iter()
        .any(|claim| claim.claim_id == "reclaimed-claim-0"));
    assert!(state
        .phase4
        .claims
        .iter()
        .any(|claim| claim.claim_id == "lease-1"));
    drop(state);

    {
        let mut state = repository.state.lock().expect("repository lock");
        for claim in &mut state.phase4.claims {
            claim.status = ClaimLifecycleStatus::Active;
        }
        state.phase4.available_plots.push(Position { x: 16, y: 10 });
    }
    let blocked = repository
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "claim-with-live-ledger".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("full live claim request")
        .data;
    assert!(!blocked.accepted);
    assert!(blocked
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("claim ledger is full")));
    let state = repository.state.lock().expect("repository lock");
    assert_eq!(state.phase4.claims.len(), super::super::MAX_CLAIMS);
    assert_eq!(
        state.phase4.available_plots.last(),
        Some(&Position { x: 16, y: 10 })
    );
}

#[test]
fn claim_id_stays_at_the_numeric_ceiling() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "claim-id-ceiling");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.next_claim_id = u64::MAX;
        state.phase4.available_plots.push(Position { x: 17, y: 10 });
    }

    let response = repository
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "claim-id-ceiling-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request")
        .data;

    let claim = response.claim.expect("accepted claim");
    assert_eq!(claim.claim_id, format!("lease-{}", u64::MAX));
    assert_eq!(claim.plot_id, format!("plot-{}", u64::MAX));
    let state = repository.state.lock().expect("repository lock");
    assert_eq!(state.phase4.next_claim_id, u64::MAX);
}
