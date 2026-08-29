use super::*;

#[test]
fn default_server_clock_matches_the_gdd_eighty_minute_day() {
    let config = ServerConfig::default();
    let real_day_seconds = config.day_length_seconds / config.world_seconds_per_tick
        * config.tick_interval.as_secs_f32();

    assert_eq!(real_day_seconds, 80.0 * 60.0);
}

#[test]
fn oversized_unsigned_environment_values_fall_back_instead_of_wrapping() {
    assert_eq!(bounded_u32(u64::MAX, 17), 17);
    assert_eq!(bounded_u16(u64::MAX, 3306), 3306);
}
