#[test]
fn authoritative_manifests_satisfy_the_content_contract() {
    super::validate().expect("checked-in content should satisfy the server schema");
}

#[test]
fn content_ids_must_be_unique_and_non_empty() {
    assert!(super::validate_id_list("test", vec!["one", "two"]).is_ok());
    assert!(super::validate_id_list("test", vec!["one", "one"]).is_err());
    assert!(super::validate_id_list("test", vec!["one", ""]).is_err());
}
