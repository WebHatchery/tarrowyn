use super::*;
use crate::state::{CropKind, CropState, TileKind, WorldState};
use macroquad_toolkit::grid::FlatGrid;
use tarrowyn_protocol::{
    FieldWeather, FoundationActivityState, FoundationCooperationWorkCredit,
    FoundationCooperationWorkKind, FoundationFieldToolKind, FoundationForgeAction,
    FoundationForgeMaterialAmount, FoundationForgeMaterialKind, FoundationInteraction,
    FoundationLandmark, FoundationResourceDeposit, FoundationResourceKind, FoundationResourceNode,
    FoundationStorehouseCompletion, FoundationStorehouseContribution,
    FoundationStorehouseContributionInput, FoundationStorehouseStage, Position, TradeBundle,
    TradeOffer, TradeStatus,
};

#[test]
fn journey_guidance_names_progress_and_the_concrete_next_step() {
    let contract = tarrowyn_protocol::FoundationJourneyContract::default();
    let next = contract.milestones[1].clone();
    let projection = tarrowyn_protocol::FoundationJourneyProjection {
        contract,
        progress: Default::default(),
        completed_milestones: 1,
        total_milestones: 12,
        next_milestone: Some(next),
        next_action: "Talk to Mara.".to_owned(),
    };

    let guidance = journey_guidance(Some(&projection)).unwrap();
    assert!(guidance.contains("NEXT 2/12"));
    assert!(guidance.contains("Ask what the camp needs"));
    assert!(guidance.contains("Talk to Mara or read the noticeboard"));
}

#[test]
fn property_touch_choice_previews_before_placement() {
    let projection = tarrowyn_protocol::FoundationPropertyProjection {
        contract: Default::default(),
        properties: Vec::new(),
        own_property: None,
        placement_preview: None,
    };
    let choice = super::property::nearby_choice(
        &projection,
        &Default::default(),
        macroquad_toolkit::grid::TilePos::new(3, 8),
        Some("owner"),
        Some(&Default::default()),
        Some(12),
        true,
    )
    .unwrap();
    assert_eq!(choice.actions[0].label, "Check site");
    assert_eq!(
        choice.actions[0].command,
        "foundation-property:preview:4:8:south"
    );
}

#[test]
fn weathered_owner_property_exposes_touch_first_controls() {
    let state = tarrowyn_protocol::FoundationPropertyState {
        property_id: "personal-property-1".to_owned(),
        owner_account_id: "owner".to_owned(),
        owner_name: "Owner".to_owned(),
        stage: tarrowyn_protocol::FoundationPropertyStage::Tent,
        anchor: Position { x: 4, y: 8 },
        entrance: tarrowyn_protocol::FoundationPropertyDirection::South,
        access: tarrowyn_protocol::FoundationPropertyAccess::OwnerOnly,
        revision: 2,
        condition: 90,
        last_maintained_unix_millis: 1,
        storage: Default::default(),
        storage_capacity: 8,
    };
    let projection = tarrowyn_protocol::FoundationPropertyProjection {
        contract: Default::default(),
        properties: vec![tarrowyn_protocol::FoundationPropertySummary {
            property_id: state.property_id.clone(),
            owner_account_id: state.owner_account_id.clone(),
            owner_name: state.owner_name.clone(),
            stage: state.stage,
            anchor: state.anchor,
            entrance: state.entrance,
            access: state.access,
            revision: state.revision,
            condition: state.condition,
            stored_units: 0,
            storage_capacity: state.storage_capacity,
        }],
        own_property: Some(state),
        placement_preview: None,
    };
    let inventory = tarrowyn_protocol::Inventory {
        timber: 4,
        stone: 2,
        ..Default::default()
    };
    let choice = super::property::nearby_choice(
        &projection,
        &Default::default(),
        macroquad_toolkit::grid::TilePos::new(3, 8),
        Some("owner"),
        Some(&inventory),
        Some(12),
        true,
    )
    .unwrap();
    let labels = choice
        .actions
        .iter()
        .map(|action| action.label.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"Inspect"));
    assert!(labels.contains(&"Maintain"));
    assert!(labels.contains(&"Improve"));
    assert!(labels.contains(&"Access"));
    assert!(labels.contains(&"Store 1"));
    assert_eq!(
        super::property::touch_command(
            &projection,
            macroquad_toolkit::grid::TilePos::new(3, 8),
            macroquad_toolkit::grid::TilePos::new(4, 8),
        )
        .as_deref(),
        Some("foundation-property:inspect:personal-property-1")
    );
}

