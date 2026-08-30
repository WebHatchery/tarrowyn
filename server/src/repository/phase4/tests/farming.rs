use super::super::super::{ServerConfig, WorldRepository};
use super::guest;
use tarrowyn_protocol::{
    CropKind, CropState, FarmingAction, FarmingRequest, FieldWeather, MovementIntent,
    ProfessionAction, ProfessionKind, ProfessionRequest,
};

#[test]
fn animal_care_is_visible_persistent_and_records_husbandry_practice() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        day_length_seconds: 10.0,
        world_seconds_per_tick: 10.0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-animal-care");
    let before = repo.state(&session.account_token).unwrap().data;
    assert_eq!(before.world.animals.len(), 1);
    assert_eq!(before.world.animals[0].name, "Bellweather");
    assert_eq!(before.player.animal_condition, 2);
    repo.tick();
    assert_eq!(
        repo.state(&session.account_token)
            .unwrap()
            .data
            .player
            .animal_condition,
        1
    );

    for (index, (dx, dy)) in [(-1, 0), (-1, 0), (-1, 0), (-1, 0), (-1, 0), (0, 1)]
        .into_iter()
        .enumerate()
    {
        repo.movement(
            &session.account_token,
            MovementIntent {
                request_id: format!("animal-move-{index}"),
                dx,
                dy,
            },
        )
        .unwrap();
    }
    let cared = repo
        .farming(
            &session.account_token,
            FarmingRequest {
                request_id: "animal-care".to_owned(),
                action: FarmingAction::TendAnimal,
                position: crate::content::farm_animal_position(),
            },
        )
        .unwrap()
        .data;
    assert!(cared.accepted);
    assert_eq!(cared.animal.as_ref().unwrap().condition, 3);
    assert_eq!(cared.player.animal_condition, 3);
    let retry = repo
        .farming(
            &session.account_token,
            FarmingRequest {
                request_id: "animal-care".to_owned(),
                action: FarmingAction::TendAnimal,
                position: crate::content::farm_animal_position(),
            },
        )
        .unwrap()
        .data;
    assert_eq!(retry, cared);
    assert_eq!(
        repo.state(&session.account_token)
            .unwrap()
            .data
            .world
            .animals[0]
            .condition,
        3
    );
    assert_eq!(
        repo.skills(&session.account_token)
            .unwrap()
            .data
            .skills
            .iter()
            .find(|skill| skill.skill_id == "animal-husbandry")
            .unwrap()
            .mastery,
        1
    );
}

#[test]
fn animal_condition_survives_repository_restart() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-phase4-animal-{}.json",
        std::process::id()
    ));
    let path_string = path.to_string_lossy().into_owned();
    let config = ServerConfig {
        persistence_path: Some(path_string),
        movement_cooldown_ticks: 0,
        ..ServerConfig::default()
    };
    {
        let repo = WorldRepository::new(config.clone());
        let session = guest(&repo, "phase4-animal-restart");
        for (index, (dx, dy)) in [(-1, 0), (-1, 0), (-1, 0), (-1, 0), (-1, 0), (0, 1)]
            .into_iter()
            .enumerate()
        {
            repo.movement(
                &session.account_token,
                MovementIntent {
                    request_id: format!("restart-animal-move-{index}"),
                    dx,
                    dy,
                },
            )
            .unwrap();
        }
        assert!(
            repo.farming(
                &session.account_token,
                FarmingRequest {
                    request_id: "restart-animal-care".to_owned(),
                    action: FarmingAction::TendAnimal,
                    position: crate::content::farm_animal_position(),
                },
            )
            .unwrap()
            .data
            .accepted
        );
    }
    let reopened = WorldRepository::new(config);
    let session = guest(&reopened, "phase4-animal-restart");
    let snapshot = reopened.state(&session.account_token).unwrap().data;
    assert_eq!(snapshot.world.animals[0].condition, 3);
    let _ = std::fs::remove_file(path);
}

#[test]
fn animal_care_requires_distance_to_the_actual_animal_position() {
    let repo = WorldRepository::new(ServerConfig::default());
    let session = guest(&repo, "phase4-animal-distance");
    let animal_position = crate::content::farm_animal_position();
    let request_position = tarrowyn_protocol::Position {
        x: animal_position.x,
        y: animal_position.y - 1,
    };
    {
        let mut state = repo.state.lock().expect("repository lock");
        state
            .identities
            .get_mut(&session.client_key)
            .expect("guest identity")
            .position = tarrowyn_protocol::Position {
            x: animal_position.x,
            y: animal_position.y - 2,
        };
        state.phase4.animals[0].condition = 1;
    }

    let response = repo
        .farming(
            &session.account_token,
            FarmingRequest {
                request_id: "animal-distance-boundary".to_owned(),
                action: FarmingAction::TendAnimal,
                position: request_position,
            },
        )
        .unwrap()
        .data;

    assert!(!response.accepted);
    assert!(response
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("beside the animal")));
    assert_eq!(response.animal.expect("animal response").condition, 1);
}

