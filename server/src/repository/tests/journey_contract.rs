use super::*;
use crate::repository::models::{RepositoryState, StoredState};
use tarrowyn_protocol::{
    AuthLinkRequest, FoundationForgeAction, FoundationForgeRequest,
    FoundationJourneyFutureGoalState, FoundationResourceAction, FoundationResourceKind,
    FoundationResourceRequest, FoundationStorehouseAction, FoundationStorehouseContributionInput,
    FoundationStorehouseRequest,
};

#[test]
fn storage_version_twenty_six_defaults_personal_journey_progress() {
    let repository = repo();
    guest(&repository, "journey-legacy");
    let stored = repository.state.lock().unwrap().to_stored();
    let mut json = serde_json::to_value(stored).unwrap();
    json["storage_version"] = serde_json::json!(26);
    let identity = json["identities"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap()
        .as_object_mut()
        .unwrap();
    identity.remove("foundation_journey");

    let legacy: StoredState = serde_json::from_value(json).unwrap();
    let restored = RepositoryState::from_stored_at(legacy, &ServerConfig::default(), 0);
    let progress = &restored.identities["journey-legacy"].foundation_journey;

    assert_eq!(progress, &Default::default());
    assert_eq!(restored.to_stored().storage_version, 27);
}

#[test]
fn accepted_canonical_actions_credit_the_complete_journey_once_and_survive_restart() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-journey-{}-{}.json",
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
    let resident = guest(&repository, "journey-resident");
    let neighbour = guest(&repository, "journey-neighbour");
    assert_progress(
        &repository,
        &resident.account_token,
        1,
        "consult-first-need",
    );

    set_position(&repository, "journey-resident", Position { x: 9, y: 6 });
    let consult = repository
        .foundation_interaction(
            &resident.account_token,
            tarrowyn_protocol::FoundationInteractionRequest {
                request_id: "journey-consult".to_owned(),
                interaction_id: "read-local-needs".to_owned(),
            },
        )
        .unwrap();
    assert!(consult.data.accepted);

    let plot_position = repository.state.lock().unwrap().plots[0].position;
    set_position(&repository, "journey-resident", plot_position);
    assert!(
        farm(
            &repository,
            &resident.account_token,
            "journey-plant",
            FarmingAction::Plant,
            plot_position,
        )
        .accepted
    );

    set_position(&repository, "journey-resident", Position { x: 12, y: 3 });
    assert!(
        resource(
            &repository,
            &resident.account_token,
            "journey-log",
            "whisperwood-edge-node",
            FoundationResourceAction::Log,
        )
        .accepted
    );
    set_position(&repository, "journey-resident", Position { x: 10, y: 4 });
    assert!(
        resource(
            &repository,
            &resident.account_token,
            "journey-mine",
            "shallow-stone-seam-node",
            FoundationResourceAction::Mine,
        )
        .accepted
    );
    assert!(
        resource(
            &repository,
            &resident.account_token,
            "journey-mine-second",
            "shallow-stone-seam-node",
            FoundationResourceAction::Mine,
        )
        .accepted
    );

    {
        let mut state = repository.state.lock().unwrap();
        let identity = state.identities.get_mut("journey-resident").unwrap();
        identity.inventory.timber = 2;
        identity.inventory.iron_ore = 2;
    }
    for (request_id, action) in [
        ("journey-charcoal", FoundationForgeAction::BurnCharcoal),
        ("journey-handle", FoundationForgeAction::ShapeHandle),
        ("journey-tool", FoundationForgeAction::ForgeFieldTool),
    ] {
        assert!(
            repository
                .foundation_forge(
                    &resident.account_token,
                    FoundationForgeRequest {
                        request_id: request_id.to_owned(),
                        action,
                    },
                )
                .unwrap()
                .data
                .accepted
        );
    }

    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("journey-resident")
        .unwrap()
        .inventory
        .wheat = 1;
    let created = repository
        .trade(
            &resident.account_token,
            TradeRequest {
                request_id: "journey-trade-create".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(neighbour.account_id.clone()),
                offer: Some(TradeBundle {
                    wheat: 1,
                    ..TradeBundle::default()
                }),
                request: Some(TradeBundle {
                    gold: 1,
                    ..TradeBundle::default()
                }),
            },
        )
        .unwrap()
        .data;
    let trade_id = created.trade.unwrap().trade_id;
    assert!(
        repository
            .trade(
                &neighbour.account_token,
                TradeRequest {
                    request_id: "journey-trade-accept".to_owned(),
                    action: TradeAction::Accept,
                    trade_id: Some(trade_id),
                    recipient_account_id: None,
                    offer: None,
                    request: None,
                },
            )
            .unwrap()
            .data
            .accepted
    );

    {
        let mut state = repository.state.lock().unwrap();
        let identity = state.identities.get_mut("journey-resident").unwrap();
        identity.position = Position { x: 6, y: 7 };
        identity.inventory.stone = 1;
    }
    assert!(
        repository
            .foundation_storehouse(
                &resident.account_token,
                FoundationStorehouseRequest {
                    request_id: "journey-storehouse".to_owned(),
                    action: FoundationStorehouseAction::Contribute,
                    landmark_id: "storehouse-site".to_owned(),
                    contribution: Some(FoundationStorehouseContributionInput::Material {
                        kind: FoundationResourceKind::Stone,
                        amount: 1,
                    }),
                },
            )
            .unwrap()
            .data
            .accepted
    );

    mature_plot(&repository, plot_position);
    set_position(&repository, "journey-resident", plot_position);
    assert!(
        farm(
            &repository,
            &resident.account_token,
            "journey-harvest",
            FarmingAction::Harvest,
            plot_position,
        )
        .accepted
    );
    assert!(
        farm(
            &repository,
            &resident.account_token,
            "journey-replant",
            FarmingAction::Plant,
            plot_position,
        )
        .accepted
    );
    let complete = repository
        .foundation_journey(&resident.account_token)
        .unwrap()
        .data;
    assert_eq!(complete.completed_milestones, 12);
    assert_eq!(complete.progress.revision, 13);
    assert!(complete.progress.completed_tick.is_some());
    assert_eq!(
        complete.progress.future_goal_state,
        FoundationJourneyFutureGoalState::Active
    );
    assert!(complete.next_milestone.is_none());

    drop(repository);
    let restarted = WorldRepository::new(config);
    let resident = guest(&restarted, "journey-resident");
    let after_restart = restarted
        .foundation_journey(&resident.account_token)
        .unwrap()
        .data;
    assert_eq!(after_restart.progress, complete.progress);

    mature_plot(&restarted, plot_position);
    let final_harvest = farm(
        &restarted,
        &resident.account_token,
        "journey-return-harvest",
        FarmingAction::Harvest,
        plot_position,
    );
    assert!(final_harvest.accepted);
    let fulfilled = restarted
        .foundation_journey(&resident.account_token)
        .unwrap()
        .data;
    assert_eq!(
        fulfilled.progress.future_goal_state,
        FoundationJourneyFutureGoalState::Complete
    );
    assert_eq!(fulfilled.progress.revision, 14);
    assert!(
        farm(
            &restarted,
            &resident.account_token,
            "journey-return-harvest",
            FarmingAction::Harvest,
            plot_position,
        )
        .accepted
    );
    assert_eq!(
        restarted
            .foundation_journey(&resident.account_token)
            .unwrap()
            .data
            .progress
            .revision,
        14
    );
    assert!(restarted.ops_health().data.integrity_ok);
    drop(restarted);
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejected_actions_do_not_credit_journey_progress() {
    let repository = repo();
    let resident = guest(&repository, "journey-rejected");
    set_position(&repository, "journey-rejected", Position { x: 0, y: 0 });

    let interaction = repository
        .foundation_interaction(
            &resident.account_token,
            tarrowyn_protocol::FoundationInteractionRequest {
                request_id: "journey-far-consult".to_owned(),
                interaction_id: "read-local-needs".to_owned(),
            },
        )
        .unwrap();
    let gather = resource(
        &repository,
        &resident.account_token,
        "journey-far-log",
        "whisperwood-edge-node",
        FoundationResourceAction::Log,
    );

    assert!(!interaction.data.accepted);
    assert!(!gather.accepted);
    assert_progress(
        &repository,
        &resident.account_token,
        1,
        "consult-first-need",
    );
}

#[test]
fn guest_reset_starts_a_new_journey_with_only_the_arrival_credit() {
    let repository = repo();
    let resident = guest(&repository, "journey-reset");
    set_position(&repository, "journey-reset", Position { x: 9, y: 6 });
    assert!(
        repository
            .foundation_interaction(
                &resident.account_token,
                tarrowyn_protocol::FoundationInteractionRequest {
                    request_id: "journey-reset-consult".to_owned(),
                    interaction_id: "read-local-needs".to_owned(),
                },
            )
            .unwrap()
            .data
            .accepted
    );
    assert_progress(
        &repository,
        &resident.account_token,
        2,
        "plant-common-field",
    );

    let replacement = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("journey-reset".to_owned()),
            reset: true,
        })
        .unwrap()
        .data;

    assert_ne!(replacement.character_id, resident.character_id);
    assert_progress(
        &repository,
        &replacement.account_token,
        1,
        "consult-first-need",
    );
}