#[test]
fn journey_guidance_names_the_durable_return_goal() {
    let mut projection = tarrowyn_protocol::FoundationJourneyProjection {
        contract: Default::default(),
        progress: tarrowyn_protocol::FoundationJourneyProgress {
            future_goal_state: tarrowyn_protocol::FoundationJourneyFutureGoalState::Active,
            ..Default::default()
        },
        completed_milestones: 12,
        total_milestones: 12,
        next_milestone: None,
        next_action: "Come back when the replanted crop is mature.".to_owned(),
    };

    let guidance = journey_guidance(Some(&projection)).unwrap();
    assert!(guidance.contains("RETURN GOAL"));
    assert!(guidance.contains("Return for the next harvest"));
    assert!(guidance.contains("Come back when the replanted crop is mature"));

    projection.progress.future_goal_state =
        tarrowyn_protocol::FoundationJourneyFutureGoalState::Complete;
    assert!(journey_guidance(Some(&projection))
        .unwrap()
        .contains("Return goal complete"));
    assert_eq!(journey_guidance(None), None);
}

fn work_credit(
    account_id: &str,
    kind: FoundationCooperationWorkKind,
    material_kind: FoundationForgeMaterialKind,
    amount: u32,
) -> FoundationCooperationWorkCredit {
    FoundationCooperationWorkCredit {
        account_id: account_id.to_owned(),
        kind,
        materials: vec![FoundationForgeMaterialAmount {
            kind: material_kind,
            amount,
        }],
        tick: 1,
    }
}

fn remote(account_id: &str, name: &str) -> crate::network::RemotePlayer {
    crate::network::RemotePlayer {
        account_id: account_id.to_owned(),
        character_id: format!("character-{account_id}"),
        display_name: name.to_owned(),
        position: TilePos::new(8, 6),
        last_seen_tick: 1,
        online: true,
    }
}

#[test]
fn credited_miner_gets_a_touch_first_two_ore_offer() {
    let mut activity = FoundationActivityState::default();
    activity.cooperation.recent_work.push(work_credit(
        "miner",
        FoundationCooperationWorkKind::Mine,
        FoundationForgeMaterialKind::IronOre,
        2,
    ));
    let inventory = tarrowyn_protocol::Inventory {
        iron_ore: 2,
        ..Default::default()
    };
    let choice = nearby_cooperation_choice(
        &activity,
        Some(&inventory),
        Some("miner"),
        &[remote("miner", "Miner"), remote("smith", "Smith")],
        &[],
        1,
    )
    .expect("eligible offer");

    assert_eq!(choice.label, "Offer 2 ore");
    assert_eq!(choice.command, "cooperation-offer-ore:smith");
    assert!(choice.detail.contains("5 actions together vs 6 solo"));
}

