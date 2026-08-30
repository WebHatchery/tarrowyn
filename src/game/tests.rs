use super::{actions::apply_chronicle_key, input::keyboard_gameplay_blocked, online_crafting_view};
use crate::network::{ConnectionState, CraftingView};

#[test]
fn chronicle_touch_keys_build_and_edit_a_query() {
    let mut query = String::new();
    apply_chronicle_key(&mut query, "S");
    apply_chronicle_key(&mut query, "T");
    apply_chronicle_key(&mut query, "space");
    apply_chronicle_key(&mut query, "R");
    apply_chronicle_key(&mut query, "delete");

    assert_eq!(query, "ST ");

    apply_chronicle_key(&mut query, "clear");
    assert!(query.is_empty());
}

#[test]
fn gameplay_keyboard_input_stops_behind_non_textual_modals() {
    assert!(!keyboard_gameplay_blocked(false, false, false));
    assert!(keyboard_gameplay_blocked(true, false, false));
    assert!(keyboard_gameplay_blocked(false, true, false));
    assert!(keyboard_gameplay_blocked(false, false, true));
}

#[test]
fn crafting_overlay_waits_for_the_shared_road_before_hiding_reconnect() {
    let crafting = Some(CraftingView {
        progress: 0.5,
        target_start: 0.4,
        target_end: 0.6,
    });

    assert!(online_crafting_view(ConnectionState::Online, crafting).is_some());
    assert!(online_crafting_view(ConnectionState::Connecting, crafting).is_none());
    assert!(online_crafting_view(ConnectionState::Degraded, crafting).is_none());
    assert!(online_crafting_view(ConnectionState::Offline, crafting).is_none());
}
