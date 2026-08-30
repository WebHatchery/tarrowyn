use super::*;

#[test]
fn farming_without_a_nearby_target_explains_where_to_go() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.projection.world.tiles = FlatGrid::new(3, 2, TileKind::Meadow);
    client.projection.player_position = TilePos::new(1, 1);

    client.queue_farming(FarmingAction::Plant);

    assert!(client.farming_queue.is_empty());
    assert!(client.status_message.contains("shared field plot"));
}

#[test]
fn farming_actions_choose_a_nearby_plot_matching_the_action() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.projection.world.tiles = FlatGrid::new(3, 2, TileKind::Field);
    client.projection.player_position = TilePos::new(1, 1);
    client
        .projection
        .world
        .tiles
        .set(TilePos::new(1, 1), TileKind::Meadow);
    client.projection.world.crops.set(
        TilePos::new(0, 1),
        Some(crate::state::CropState {
            kind: crate::state::CropKind::Wheat,
            stage: crate::state::CropState::MATURE_STAGE,
        }),
    );
    client.projection.world.crops.set(
        TilePos::new(1, 0),
        Some(crate::state::CropState {
            kind: crate::state::CropKind::Turnip,
            stage: 1,
        }),
    );

    client.queue_farming(FarmingAction::Harvest);
    assert!(client.farming_pending());
    client.queue_farming(FarmingAction::Tend);
    assert!(client.status_message.contains("already waiting"));
    assert_eq!(client.farming_queue.len(), 1);
    assert_eq!(
        client.farming_queue.back().map(|request| request.position),
        Some(Position { x: 0, y: 1 })
    );
    client.farming_queue.clear();

    client.queue_farming(FarmingAction::Tend);
    assert_eq!(
        client.farming_queue.back().map(|request| request.position),
        Some(Position { x: 1, y: 0 })
    );
    client.farming_queue.clear();

    client.queue_farming(FarmingAction::Plant);
    assert_eq!(
        client.farming_queue.back().map(|request| request.position),
        Some(Position { x: 2, y: 1 })
    );
}

#[test]
fn farming_success_notice_names_crop_and_plot() {
    let plot = FarmPlot {
        position: Position { x: 2, y: 3 },
        crop: Some(CropState {
            kind: CropKind::Turnip,
            stage: 2,
            quality: 2,
            planted_tick: 4,
            last_tended_tick: Some(5),
        }),
    };

    assert_eq!(
        super::super::commands::farming_success_notice(FarmingAction::Tend, Some(plot), None),
        "Tended Turnip at plot (2, 3); growth stage 2/3."
    );
    assert_eq!(
        super::super::commands::farming_success_notice(FarmingAction::Harvest, Some(plot), None),
        "Harvested Turnip from plot (2, 3)."
    );
}

#[test]
fn farming_success_notice_names_animal_condition() {
    let animal = FarmAnimal {
        animal_id: "bellweather-goat".to_owned(),
        name: "Bellweather".to_owned(),
        kind: FarmAnimalKind::Goat,
        position: Position { x: 1, y: 1 },
        condition: 3,
        max_condition: 3,
        last_cared_tick: 8,
        last_cared_day: 2,
    };

    assert_eq!(
        super::super::commands::farming_success_notice(
            FarmingAction::TendAnimal,
            None,
            Some(&animal)
        ),
        "Cared for Bellweather • condition 3/3."
    );
}

#[test]
fn animal_care_targets_the_nearby_animal() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.projection.player_position = TilePos::new(1, 1);
    client.projection.animals = vec![FarmAnimal {
        animal_id: "bellweather-goat".to_owned(),
        name: "Bellweather".to_owned(),
        kind: tarrowyn_protocol::FarmAnimalKind::Goat,
        position: Position { x: 1, y: 1 },
        condition: 2,
        max_condition: 3,
        last_cared_tick: 0,
        last_cared_day: 1,
    }];

    client.queue_farming(FarmingAction::TendAnimal);

    assert!(matches!(
        client.farming_queue.front(),
        Some(request) if request.action == FarmingAction::TendAnimal
            && request.position == Position { x: 1, y: 1 }
    ));
}

#[test]
fn farming_backpressure_does_not_claim_pending_confirmation() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.projection.world.tiles = FlatGrid::new(3, 2, TileKind::Field);
    client.projection.player_position = TilePos::new(1, 1);
    client.projection.world.crops.set(
        TilePos::new(1, 0),
        Some(crate::state::CropState {
            kind: crate::state::CropKind::Turnip,
            stage: 1,
        }),
    );
    for index in 0..super::super::queue::MAX_PENDING_COMMANDS {
        client.farming_queue.push_back(FarmingRequest {
            request_id: format!("queued-{index}"),
            action: FarmingAction::Plant,
            position: Position { x: 0, y: 0 },
        });
    }

    client.queue_farming(FarmingAction::Tend);

    assert!(!client.action_awaiting_confirmation);
    assert!(client.pending_request_id.is_none());
    assert!(client.status_message.contains("ledger is busy"));
    assert_eq!(
        client.farming_queue.len(),
        super::super::queue::MAX_PENDING_COMMANDS
    );
}
