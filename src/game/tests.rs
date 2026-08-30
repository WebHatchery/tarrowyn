use super::actions::apply_chronicle_key;

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