#[test]
fn legacy_bellweather_position_restores_beside_manifest_fields() {
    let restored = super::super::restore_animals(vec![tarrowyn_protocol::FarmAnimal {
        animal_id: "bellweather-goat".to_owned(),
        name: "Bellweather".to_owned(),
        kind: tarrowyn_protocol::FarmAnimalKind::Goat,
        position: tarrowyn_protocol::Position { x: 3, y: 5 },
        condition: 1,
        max_condition: 3,
        last_cared_tick: 9,
        last_cared_day: 2,
    }]);

    assert_eq!(restored[0].position, crate::content::farm_animal_position());
    assert_eq!(restored[0].condition, 1);
    assert_eq!(restored[0].last_cared_tick, 9);
}

#[test]
fn field_tool_condition_connects_active_farming_to_a_repair_order() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        world_seconds_per_tick: 10.0,
        crop_stage_seconds: 10.0,
        ..ServerConfig::default()
    });
    let requester = guest(&repo, "phase4-tool-requester");
    let provider = guest(&repo, "phase4-tool-provider");
    let plot_position = crate::content::farm_plot_positions()[2];
    for (index, (dx, dy)) in [(1, 0), (1, 0), (0, 1), (0, 1)].into_iter().enumerate() {
        repo.movement(
            &requester.account_token,
            MovementIntent {
                request_id: format!("tool-move-{index}"),
                dx,
                dy,
            },
        )
        .unwrap();
    }
    assert!(
        repo.farming(
            &requester.account_token,
            FarmingRequest {
                request_id: "tool-plant".to_owned(),
                action: FarmingAction::Plant,
                position: plot_position,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    let tended = repo
        .farming(
            &requester.account_token,
            FarmingRequest {
                request_id: "tool-tend".to_owned(),
                action: FarmingAction::Tend,
                position: plot_position,
            },
        )
        .unwrap()
        .data;
    assert!(tended.accepted);
    assert_eq!(tended.player.field_tool_condition, 2);
    for _ in 0..3 {
        repo.tick();
    }
    assert!(
        repo.farming(
            &requester.account_token,
            FarmingRequest {
                request_id: "tool-harvest".to_owned(),
                action: FarmingAction::Harvest,
                position: plot_position,
            },
        )
        .unwrap()
        .data
        .accepted
    );

    let order = repo
        .profession_order(
            &requester.account_token,
            ProfessionRequest {
                request_id: "tool-order".to_owned(),
                action: ProfessionAction::CreateOrder,
                order_id: None,
                profession: Some(ProfessionKind::Carpenter),
                capability_id: None,
                service: Some("Repair the farmer's field tool".to_owned()),
                timing_score: None,
            },
        )
        .unwrap()
        .data
        .order
        .unwrap();
    assert!(
        repo.profession_order(
            &provider.account_token,
            ProfessionRequest {
                request_id: "tool-learn".to_owned(),
                action: ProfessionAction::LearnCapability,
                order_id: None,
                profession: Some(ProfessionKind::Carpenter),
                capability_id: None,
                service: None,
                timing_score: None,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    assert!(
        repo.profession_order(
            &provider.account_token,
            ProfessionRequest {
                request_id: "tool-accept".to_owned(),
                action: ProfessionAction::AcceptOrder,
                order_id: Some(order.order_id.clone()),
                profession: None,
                capability_id: None,
                service: None,
                timing_score: None,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    assert!(
        repo.profession_order(
            &provider.account_token,
            ProfessionRequest {
                request_id: "tool-complete".to_owned(),
                action: ProfessionAction::CompleteOrder,
                order_id: Some(order.order_id),
                profession: None,
                capability_id: None,
                service: None,
                timing_score: Some(100),
            },
        )
        .unwrap()
        .data
        .accepted
    );
    assert_eq!(
        repo.inventory(&requester.account_token)
            .unwrap()
            .data
            .field_tool_condition,
        3
    );
}

#[test]
fn field_outlook_and_recent_tending_buffer_environmental_pressure() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        day_length_seconds: 10.0,
        world_seconds_per_tick: 10.0,
        crop_stage_seconds: 10.0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-field-pressure");
    let first_plot = crate::content::farm_plot_positions()[0];
    let second_plot = crate::content::farm_plot_positions()[1];
    for (index, (dx, dy)) in [
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (0, 1),
        (0, 1),
    ]
    .into_iter()
    .enumerate()
    {
        repo.movement(
            &session.account_token,
            MovementIntent {
                request_id: format!("pressure-move-{index}"),
                dx,
                dy,
            },
        )
        .unwrap();
    }
    assert!(
        repo.farming(
            &session.account_token,
            FarmingRequest {
                request_id: "pressure-plant-one".to_owned(),
                action: FarmingAction::Plant,
                position: first_plot,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    repo.movement(
        &session.account_token,
        MovementIntent {
            request_id: "pressure-step-down".to_owned(),
            dx: 0,
            dy: 1,
        },
    )
    .unwrap();
    assert!(
        repo.farming(
            &session.account_token,
            FarmingRequest {
                request_id: "pressure-plant-two".to_owned(),
                action: FarmingAction::Plant,
                position: second_plot,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    assert!(
        repo.farming(
            &session.account_token,
            FarmingRequest {
                request_id: "pressure-tend-two".to_owned(),
                action: FarmingAction::Tend,
                position: second_plot,
            },
        )
        .unwrap()
        .data
        .accepted
    );

    repo.tick();
    let snapshot = repo.state(&session.account_token).unwrap().data;
    assert_eq!(snapshot.player.field_weather, FieldWeather::Clear);
    assert_eq!(snapshot.player.field_pest_pressure, 1);
    assert_eq!(
        snapshot
            .world
            .plots
            .iter()
            .find(|plot| plot.position == first_plot)
            .unwrap()
            .crop
            .unwrap()
            .quality,
        0
    );
    assert_eq!(
        snapshot
            .world
            .plots
            .iter()
            .find(|plot| plot.position == second_plot)
            .unwrap()
            .crop
            .unwrap()
            .quality,
        2
    );
}

#[test]
fn farming_rewards_saturate_player_counters_at_the_numeric_ceiling() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "farming-counter-ceiling");
    let plot_position = crate::content::farm_plot_positions()[0];
    {
        let mut state = repository.state.lock().unwrap();
        let identity = state.identities.get_mut("farming-counter-ceiling").unwrap();
        identity.position = plot_position;
        identity.inventory.wheat = u32::MAX;
        identity.gold = u32::MAX;
        identity.skill = u32::MAX;
        state
            .plots
            .iter_mut()
            .find(|plot| plot.position == plot_position)
            .unwrap()
            .crop = Some(CropState {
            kind: CropKind::Wheat,
            stage: CropState::MATURE_STAGE,
            quality: 1,
            planted_tick: 0,
            last_tended_tick: None,
        });
    }

    let response = repository
        .farming(
            &session.account_token,
            FarmingRequest {
                request_id: "farming-counter-ceiling-harvest".to_owned(),
                action: FarmingAction::Harvest,
                position: plot_position,
            },
        )
        .unwrap()
        .data;
    assert!(response.accepted);
    assert_eq!(response.player.inventory.wheat, u32::MAX);
    assert_eq!(response.player.gold, u32::MAX);
    assert_eq!(response.player.skill, u32::MAX);
}

#[test]
fn crop_tending_rejects_a_same_beat_burst_without_spending_tool_condition() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "farming-same-beat");
    let plot_position = crate::content::farm_plot_positions()[0];
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .identities
            .get_mut(&session.client_key)
            .expect("identity")
            .position = plot_position;
        state
            .plots
            .iter_mut()
            .find(|plot| plot.position == plot_position)
            .expect("plot")
            .crop = Some(CropState {
            kind: CropKind::Wheat,
            stage: 0,
            quality: 1,
            planted_tick: 0,
            last_tended_tick: None,
        });
    }

    let request = |request_id: &str| FarmingRequest {
        request_id: request_id.to_owned(),
        action: FarmingAction::Tend,
        position: plot_position,
    };
    let first = repository.farming(&session.account_token, request("farming-first-tend"));
    assert!(first.expect("first tending").data.accepted);
    let rejected = repository
        .farming(&session.account_token, request("farming-second-tend"))
        .expect("same-beat tending response")
        .data;
    assert!(!rejected.accepted);
    assert!(rejected
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("already been tended")));
    assert_eq!(rejected.player.field_tool_condition, 2);
    assert_eq!(
        rejected
            .plot
            .expect("plot response")
            .crop
            .expect("crop response")
            .stage,
        1
    );

    repository.tick();
    let next_beat = repository
        .farming(&session.account_token, request("farming-next-tend"))
        .expect("next-beat tending response")
        .data;
    assert!(next_beat.accepted);
    assert_eq!(next_beat.player.field_tool_condition, 1);
}
