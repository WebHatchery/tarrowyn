use crate::{ServerConfig, WorldRepository};
use std::time::Duration;
use tarrowyn_protocol::{SettlementCondition, SettlementProjection};

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

    for _ in 0..11 {
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
    assert!(settlement(&after_departure, "saltmere").player_activity < 15);
    assert!(repository.account(&session.account_token).is_err());
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
