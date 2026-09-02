use super::*;
use crate::repository::models::{RepositoryState, StoredState};
use tarrowyn_protocol::FoundationFieldToolKind;

#[test]
fn storage_version_twenty_two_defaults_new_forge_ledger_and_materials() {
    let repository = repo();
    let session = guest(&repository, "legacy-forge-contract");
    let player = repository.inventory(&session.account_token).unwrap().data;
    let mut player_json = serde_json::to_value(player).unwrap();
    player_json
        .as_object_mut()
        .unwrap()
        .remove("field_tool_kind");
    let legacy_player: tarrowyn_protocol::PlayerProjection =
        serde_json::from_value(player_json).unwrap();
    assert_eq!(
        legacy_player.field_tool_kind,
        FoundationFieldToolKind::Crude
    );

    let stored = repository.state.lock().unwrap().to_stored();
    let mut json = serde_json::to_value(stored).unwrap();
    json["storage_version"] = serde_json::json!(22);
    let identity = json["identities"]["legacy-forge-contract"]
        .as_object_mut()
        .unwrap();
    identity.remove("field_tool_kind");
    identity.remove("foundation_forge_results");
    let inventory = identity["inventory"].as_object_mut().unwrap();
    inventory.remove("charcoal");
    inventory.remove("tool_handles");

    let legacy: StoredState = serde_json::from_value(json).unwrap();
    let restored = RepositoryState::from_stored_at(legacy, &ServerConfig::default(), 0);
    let restored_identity = &restored.identities["legacy-forge-contract"];

    assert_eq!(
        restored_identity.field_tool_kind,
        FoundationFieldToolKind::Crude
    );
    assert_eq!(restored_identity.inventory.charcoal, 0);
    assert_eq!(restored_identity.inventory.tool_handles, 0);
    assert!(restored_identity.foundation_forge_results.is_empty());
    assert_eq!(restored.to_stored().storage_version, 24);
}

#[test]
fn persistent_integrity_uses_the_tool_kinds_typed_condition_ceiling() {
    let repository = repo();
    guest(&repository, "iron-tool-integrity");
    {
        let mut state = repository.state.lock().unwrap();
        let identity = state.identities.get_mut("iron-tool-integrity").unwrap();
        identity.field_tool_kind = FoundationFieldToolKind::Iron;
        identity.field_tool_condition = FoundationFieldToolKind::Iron.max_condition();
    }
    assert!(repository.ops_health().data.integrity_ok);

    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("iron-tool-integrity")
        .unwrap()
        .field_tool_condition = FoundationFieldToolKind::Iron.max_condition() + 1;

    assert!(!repository.ops_health().data.integrity_ok);
}
