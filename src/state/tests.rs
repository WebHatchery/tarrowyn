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
fn day_rollover_advances_the_clock_and_crops() {
    let config = test_config();
    let mut session = GameSession::new(&config);

    assert!(session.update_clock(&config, 181.0));
    assert_eq!(session.day, 2);
    assert!(session.day_seconds < 2.0);
    assert!(session.last_activity().contains("Day 2"));
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
