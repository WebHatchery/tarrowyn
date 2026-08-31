use super::*;

#[test]
fn default_server_clock_matches_the_gdd_eighty_minute_day() {
    let config = ServerConfig::default();
    let real_day_seconds = config.day_length_seconds / config.world_seconds_per_tick
        * config.tick_interval.as_secs_f32();

    assert_eq!(real_day_seconds, 80.0 * 60.0);
}

#[test]
fn default_server_world_values_match_the_shared_game_config_manifest() {
    let config = ServerConfig::default();
    let content = crate::content::game_config_defaults();

    assert_eq!(config.world_width, content.world_width);
    assert_eq!(config.world_height, content.world_height);
    assert_eq!(config.day_length_seconds, content.day_length_seconds);
    assert_eq!(config.starting_gold, content.starting_gold);
    assert_eq!(config.starting_seeds, content.starting_seeds);
}

#[test]
fn oversized_unsigned_environment_values_fall_back_instead_of_wrapping() {
    assert_eq!(bounded_u32(u64::MAX, 17), 17);
    assert_eq!(bounded_u16(u64::MAX, 3306), 3306);
    let representable = u64::try_from(usize::MAX).expect("usize fits in u64");
    assert_eq!(bounded_usize(representable, 17), usize::MAX);
    if representable < u64::MAX {
        assert_eq!(bounded_usize(u64::MAX, 17), 17);
    }
}

#[test]
fn http_pool_settings_preserve_auto_mode_and_bounded_limits() {
    assert_eq!(bounded_http_worker_setting(0, 0), 0);
    assert_eq!(bounded_http_worker_setting(1, 0), MIN_HTTP_REQUEST_WORKERS);
    assert_eq!(bounded_http_worker_setting(33, 0), MAX_HTTP_REQUEST_WORKERS);

    assert_eq!(
        bounded_http_queue_capacity(0, DEFAULT_HTTP_REQUEST_QUEUE_CAPACITY),
        MIN_HTTP_REQUEST_QUEUE_CAPACITY
    );
    assert_eq!(
        bounded_http_queue_capacity(99_999, DEFAULT_HTTP_REQUEST_QUEUE_CAPACITY),
        MAX_HTTP_REQUEST_QUEUE_CAPACITY
    );
}
