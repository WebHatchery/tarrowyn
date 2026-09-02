use super::*;
use crate::repository::models::{RepositoryState, StoredState};
use tarrowyn_protocol::{
    AuthLinkRequest, FoundationResourceKind, FoundationStorehouseAction,
    FoundationStorehouseCompletion, FoundationStorehouseContribution,
    FoundationStorehouseContributionInput, FoundationStorehouseRequest, FoundationStorehouseStage,
    Position,
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
    assert_eq!(restored.to_stored().storage_version, 28);
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

#[test]
fn storehouse_inspection_and_contribution_require_the_named_nearby_landmark() {
    let repository = repo();
    let visitor = guest(&repository, "storehouse-proximity");
    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("storehouse-proximity")
        .unwrap()
        .position = Position { x: 0, y: 0 };
    let far_request = storehouse_request(
        "storehouse-far",
        FoundationStorehouseAction::Inspect,
        "builder-mara",
        None,
    );
    let far = repository
        .foundation_storehouse(&visitor.account_token, far_request.clone())
        .unwrap()
        .data;
    assert!(!far.accepted);
    assert!(far.reason.as_deref().unwrap().contains("Walk beside"));

    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("storehouse-proximity")
        .unwrap()
        .position = Position { x: 7, y: 5 };
    let replay = repository
        .foundation_storehouse(&visitor.account_token, far_request)
        .unwrap()
        .data;
    assert_eq!(replay, far);
    let nearby = repository
        .foundation_storehouse(
            &visitor.account_token,
            storehouse_request(
                "storehouse-near",
                FoundationStorehouseAction::Inspect,
                "builder-mara",
                None,
            ),
        )
        .unwrap()
        .data;
    assert!(nearby.accepted);

    let notice_contribution = repository
        .foundation_storehouse(
            &visitor.account_token,
            storehouse_request(
                "storehouse-notice-contribution",
                FoundationStorehouseAction::Contribute,
                "first-beacon-noticeboard",
                Some(FoundationStorehouseContributionInput::Gold {
                    toward: FoundationResourceKind::Stone,
                    amount: 3,
                }),
            ),
        )
        .unwrap()
        .data;
    assert!(!notice_contribution.accepted);
    assert_eq!(notice_contribution.player.gold, nearby.player.gold);
}

#[test]
fn mixed_contributions_advance_once_and_create_one_persistent_public_storehouse() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-storehouse-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        backup_path: None,
        ..ServerConfig::default()
    };
    let repository = WorldRepository::new(config.clone());
    let hauler = guest(&repository, "storehouse-hauler");
    let patron = guest(&repository, "storehouse-patron");
    {
        let mut state = repository.state.lock().unwrap();
        let hauler = state.identities.get_mut("storehouse-hauler").unwrap();
        hauler.position = Position { x: 6, y: 7 };
        hauler.inventory.timber = 8;
        hauler.inventory.stone = 3;
        state
            .identities
            .get_mut("storehouse-patron")
            .unwrap()
            .position = Position { x: 7, y: 5 };
    }

    let stone = contribute_material(
        &repository,
        &hauler.account_token,
        "storehouse-stone-3",
        FoundationResourceKind::Stone,
        3,
    );
    assert_eq!(
        stone.storehouse.current_stage,
        FoundationStorehouseStage::SiteMarked
    );
    let foundation = contribute_material(
        &repository,
        &hauler.account_token,
        "storehouse-timber-1",
        FoundationResourceKind::Timber,
        1,
    );
    assert_eq!(
        foundation.storehouse.current_stage,
        FoundationStorehouseStage::FoundationLaid
    );
    contribute_material(
        &repository,
        &hauler.account_token,
        "storehouse-timber-5",
        FoundationResourceKind::Timber,
        5,
    );
    let frame = contribute_gold(
        &repository,
        &patron.account_token,
        "storehouse-fund-stone-1",
        FoundationResourceKind::Stone,
        3,
    );
    assert_eq!(
        frame.storehouse.current_stage,
        FoundationStorehouseStage::FrameRaised
    );
    contribute_material(
        &repository,
        &hauler.account_token,
        "storehouse-timber-2",
        FoundationResourceKind::Timber,
        2,
    );
    let final_request = storehouse_request(
        "storehouse-fund-stone-2",
        FoundationStorehouseAction::Contribute,
        "builder-mara",
        Some(FoundationStorehouseContributionInput::Gold {
            toward: FoundationResourceKind::Stone,
            amount: 6,
        }),
    );
    let completed = repository
        .foundation_storehouse(&patron.account_token, final_request.clone())
        .unwrap()
        .data;
    assert!(completed.accepted);
    assert_eq!(
        completed.storehouse.current_stage,
        FoundationStorehouseStage::Operational
    );
    assert_eq!(
        completed.player.gold,
        ServerConfig::default().starting_gold - 9
    );
    assert_eq!(
        completed
            .storehouse
            .completion
            .as_ref()
            .unwrap()
            .contributor_account_ids,
        vec![hauler.account_id, patron.account_id]
    );
    assert_eq!(storehouse_infrastructure_count(&repository), 1);
    assert!(repository.ops_health().data.integrity_ok);

    let replay = repository
        .foundation_storehouse(&patron.account_token, final_request.clone())
        .unwrap()
        .data;
    assert_eq!(replay, completed);
    assert_eq!(storehouse_infrastructure_count(&repository), 1);
    drop(repository);

    let restarted = WorldRepository::new(config);
    let patron = guest(&restarted, "storehouse-patron");
    let replay_after_restart = restarted
        .foundation_storehouse(&patron.account_token, final_request)
        .unwrap()
        .data;
    assert_eq!(replay_after_restart, completed);
    assert_eq!(storehouse_infrastructure_count(&restarted), 1);
    assert!(restarted.ops_health().data.integrity_ok);
    drop(restarted);
    let _ = std::fs::remove_file(path);
}

