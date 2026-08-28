use super::super::super::{ServerConfig, WorldRepository};
use super::guest;
use tarrowyn_protocol::{
    FarmingAction, FarmingRequest, FieldWeather, MovementIntent, ProfessionAction, ProfessionKind,
    ProfessionRequest,
};

#[test]
fn animal_care_is_visible_persistent_and_records_husbandry_practice() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-animal-care");
    let before = repo.state(&session.account_token).unwrap().data;
    assert_eq!(before.world.animals.len(), 1);
    assert_eq!(before.world.animals[0].name, "Bellweather");
    assert_eq!(before.player.animal_condition, 2);

    for (index, (dx, dy)) in [(-1, 0), (-1, 0), (-1, 0), (-1, 0), (0, -1)]
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
                position: tarrowyn_protocol::Position { x: 3, y: 5 },
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
                position: tarrowyn_protocol::Position { x: 3, y: 5 },
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
        for (index, (dx, dy)) in [(-1, 0), (-1, 0), (-1, 0), (-1, 0), (0, -1)]
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
                    position: tarrowyn_protocol::Position { x: 3, y: 5 },
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
fn field_tool_condition_connects_active_farming_to_a_repair_order() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        world_seconds_per_tick: 10.0,
        crop_stage_seconds: 10.0,
        ..ServerConfig::default()
    });
    let requester = guest(&repo, "phase4-tool-requester");
    let provider = guest(&repo, "phase4-tool-provider");
    for (index, (dx, dy)) in [(-1, 0), (-1, 0), (-1, 0), (-1, 0), (0, -1)]
        .into_iter()
        .enumerate()
    {
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
                position: tarrowyn_protocol::Position { x: 4, y: 4 },
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
                position: tarrowyn_protocol::Position { x: 4, y: 4 },
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
                position: tarrowyn_protocol::Position { x: 4, y: 4 },
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
    for (index, (dx, dy)) in [(-1, 0), (-1, 0), (-1, 0), (-1, 0), (0, -1)]
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
    let first_plot = tarrowyn_protocol::Position { x: 4, y: 4 };
    let second_plot = tarrowyn_protocol::Position { x: 5, y: 4 };
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
            request_id: "pressure-step-sideways".to_owned(),
            dx: 1,
            dy: 0,
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
