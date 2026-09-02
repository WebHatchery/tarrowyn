use super::*;
use crate::repository::models::RepositoryState;
use std::time::Duration;

fn crop_config() -> ServerConfig {
    ServerConfig {
        tick_interval: Duration::from_millis(250),
        world_seconds_per_tick: 0.25,
        crop_stage_seconds: 1.0,
        ..ServerConfig::default()
    }
}

fn planted_state(config: &ServerConfig) -> crate::repository::models::StoredState {
    let mut state = RepositoryState::fresh(config);
    state.tick = 10;
    state.plots[0].crop = Some(CropState {
        kind: CropKind::Wheat,
        stage: 0,
        quality: 2,
        planted_tick: 10,
        growth_ticks: 0,
        last_tended_tick: None,
    });
    state.to_stored()
}

#[test]
fn repository_downtime_advances_only_bounded_crop_progress() {
    let config = crop_config();
    let mut stored = planted_state(&config);
    stored.persisted_at_unix_millis = 1_000;

    let restored = RepositoryState::from_stored_at(stored, &config, 2_000);
    let crop = restored.plots[0].crop.expect("restored crop");

    assert_eq!(restored.tick, 10);
    assert_eq!(crop.growth_ticks, 4);
    assert_eq!(crop.stage, 1);
    assert_eq!(crop.quality, 2);
}

#[test]
fn offline_crop_can_mature_without_repetitive_tending() {
    let config = crop_config();
    let mut stored = planted_state(&config);
    stored.persisted_at_unix_millis = 10_000;

    let restored = RepositoryState::from_stored_at(stored, &config, 13_000);
    let crop = restored.plots[0].crop.expect("restored crop");

    assert!(crop.mature());
    assert_eq!(crop.growth_ticks, 12);
    assert_eq!(crop.last_tended_tick, None);
}

#[test]
fn future_or_legacy_timestamps_cannot_invent_unbounded_progress() {
    let config = crop_config();
    let mut future = planted_state(&config);
    future.persisted_at_unix_millis = 5_000;
    let future = RepositoryState::from_stored_at(future, &config, 4_000);
    assert_eq!(future.plots[0].crop.unwrap().growth_ticks, 0);

    let mut legacy = planted_state(&config);
    legacy.storage_version = 21;
    legacy.persisted_at_unix_millis = 0;
    legacy.tick = 18;
    let legacy = RepositoryState::from_stored_at(legacy, &config, u64::MAX);
    assert_eq!(legacy.plots[0].crop.unwrap().growth_ticks, 8);
}

#[test]
fn offline_crop_progress_is_capped_at_seven_real_days() {
    let config = crop_config();
    let mut stored = planted_state(&config);
    stored.persisted_at_unix_millis = 1;

    let restored = RepositoryState::from_stored_at(stored, &config, u64::MAX);

    assert_eq!(restored.plots[0].crop.unwrap().growth_ticks, 2_419_200);
}

#[test]
fn file_backed_repository_reopen_applies_offline_crop_progress() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-offline-crop-{}-{}.json",
        std::process::id(),
        super::super::phase4::unix_time_seconds()
    ));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        backup_path: None,
        ..crop_config()
    };
    let mut stored = planted_state(&config);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    stored.persisted_at_unix_millis = now.saturating_sub(1_000);
    std::fs::write(&path, serde_json::to_vec_pretty(&stored).unwrap()).unwrap();

    let repository = WorldRepository::new(config);
    let state = repository.state.lock().unwrap();
    let crop = state.plots[0].crop.expect("reopened crop");
    assert_eq!(state.tick, 10);
    assert!(crop.growth_ticks >= 4);
    assert!(crop.stage >= 1);
    drop(state);
    drop(repository);
    let _ = std::fs::remove_file(path);
}
