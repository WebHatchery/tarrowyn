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
    assert_eq!(data.config.starting_seeds, 6);
    assert_eq!(data.config.server_url, "http://127.0.0.1:8787");
}

#[test]
fn published_connection_prefers_gateway_and_native_override_wins() {
    assert_eq!(
        select_connection_url(
            "http://127.0.0.1:8787",
            " https://example.test/tarrowyn ",
            None,
            true,
        ),
        "https://example.test/tarrowyn"
    );
    assert_eq!(
        select_connection_url(
            "http://127.0.0.1:8787",
            "https://example.test/tarrowyn",
            Some(" https://override.test "),
            true,
        ),
        "https://override.test"
    );
    assert_eq!(
        select_connection_url(
            "http://127.0.0.1:8787",
            "https://example.test/tarrowyn",
            None,
            false,
        ),
        "http://127.0.0.1:8787"
    );
    assert_eq!(
        select_connection_url("http://127.0.0.1:8787", "   ", None, true,),
        "http://127.0.0.1:8787"
    );
}