#[test]
fn credited_logger_gets_the_exact_incoming_ore_acceptance() {
    let mut activity = FoundationActivityState::default();
    activity.cooperation.recent_work.extend([
        work_credit(
            "miner",
            FoundationCooperationWorkKind::Mine,
            FoundationForgeMaterialKind::IronOre,
            2,
        ),
        work_credit(
            "smith",
            FoundationCooperationWorkKind::Log,
            FoundationForgeMaterialKind::Timber,
            2,
        ),
    ]);
    let inventory = tarrowyn_protocol::Inventory {
        timber: 2,
        ..Default::default()
    };
    let trade = TradeOffer {
        trade_id: "trade-7".to_owned(),
        creator_account_id: "miner".to_owned(),
        creator_name: "Miner".to_owned(),
        recipient_account_id: "smith".to_owned(),
        recipient_name: "Smith".to_owned(),
        offer: TradeBundle {
            iron_ore: 2,
            ..Default::default()
        },
        request: TradeBundle::default(),
        status: TradeStatus::Pending,
        created_tick: 1,
        expires_tick: 20,
    };
    let choice = nearby_cooperation_choice(
        &activity,
        Some(&inventory),
        Some("smith"),
        &[remote("miner", "Miner"), remote("smith", "Smith")],
        &[trade],
        1,
    )
    .expect("eligible acceptance");

    assert_eq!(choice.label, "Accept 2 ore");
    assert_eq!(choice.command, "cooperation-accept-ore:trade-7");
    assert!(choice.detail.contains("charcoal, a handle, and the tool"));
}

#[test]
fn uncredited_goods_do_not_claim_the_measured_cooperation_shortcut() {
    let inventory = tarrowyn_protocol::Inventory {
        iron_ore: 2,
        ..Default::default()
    };
    assert!(nearby_cooperation_choice(
        &FoundationActivityState::default(),
        Some(&inventory),
        Some("miner"),
        &[remote("smith", "Smith")],
        &[],
        1,
    )
    .is_none());
    assert!(
        cooperation_detail(&FoundationActivityState::default(), Some("miner"))
            .contains("Solo fallback open")
    );
}

fn storehouse_context<'a>(
    landmark: &'a FoundationLandmark,
    interaction_action: &'a str,
) -> FoundationContext<'a> {
    FoundationContext {
        landmark,
        interaction_id: "inspect-storehouse-site",
        interaction_action,
        action_label: "Inspect site".to_owned(),
        resource_node_id: None,
        resource_action: None,
        cache_action: None,
        cache_resource: None,
    }
}

fn storehouse_landmark() -> FoundationLandmark {
    FoundationLandmark {
        id: "storehouse-site".to_owned(),
        kind: "construction_space".to_owned(),
        name: "Storehouse site".to_owned(),
        position: Position { x: 6, y: 7 },
        visible: true,
        permanent: false,
        note: "The first public project".to_owned(),
    }
}

#[test]
fn nearby_storehouse_offers_goods_and_exact_gold_as_separate_touch_actions() {
    let landmark = storehouse_landmark();
    let context = storehouse_context(&landmark, "inspect_or_contribute");
    let inventory = Inventory {
        timber: 2,
        ..Default::default()
    };
    let choice = storehouse::nearby_choice(
        Some(&context),
        &tarrowyn_protocol::FoundationStorehouseState::default(),
        Some(&inventory),
        Some(3),
    )
    .unwrap();

    assert_eq!(choice.material.as_ref().unwrap().label, "Give 1 timber");
    assert_eq!(
        choice.material.as_ref().unwrap().command,
        "foundation-storehouse:storehouse-site:material:timber:1"
    );
    assert_eq!(choice.gold.as_ref().unwrap().label, "Fund stone 3g");
    assert_eq!(
        choice.gold.as_ref().unwrap().command,
        "foundation-storehouse:storehouse-site:gold:stone:3"
    );
    assert!(choice.detail.contains("Timber 0/8; stone 0/6"));
    assert!(choice.detail.contains("0 contributors"));
}

