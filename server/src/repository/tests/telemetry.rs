use super::super::ServerConfig;
use super::super::WorldRepository;
use std::time::Duration;

#[test]
fn tick_telemetry_tracks_average_and_drift_without_using_configured_interval_as_a_result() {
    let repository = WorldRepository::new(ServerConfig {
        tick_interval: Duration::from_millis(10),
        backup_path: None,
        ..ServerConfig::default()
    });

    repository.record_tick_duration(Duration::from_millis(4));
    repository.record_tick_duration(Duration::from_millis(12));

    let telemetry = repository.tick_telemetry.lock().unwrap();
    assert_eq!(telemetry.last_tick_ms, 12);
    assert_eq!(telemetry.average_tick_ms, 5);
    assert_eq!(telemetry.tick_drift_count, 1);
    assert!(telemetry.last_tick_drift);
}
