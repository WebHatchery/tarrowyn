use super::*;

#[test]
fn phase_zero_data_loads() {
    let data = GameData::load().unwrap();

    assert_eq!(data.config.game_name, "years_of_tarrowyn");
    assert_eq!(data.actions.len(), 4);
    assert!(data.actions.contains("listen"));
    assert_eq!(data.crops.len(), 3);
    assert_eq!(data.config.world_width, 18);
    assert_eq!(data.config.world_height, 11);
}