#[test]
fn storehouse_touch_state_explains_recovery_and_persistent_operation() {
    let landmark = storehouse_landmark();
    let context = storehouse_context(&landmark, "inspect_or_contribute");
    let mut project = tarrowyn_protocol::FoundationStorehouseState::default();
    let recovery = storehouse::nearby_choice(
        Some(&context),
        &project,
        Some(&Inventory::default()),
        Some(0),
    )
    .unwrap();
    assert!(recovery.material.is_none());
    assert!(!recovery.gold.as_ref().unwrap().enabled);
    assert!(recovery.detail.contains("Gather timber at the woodland"));

    project.current_stage = FoundationStorehouseStage::Operational;
    project
        .contributions
        .push(FoundationStorehouseContribution {
            contribution_id: "storehouse-contribution-2".to_owned(),
            account_id: "builder-account".to_owned(),
            input: FoundationStorehouseContributionInput::Material {
                kind: FoundationResourceKind::Timber,
                amount: 8,
            },
            credited_kind: FoundationResourceKind::Timber,
            credited_units: 8,
            contributed_tick: 1,
        });
    project.completion = Some(FoundationStorehouseCompletion {
        completed_tick: 2,
        contributor_account_ids: vec!["builder-account".to_owned()],
        operational_infrastructure_id: "first-beacon-storehouse".to_owned(),
    });
    let complete = storehouse::nearby_choice(Some(&context), &project, None, None).unwrap();
    assert!(complete.operational);
    assert_eq!(
        complete.inspect_command,
        "foundation-storehouse:storehouse-site:inspect"
    );
    assert!(complete.detail.contains("Operational storehouse"));
    assert!(complete.detail.contains("1 contributors"));
}

fn farm_world(crop: Option<CropState>) -> WorldState {
    let mut tiles = FlatGrid::new(4, 4, TileKind::Meadow);
    let mut crops = FlatGrid::new(4, 4, None);
    let plot = TilePos::new(2, 2);
    tiles.set(plot, TileKind::Field);
    crops.set(plot, crop);
    WorldState {
        tiles,
        crops,
        reachable: Default::default(),
    }
}

#[test]
fn forge_choice_walks_the_touch_first_preparation_chain() {
    let inventory = tarrowyn_protocol::Inventory {
        timber: 2,
        iron_ore: 2,
        ..Default::default()
    };
    let charcoal = nearby_forge_choice(
        Some(&inventory),
        Some(FoundationFieldToolKind::Crude),
        Some(2),
    );
    assert_eq!(charcoal.action, FoundationForgeAction::BurnCharcoal);
    assert_eq!(charcoal.label, "Burn charcoal");
    assert!(charcoal.detail.contains("2 timber, 2 ore"));
    assert!(charcoal
        .detail
        .contains("missing 0 ore, 1 charcoal, 1 handle"));

    let inventory = tarrowyn_protocol::Inventory {
        timber: 1,
        iron_ore: 2,
        charcoal: 1,
        ..Default::default()
    };
    let handle = nearby_forge_choice(Some(&inventory), None, Some(1));
    assert_eq!(handle.action, FoundationForgeAction::ShapeHandle);
    assert_eq!(handle.label, "Shape tool handle");
}

#[test]
fn forge_choice_exposes_ready_recipe_and_improved_capacity() {
    let ready = tarrowyn_protocol::Inventory {
        iron_ore: 2,
        charcoal: 1,
        tool_handles: 1,
        ..Default::default()
    };
    let choice = nearby_forge_choice(Some(&ready), Some(FoundationFieldToolKind::Crude), Some(1));
    assert_eq!(choice.action, FoundationForgeAction::ForgeFieldTool);
    assert_eq!(
        foundation_forge_command(choice.action),
        "foundation-forge:forge-field-tool"
    );

    let iron = nearby_forge_choice(
        Some(&Inventory::default()),
        Some(FoundationFieldToolKind::Iron),
        Some(6),
    );
    assert_eq!(iron.action, FoundationForgeAction::Inspect);
    assert!(iron.detail.contains("iron field tool 6/6"));
    assert!(iron.detail.contains("ready for 6 field actions"));
}

