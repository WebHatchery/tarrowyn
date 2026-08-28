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

#[test]
fn server_crop_rotation_follows_the_validated_manifest() {
    assert_eq!(
        super::crop_kind_for_seed(0),
        tarrowyn_protocol::CropKind::Wheat
    );
    assert_eq!(
        super::crop_kind_for_seed(1),
        tarrowyn_protocol::CropKind::Turnip
    );
    assert_eq!(
        super::crop_kind_for_seed(2),
        tarrowyn_protocol::CropKind::Moonberry
    );
    assert_eq!(
        super::crop_kind_for_seed(3),
        tarrowyn_protocol::CropKind::Wheat
    );
}
