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

#[test]
fn root_practice_requests_keep_the_entry_skill_explicit() {
    let request = SkillRequest {
        request_id: "practice-1".to_owned(),
        action: SkillAction::Practice,
        skill_id: Some("fishing".to_owned()),
        target_account_id: None,
    };
    let encoded = serde_json::to_string(&request).expect("practice request should encode");
    assert!(encoded.contains("\"action\":\"practice\""));
    assert!(encoded.contains("\"skill_id\":\"fishing\""));
}
