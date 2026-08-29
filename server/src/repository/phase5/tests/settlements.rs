use crate::{ServerConfig, WorldRepository};
use std::time::Duration;
use tarrowyn_protocol::{
    ClaimLifecycleAction, ClaimLifecycleRequest, SettlementCondition, SettlementProjection,
};

#[test]
fn settlement_activity_is_local_and_declines_after_the_last_player_leaves() {
    let repository = WorldRepository::new(ServerConfig {
        household_decision_interval_ticks: 1,
        session_ttl_seconds: 0,
        tick_interval: Duration::from_millis(1),
        ..ServerConfig::default()
    });
    let session = super::guest(&repository, "phase5-settlement-activity");
    let baseline = settlement_snapshot(&repository);

    repository.tick();
    let supported = settlement_snapshot(&repository);
    assert!(
        settlement(&supported, "hearth").player_activity
            > settlement(&baseline, "hearth").player_activity
    );
    assert!(
        settlement(&supported, "saltmere").player_activity
            < settlement(&baseline, "saltmere").player_activity
    );

    for _ in 0..9 {
        repository.tick();
    }
    let after_departure = settlement_snapshot(&repository);
    assert!(
        settlement(&after_departure, "hearth").player_activity
            < settlement(&supported, "hearth").player_activity
    );
    assert_eq!(
        settlement(&after_departure, "saltmere").condition,
        SettlementCondition::Strained
    );
    let saltmere = settlement(&after_departure, "saltmere");
    assert!(saltmere.player_activity < 15);
    assert!(saltmere.safety < settlement(&baseline, "saltmere").safety);
    assert!(saltmere.industry < settlement(&baseline, "saltmere").industry);
    assert!(saltmere.governance < settlement(&baseline, "saltmere").governance);
    assert_ne!(
        saltmere.infrastructure,
        settlement(&baseline, "saltmere").infrastructure
    );
    assert!(repository.account(&session.account_token).is_err());
}

#[test]
fn settlement_projection_rolls_up_nearest_claims_plots_and_public_works() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = super::guest(&repository, "phase5-settlement-facilities");

    let initial = repository
        .settlements(&session.account_token)
        .expect("settlements")
        .data
        .settlements;
    assert_eq!(settlement(&initial, "hearth").available_plot_count, 1);
    assert_eq!(settlement(&initial, "saltmere").available_plot_count, 2);
    assert_eq!(
        settlement(&initial, "whisperwood-outpost").available_plot_count,
        0
    );
    assert!(settlement(&initial, "hearth")
        .public_works
        .iter()
        .any(|work| work == "Town hall"));
    assert_eq!(
        settlement(&initial, "whisperwood-outpost").public_works,
        vec!["Whisperwood watchtower"]
    );
    assert_eq!(
        settlement(&initial, "saltmere").public_works,
        vec!["Saltmere quay"]
    );

    let requested = repository
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "facility-claim".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request")
        .data;
    assert!(requested.accepted);
    let after_claim = repository
        .settlements(&session.account_token)
        .expect("settlements after claim")
        .data
        .settlements;
    assert_eq!(
        after_claim
            .iter()
            .map(|settlement| settlement.claim_count)
            .sum::<u32>(),
        1
    );
    assert_eq!(
        after_claim
            .iter()
            .map(|settlement| settlement.available_plot_count)
            .sum::<u32>(),
        2
    );
}

fn settlement_snapshot(repository: &WorldRepository) -> Vec<SettlementProjection> {
    repository
        .state
        .lock()
        .expect("world repository lock poisoned")
        .phase5
        .settlements
        .clone()
}

fn settlement<'a>(
    settlements: &'a [SettlementProjection],
    location_id: &str,
) -> &'a SettlementProjection {
    settlements
        .iter()
        .find(|settlement| settlement.location_id == location_id)
        .expect("settlement should exist")
}
