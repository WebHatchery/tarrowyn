use super::*;

#[test]
fn default_server_clock_matches_the_gdd_eighty_minute_day() {
    let config = ServerConfig::default();
    let real_day_seconds = config.day_length_seconds / config.world_seconds_per_tick
        * config.tick_interval.as_secs_f32();

    assert_eq!(real_day_seconds, 80.0 * 60.0);
}