#[test]
fn resetting_a_contributor_anonymizes_durable_storehouse_history() {
    let repository = repo();
    let contributor = guest(&repository, "storehouse-reset");
    {
        let mut state = repository.state.lock().unwrap();
        let identity = state.identities.get_mut("storehouse-reset").unwrap();
        identity.position = Position { x: 6, y: 7 };
        identity.inventory.timber = 1;
    }
    let accepted = contribute_material(
        &repository,
        &contributor.account_token,
        "storehouse-before-reset",
        FoundationResourceKind::Timber,
        1,
    );
    assert!(accepted.accepted);

    repository
        .guest_session(tarrowyn_protocol::GuestSessionRequest {
            client_key: Some("storehouse-reset".to_owned()),
            reset: true,
        })
        .unwrap();

    let state = repository.state.lock().unwrap();
    assert_eq!(
        state.foundation_activity.storehouse.contributions[0].account_id,
        "former-resident"
    );
    drop(state);
    assert!(repository.ops_health().data.integrity_ok);
}

#[test]
fn account_link_migrates_contribution_and_replay_attribution() {
    let repository = repo();
    let contributor = guest(&repository, "storehouse-link");
    {
        let mut state = repository.state.lock().unwrap();
        let identity = state.identities.get_mut("storehouse-link").unwrap();
        identity.position = Position { x: 6, y: 7 };
        identity.inventory.timber = 1;
    }
    let request = storehouse_request(
        "storehouse-before-link",
        FoundationStorehouseAction::Contribute,
        "storehouse-site",
        Some(FoundationStorehouseContributionInput::Material {
            kind: FoundationResourceKind::Timber,
            amount: 1,
        }),
    );
    repository
        .foundation_storehouse(&contributor.account_token, request.clone())
        .unwrap();

    let linked = repository
        .auth_link(
            &contributor.account_token,
            AuthLinkRequest {
                request_id: "link-storehouse-contributor".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "storehouse-contributor-subject".to_owned(),
                display_name: Some("Storehouse Patron".to_owned()),
            },
        )
        .unwrap()
        .data;
    let replay = repository
        .foundation_storehouse(&linked.session.account_token, request)
        .unwrap()
        .data;

    assert_eq!(replay.player.account_id, linked.account_id);
    assert_eq!(
        replay.storehouse.contributions[0].account_id,
        linked.account_id
    );
    assert_eq!(
        repository
            .state
            .lock()
            .unwrap()
            .foundation_activity
            .storehouse
            .contributions[0]
            .account_id,
        linked.account_id
    );
    assert!(repository.ops_health().data.integrity_ok);
}

#[test]
fn malformed_storehouse_progress_degrades_readiness() {
    let repository = repo();
    repository
        .state
        .lock()
        .unwrap()
        .foundation_activity
        .storehouse
        .current_stage = FoundationStorehouseStage::Operational;

    let health = repository.ops_health().data;

    assert!(!health.integrity_ok);
    assert!(!health.ready);
}

fn contribute_material(
    repository: &WorldRepository,
    token: &str,
    request_id: &str,
    kind: FoundationResourceKind,
    amount: u32,
) -> tarrowyn_protocol::FoundationStorehouseResponse {
    repository
        .foundation_storehouse(
            token,
            storehouse_request(
                request_id,
                FoundationStorehouseAction::Contribute,
                "storehouse-site",
                Some(FoundationStorehouseContributionInput::Material { kind, amount }),
            ),
        )
        .unwrap()
        .data
}

fn contribute_gold(
    repository: &WorldRepository,
    token: &str,
    request_id: &str,
    toward: FoundationResourceKind,
    amount: u32,
) -> tarrowyn_protocol::FoundationStorehouseResponse {
    repository
        .foundation_storehouse(
            token,
            storehouse_request(
                request_id,
                FoundationStorehouseAction::Contribute,
                "builder-mara",
                Some(FoundationStorehouseContributionInput::Gold { toward, amount }),
            ),
        )
        .unwrap()
        .data
}

fn storehouse_request(
    request_id: &str,
    action: FoundationStorehouseAction,
    landmark_id: &str,
    contribution: Option<FoundationStorehouseContributionInput>,
) -> FoundationStorehouseRequest {
    FoundationStorehouseRequest {
        request_id: request_id.to_owned(),
        action,
        landmark_id: landmark_id.to_owned(),
        contribution,
    }
}

fn storehouse_infrastructure_count(repository: &WorldRepository) -> usize {
    repository
        .state
        .lock()
        .unwrap()
        .phase4
        .infrastructure
        .iter()
        .filter(|record| record.infrastructure_id == "first-beacon-storehouse")
        .count()
}