#[test]
fn nearby_empty_plot_exposes_touch_first_planting() {
    let choice = nearby_farm_choice(
        &farm_world(None),
        TilePos::new(2, 1),
        Some(2),
        Some(FieldWeather::Clear),
        Some(0),
    )
    .expect("nearby plot");

    assert_eq!(choice.action, FarmingAction::Plant);
    assert_eq!(choice.label, "Plant crop");
    assert!(choice.detail.contains("uses one seed"));
}

#[test]
fn growing_crop_explains_optional_maintenance_and_conditions() {
    let choice = nearby_farm_choice(
        &farm_world(Some(CropState {
            kind: CropKind::Turnip,
            stage: 2,
        })),
        TilePos::new(2, 2),
        Some(1),
        Some(FieldWeather::HeavyRain),
        Some(2),
    )
    .expect("nearby crop");

    assert_eq!(choice.action, FarmingAction::Tend);
    assert_eq!(choice.label, "Tend / water");
    assert!(choice.detail.contains("stage 2/3"));
    assert!(choice.detail.contains("optional"));
    assert!(choice.detail.contains("Tool 1/3"));
    assert!(choice.detail.contains("heavy rain"));
    assert!(choice.detail.contains("pests 2/2"));
}

#[test]
fn mature_crop_exposes_touch_first_harvest() {
    let choice = nearby_farm_choice(
        &farm_world(Some(CropState {
            kind: CropKind::Moonberry,
            stage: CropState::MATURE_STAGE,
        })),
        TilePos::new(1, 2),
        None,
        None,
        None,
    )
    .expect("nearby crop");

    assert_eq!(choice.action, FarmingAction::Harvest);
    assert_eq!(choice.label, "Harvest crop");
    assert!(choice.detail.contains("Moonberry is ready"));
}

fn baseline() -> FoundationBaseline {
    FoundationBaseline {
        fixture_id: "first-beacon-baseline-v1".to_owned(),
        schema_version: 1,
        settlement_id: "hearth-settlement".to_owned(),
        landmarks: vec![
            FoundationLandmark {
                id: "first-beacon".to_owned(),
                kind: "beacon".to_owned(),
                name: "First Beacon".to_owned(),
                position: Position { x: 8, y: 6 },
                visible: true,
                permanent: true,
                note: "Arrival".to_owned(),
            },
            FoundationLandmark {
                id: "builder-mara".to_owned(),
                kind: "npc".to_owned(),
                name: "Mara the Builder".to_owned(),
                position: Position { x: 7, y: 5 },
                visible: true,
                permanent: true,
                note: "Builder".to_owned(),
            },
        ],
        interactions: vec![
            FoundationInteraction {
                id: "arrive-first-beacon".to_owned(),
                landmark_id: "first-beacon".to_owned(),
                action: "arrive_or_travel".to_owned(),
                authority: "server".to_owned(),
                note: String::new(),
            },
            FoundationInteraction {
                id: "speak-with-builder".to_owned(),
                landmark_id: "builder-mara".to_owned(),
                action: "speak_or_request_construction".to_owned(),
                authority: "server".to_owned(),
                note: String::new(),
            },
        ],
    }
}

#[test]
fn exact_landmark_wins_over_an_adjacent_landmark() {
    let fixture = baseline();
    let activity = FoundationActivityState::default();
    let context =
        nearby_context(&fixture, &activity, TilePos::new(7, 5), None).expect("nearby context");

    assert_eq!(context.landmark.id, "builder-mara");
    assert_eq!(context.action_label, "Talk to Mara");
}

#[test]
fn context_requires_visible_adjacent_landmark() {
    let fixture = baseline();

    assert!(nearby_context(
        &fixture,
        &FoundationActivityState::default(),
        TilePos::new(2, 2),
        None,
    )
    .is_none());
}