#[test]
fn account_link_preserves_journey_progress_and_malformed_credit_degrades_readiness() {
    let repository = repo();
    let resident = guest(&repository, "journey-link");
    set_position(&repository, "journey-link", Position { x: 9, y: 6 });
    repository
        .foundation_interaction(
            &resident.account_token,
            tarrowyn_protocol::FoundationInteractionRequest {
                request_id: "journey-link-consult".to_owned(),
                interaction_id: "read-local-needs".to_owned(),
            },
        )
        .unwrap();
    let before = repository
        .foundation_journey(&resident.account_token)
        .unwrap()
        .data
        .progress;
    let linked = repository
        .auth_link(
            &resident.account_token,
            AuthLinkRequest {
                request_id: "journey-link-account".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "journey-link-subject".to_owned(),
                display_name: Some("Returning Wayfarer".to_owned()),
            },
        )
        .unwrap()
        .data;
    assert_eq!(
        repository
            .foundation_journey(&linked.session.account_token)
            .unwrap()
            .data
            .progress,
        before
    );
    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("journey-link")
        .unwrap()
        .foundation_journey
        .credits[0]
        .evidence_ref = String::new();
    assert!(!repository.ops_health().data.integrity_ok);
}

fn assert_progress(
    repository: &WorldRepository,
    token: &str,
    completed: u16,
    next_milestone_id: &str,
) {
    let projection = repository.foundation_journey(token).unwrap().data;
    assert_eq!(projection.completed_milestones, completed);
    assert_eq!(
        projection.next_milestone.unwrap().milestone_id,
        next_milestone_id
    );
}

fn set_position(repository: &WorldRepository, identity_key: &str, position: Position) {
    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut(identity_key)
        .unwrap()
        .position = position;
}

fn resource(
    repository: &WorldRepository,
    token: &str,
    request_id: &str,
    node_id: &str,
    action: FoundationResourceAction,
) -> tarrowyn_protocol::FoundationResourceResponse {
    repository
        .foundation_resource(
            token,
            FoundationResourceRequest {
                request_id: request_id.to_owned(),
                node_id: node_id.to_owned(),
                action,
            },
        )
        .unwrap()
        .data
}

fn farm(
    repository: &WorldRepository,
    token: &str,
    request_id: &str,
    action: FarmingAction,
    position: Position,
) -> FarmingResponse {
    repository
        .farming(
            token,
            FarmingRequest {
                request_id: request_id.to_owned(),
                action,
                position,
            },
        )
        .unwrap()
        .data
}

fn mature_plot(repository: &WorldRepository, position: Position) {
    let mut state = repository.state.lock().unwrap();
    state
        .plots
        .iter_mut()
        .find(|plot| plot.position == position)
        .unwrap()
        .crop
        .as_mut()
        .unwrap()
        .stage = CropState::MATURE_STAGE;
}
