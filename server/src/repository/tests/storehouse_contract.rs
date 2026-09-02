use super::*;
use crate::repository::models::{RepositoryState, StoredState};
use tarrowyn_protocol::{
    FoundationResourceKind, FoundationStorehouseCompletion, FoundationStorehouseContribution,
    FoundationStorehouseContributionInput, FoundationStorehouseStage,
};

#[test]
fn storage_version_twenty_five_defaults_the_first_storehouse_contract() {
    let repository = repo();
    let stored = repository.state.lock().unwrap().to_stored();
    let mut json = serde_json::to_value(stored).unwrap();
    json["storage_version"] = serde_json::json!(25);
    json["foundation_activity"]
        .as_object_mut()
        .unwrap()
        .remove("storehouse");

    let legacy: StoredState = serde_json::from_value(json).unwrap();
    let restored = RepositoryState::from_stored_at(legacy, &ServerConfig::default(), 0);
    let storehouse = &restored.foundation_activity.storehouse;

    assert_eq!(storehouse.project_id, "first-beacon-storehouse");
    assert_eq!(storehouse.requirements.len(), 2);
    assert_eq!(storehouse.stages.len(), 4);
    assert_eq!(restored.to_stored().storage_version, 26);
}

#[test]
fn attributed_stage_and_completion_records_survive_the_repository_boundary() {
    let repository = repo();
    let builder = guest(&repository, "storehouse-contract-builder");
    {
        let mut state = repository.state.lock().unwrap();
        let storehouse = &mut state.foundation_activity.storehouse;
        storehouse.revision = 3;
        storehouse.current_stage = FoundationStorehouseStage::Operational;
        storehouse
            .contributions
            .push(FoundationStorehouseContribution {
                contribution_id: "storehouse-contribution-1".to_owned(),
                account_id: builder.account_id.clone(),
                input: FoundationStorehouseContributionInput::Material {
                    kind: FoundationResourceKind::Timber,
                    amount: 8,
                },
                credited_kind: FoundationResourceKind::Timber,
                credited_units: 8,
                contributed_tick: 4,
            });
        storehouse.completion = Some(FoundationStorehouseCompletion {
            completed_tick: 9,
            contributor_account_ids: vec![builder.account_id],
            operational_infrastructure_id: storehouse.operational_infrastructure_id.clone(),
        });
    }

    let stored = repository.state.lock().unwrap().to_stored();
    let restored = RepositoryState::from_stored(stored, &ServerConfig::default());

    assert_eq!(
        restored.foundation_activity.storehouse,
        repository
            .state
            .lock()
            .unwrap()
            .foundation_activity
            .storehouse
    );
}
