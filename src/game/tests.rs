use super::{actions::apply_chronicle_key, input::keyboard_gameplay_blocked};

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
