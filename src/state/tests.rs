use super::*;
use crate::data::{ActionKind, GameConfig};

fn test_config() -> GameConfig {
    GameConfig {
        game_name: "years_of_tarrowyn".to_owned(),
        display_name: "The Years of Tarrowyn".to_owned(),
        save_slot: "phase_0".to_owned(),
        version: "0.1.0".to_owned(),
        world_width: 18,
        world_height: 11,
        day_length_seconds: 180.0,
        starting_gold: 12,
        starting_seeds: 6,
        starting_skill: 1,
    }
}

fn action(id: &str, kind: ActionKind) -> ActionDef {
    ActionDef {
        id: id.to_owned(),
        name: id.to_owned(),
        description: String::new(),
        kind,
    }
}

#[test]
fn water_blocks_movement_but_the_path_is_open() {
    let mut session = GameSession::new(&test_config());

    assert!(session.move_player(-1, 0));
    assert_eq!(session.player.position, TilePos::new(7, 6));

    session.player.position = TilePos::new(15, 6);
    assert!(!session.move_player(1, 0));
}

#[test]
fn offline_fixture_uses_the_configured_starting_seed_count() {
    let mut config = test_config();
    config.starting_seeds = 9;

    let session = GameSession::new(&config);

    assert_eq!(session.player.inventory.seeds, 9);
}

#[test]
fn tending_then_harvesting_changes_local_progression() {
    let mut session = GameSession::new(&test_config());
    session.player.position = TilePos::new(4, 5);

    let tend = session.apply_action(&action("tend", ActionKind::Tend));
    assert!(tend.success);
    let harvest = session.apply_action(&action("harvest", ActionKind::Harvest));
    assert!(harvest.success);
    assert_eq!(session.player.inventory.turnips, 1);
    assert_eq!(session.player.gold, 14);
}

#[test]
fn empty_seed_pouch_points_to_the_shared_road_market() {
    let config = test_config();
    let mut session = GameSession::new(&config);
    session.player.inventory.seeds = 0;

    let result = session.apply_action(&action("plant", ActionKind::Plant));

    assert!(!result.success);
    assert!(result.message.contains("shared-road market"));
}

#[test]
fn day_rollover_advances_the_clock_and_crops() {
    let config = test_config();
    let mut session = GameSession::new(&config);

    assert!(session.update_clock(&config, 181.0));
    assert_eq!(session.day, 2);
    assert!(session.day_seconds < 2.0);
    assert!(session.last_activity().contains("Day 2"));
}

#[test]
fn huge_offline_clock_delta_catches_up_without_a_day_loop() {
    let config = test_config();
    let mut session = GameSession::new(&config);

    assert!(session.update_clock(&config, f32::MAX));
    assert_eq!(session.day, u32::MAX);
    assert!(session.day_seconds < config.day_length_seconds);
    assert_eq!(session.crops_ready(), 3);
}

#[test]
fn offline_fixture_uses_the_same_day_period_boundaries() {
    let config = test_config();
    let mut session = GameSession::new(&config);
    session.day_seconds = 45.0;

    assert_eq!(
        session.time_of_day(&config),
        tarrowyn_protocol::TimeOfDay::Afternoon
    );
    assert!(!session.is_night(&config));
}

#[test]
fn current_saves_migrate_to_the_configured_version() {
    let config = test_config();
    let mut save = GameSession::new(&config).to_save("0.0.1");
    save.day = 0;
    save.day_seconds = 999.0;
    let value = serde_json::to_value(save).unwrap();

    let migrated = migrate_save_value(Some("0.0.1".to_owned()), value, &config).unwrap();

    assert_eq!(migrated.version, "0.1.0");
    assert_eq!(migrated.day, 1);
    assert_eq!(migrated.day_seconds, 180.0);
}

#[test]
fn offline_progression_counters_stay_at_the_numeric_ceiling() {
    let config = test_config();
    let mut session = GameSession::new(&config);
    session.player.position = TilePos::new(4, 5);
    session.player.actions_completed = u32::MAX;
    session.player.seeds_planted = u32::MAX;
    session.player.skill = u32::MAX;
    session.player.gold = u32::MAX;
    session.player.reputation = u32::MAX;
    session.player.inventory.wheat = u32::MAX;
    session.player.inventory.turnips = u32::MAX;
    session.player.inventory.seeds = 1;
    session.day = u32::MAX;
    session.day_seconds = config.day_length_seconds;

    assert!(session.update_clock(&config, 0.0));
    assert_eq!(session.day, u32::MAX);

    assert!(
        session
            .apply_action(&action("plant", ActionKind::Plant))
            .success
    );
    assert_eq!(session.player.seeds_planted, u32::MAX);
    assert_eq!(session.player.actions_completed, u32::MAX);

    assert!(
        session
            .apply_action(&action("tend", ActionKind::Tend))
            .success
    );
    assert_eq!(session.player.skill, u32::MAX);

    assert!(
        session
            .apply_action(&action("harvest", ActionKind::Harvest))
            .success
    );
    assert_eq!(session.player.inventory.wheat, u32::MAX);
    assert_eq!(session.player.inventory.total_crops(), u32::MAX);
    assert_eq!(session.player.gold, u32::MAX);
    assert_eq!(session.player.skill, u32::MAX);

    session.player.position = TAVERN_TILE;
    assert!(
        session
            .apply_action(&action("listen", ActionKind::Listen))
            .success
    );
    assert_eq!(session.player.reputation, u32::MAX);
}
