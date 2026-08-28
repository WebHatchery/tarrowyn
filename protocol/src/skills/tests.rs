use super::*;

#[test]
fn school_requests_use_an_explicit_touchable_action() {
    let request = SkillRequest {
        request_id: "school-1".to_owned(),
        action: SkillAction::Teach,
        skill_id: Some("sword-fighting".to_owned()),
        target_account_id: Some("guest-2".to_owned()),
    };
    let encoded = serde_json::to_string(&request).unwrap();
    assert!(encoded.contains("\"action\":\"teach\""));
    assert!(encoded.contains("sword-fighting"));
}