#[test]
fn nearby_woodland_becomes_a_productive_resource_command() {
    let mut fixture = baseline();
    fixture.landmarks.push(FoundationLandmark {
        id: "whisperwood-edge".to_owned(),
        kind: "woodland".to_owned(),
        name: "Whisperwood edge".to_owned(),
        position: Position { x: 13, y: 3 },
        visible: true,
        permanent: false,
        note: "Nearby timber".to_owned(),
    });
    fixture.interactions.push(FoundationInteraction {
        id: "work-whisperwood-edge".to_owned(),
        landmark_id: "whisperwood-edge".to_owned(),
        action: "log".to_owned(),
        authority: "server".to_owned(),
        note: String::new(),
    });
    let activity = FoundationActivityState {
        resource_nodes: vec![FoundationResourceNode {
            node_id: "whisperwood-edge-node".to_owned(),
            landmark_id: "whisperwood-edge".to_owned(),
            deposits: vec![FoundationResourceDeposit {
                kind: FoundationResourceKind::Timber,
                remaining: 12,
                capacity: 12,
            }],
            recovery_interval_ticks: 6,
            last_recovered_tick: 0,
        }],
        crude_tool_access: Vec::new(),
        shared_cache: tarrowyn_protocol::FoundationSharedCache::default(),
        cooperation: Default::default(),
        storehouse: Default::default(),
    };

    let context = nearby_context(&fixture, &activity, TilePos::new(12, 3), None).unwrap();

    assert_eq!(context.action_label, "Gather timber");
    assert_eq!(context.resource_node_id, Some("whisperwood-edge-node"));
    assert_eq!(context.resource_action, Some(FoundationResourceAction::Log));
}

#[test]
fn nearby_cache_prefers_a_visible_store_action_for_carried_materials() {
    let (fixture, activity) = cache_fixture(tarrowyn_protocol::Inventory::default());
    let inventory = tarrowyn_protocol::Inventory {
        stone: 2,
        ..Default::default()
    };

    let context =
        nearby_context(&fixture, &activity, TilePos::new(9, 6), Some(&inventory)).unwrap();

    assert_eq!(context.action_label, "Store stone");
    assert_eq!(context.cache_action, Some(FoundationCacheAction::Deposit));
    assert_eq!(context.cache_resource, Some(FoundationResourceKind::Stone));
}

#[test]
fn nearby_cache_exposes_collection_when_the_player_carries_no_materials() {
    let cache_inventory = tarrowyn_protocol::Inventory {
        iron_ore: 1,
        ..Default::default()
    };
    let (fixture, activity) = cache_fixture(cache_inventory);
    let inventory = tarrowyn_protocol::Inventory::default();

    let context =
        nearby_context(&fixture, &activity, TilePos::new(9, 6), Some(&inventory)).unwrap();

    assert_eq!(context.action_label, "Collect iron ore");
    assert_eq!(context.cache_action, Some(FoundationCacheAction::Withdraw));
    assert_eq!(
        context.cache_resource,
        Some(FoundationResourceKind::IronOre)
    );
}

fn cache_fixture(
    inventory: tarrowyn_protocol::Inventory,
) -> (FoundationBaseline, FoundationActivityState) {
    let mut fixture = baseline();
    fixture.landmarks.push(FoundationLandmark {
        id: "first-beacon-cache".to_owned(),
        kind: "cache".to_owned(),
        name: "Shared cache".to_owned(),
        position: Position { x: 9, y: 6 },
        visible: true,
        permanent: true,
        note: "Shared materials".to_owned(),
    });
    fixture.interactions.push(FoundationInteraction {
        id: "use-shared-cache".to_owned(),
        landmark_id: "first-beacon-cache".to_owned(),
        action: "deposit_or_collect".to_owned(),
        authority: "server".to_owned(),
        note: String::new(),
    });
    let activity = FoundationActivityState {
        shared_cache: tarrowyn_protocol::FoundationSharedCache {
            landmark_id: "first-beacon-cache".to_owned(),
            inventory,
            capacity: 64,
        },
        cooperation: Default::default(),
        ..Default::default()
    };
    (fixture, activity)
}
