use super::*;
use crate::config::ServerConfig;
use tarrowyn_protocol::{
    ChatRequest, ClaimAction, ClaimRequest, ClaimStatus, CombatAction, CombatRequest,
    ContractAction, ContractRequest, CropKind, CropState, ExpeditionAction, ExpeditionRequest,
    ExpeditionRole, FarmingAction, FarmingRequest, GuestSessionRequest, HouseholdStatus,
    MovementIntent, Position, SupportRepairAction, SupportRepairRequest, TileKind, TradeAction,
    TradeBundle, TradeRequest, TradeStatus, WeaponKind, WorldEvent,
};

mod chat_validation;
mod core_metadata_integrity;
mod core_replay_integrity;
mod events;
mod input_bounds;
mod integrity;
mod market_integrity;
mod movement_validation;
mod numeric_boundaries;
mod persistence;
mod phase3;
mod phase3_replay_integrity;
mod phase3_state_integrity;
mod phase4_replay_integrity;
mod phase4_state_integrity;
mod phase5_metadata_integrity;
mod phase5_replay_integrity;
mod regional_record_integrity;
mod request_validation;
mod reset;
mod settlement_integrity;
mod telemetry;
mod trade_retention;
mod trade_validation;

fn repo() -> WorldRepository {
    WorldRepository::new(ServerConfig {
        session_ttl_seconds: 5,
        ..ServerConfig::default()
    })
}

fn guest(repo: &WorldRepository, key: &str) -> GuestSessionResponse {
    repo.guest_session(GuestSessionRequest {
        client_key: Some(key.to_owned()),
        reset: false,
    })
    .expect("guest session")
    .data
}

#[test]
fn guest_identity_uses_the_shared_starting_skill() {
    let repository = repo();
    let session = guest(&repository, "shared-starting-skill");
    let player = repository.inventory(&session.account_token).unwrap().data;

    assert_eq!(player.skill, crate::content::starting_skill());
    assert_eq!(
        player.inventory.seeds,
        ServerConfig::default().starting_seeds
    );
}

#[test]
fn guest_sessions_are_distinct_but_resume_by_client_key() {
    let repo = repo();
    let first = guest(&repo, "one");
    let second = guest(&repo, "two");
    let resumed = guest(&repo, "one");
    assert_ne!(first.character_id, second.character_id);
    assert_eq!(first.character_id, resumed.character_id);
    assert_ne!(first.account_token, resumed.account_token);
}

#[test]
fn movement_is_server_authoritative_and_rejects_water() {
    let repo = repo();
    let session = guest(&repo, "walker");
    let accepted = repo
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "valid".to_owned(),
                dx: 0,
                dy: 1,
            },
        )
        .unwrap()
        .data;
    assert!(accepted.accepted);
    assert_eq!(accepted.position, Position { x: 8, y: 7 });

    repo.tick();
    let rejected = repo
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "invalid".to_owned(),
                dx: 8,
                dy: 0,
            },
        )
        .unwrap()
        .data;
    assert!(!rejected.accepted);
    assert_eq!(rejected.position, Position { x: 8, y: 7 });
}

#[test]
fn chat_events_are_ordered_and_clock_ticks_once_for_the_world() {
    let repo = repo();
    let one = guest(&repo, "one");
    let two = guest(&repo, "two");
    let before = repo.world(&one.account_token).unwrap().data.cursor;
    repo.chat(
        &one.account_token,
        ChatRequest {
            request_id: "chat-one".to_owned(),
            channel: "settlement".to_owned(),
            text: "Hello from one".to_owned(),
        },
    )
    .unwrap();
    repo.chat(
        &two.account_token,
        ChatRequest {
            request_id: "chat-two".to_owned(),
            channel: "settlement".to_owned(),
            text: "Hello from two".to_owned(),
        },
    )
    .unwrap();
    let events = repo.events(&one.account_token, before).unwrap().data.events;
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|record| match &record.event {
            WorldEvent::Chat(message) => Some(message.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, ["Hello from one", "Hello from two"]);

    let day_before = repo.world(&one.account_token).unwrap().data.clock.seconds;
    repo.tick();
    let day_after = repo.world(&one.account_token).unwrap().data.clock.seconds;
    assert_eq!(
        day_after - day_before,
        ServerConfig::default().world_seconds_per_tick
    );
    assert_eq!(repo.server_tick(), 1);
    assert_eq!(
        repo.world(&two.account_token).unwrap().data.players.len(),
        2
    );
}

#[test]
fn world_contains_the_phase_zero_collision_map() {
    let repo = repo();
    let session = guest(&repo, "map");
    let world = repo.world(&session.account_token).unwrap().data;
    assert_eq!(world.width, 18);
    assert_eq!(world.height, 11);
    assert!(world.tiles.iter().any(|tile| tile.kind == TileKind::Water));
    assert!(world.tiles.iter().any(|tile| tile.kind == TileKind::Field));
}

#[test]
fn fresh_world_farm_plots_follow_the_validated_region_manifest() {
    let repository = repo();
    let session = guest(&repository, "manifest-farm-plots");
    let world = repository.world(&session.account_token).unwrap().data;
    let expected = crate::content::farm_plot_positions();

    assert_eq!(
        world
            .plots
            .iter()
            .map(|plot| plot.position)
            .collect::<Vec<_>>(),
        expected
    );
    for position in expected {
        assert_eq!(
            world
                .tiles
                .iter()
                .find(|tile| tile.position == position)
                .map(|tile| tile.kind),
            Some(TileKind::Field)
        );
    }
}

#[test]
fn empty_legacy_farm_layout_upgrades_without_moving_crop_state() {
    let legacy = [(3, 4), (3, 5), (4, 4), (4, 5), (5, 4), (5, 5)]
        .into_iter()
        .map(|(x, y)| tarrowyn_protocol::FarmPlot {
            position: Position { x, y },
            crop: None,
        })
        .collect();

    let restored = super::world::restore_plots(legacy);

    assert_eq!(
        restored
            .iter()
            .map(|plot| plot.position)
            .collect::<Vec<_>>(),
        crate::content::farm_plot_positions()
    );
    assert!(restored.iter().all(|plot| plot.crop.is_none()));
}

#[test]
fn populated_legacy_farm_layout_remains_unchanged() {
    let stored = [(3, 4), (3, 5), (4, 4), (4, 5), (5, 4), (5, 5)]
        .into_iter()
        .enumerate()
        .map(|(index, (x, y))| tarrowyn_protocol::FarmPlot {
            position: Position { x, y },
            crop: (index == 0).then_some(CropState {
                kind: CropKind::Wheat,
                stage: 1,
                quality: 2,
                planted_tick: 4,
                last_tended_tick: None,
            }),
        })
        .collect::<Vec<_>>();

    assert_eq!(super::world::restore_plots(stored.clone()), stored);
}

#[test]
fn movement_rate_limit_chat_bound_and_session_expiry_are_server_rules() {
    let repo = repo();
    let session = guest(&repo, "rules");
    let first = repo
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "step-one".to_owned(),
                dx: 1,
                dy: 0,
            },
        )
        .unwrap()
        .data;
    assert!(first.accepted);
    let too_fast = repo
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "step-two".to_owned(),
                dx: 1,
                dy: 0,
            },
        )
        .unwrap()
        .data;
    assert!(!too_fast.accepted);

    repo.tick();
    let too_long = repo
        .chat(
            &session.account_token,
            ChatRequest {
                request_id: "too-long".to_owned(),
                channel: "settlement".to_owned(),
                text: "x".repeat(161),
            },
        )
        .unwrap()
        .data;
    assert!(!too_long.accepted);

    for _ in 0..21 {
        repo.tick();
    }
    assert!(repo.world(&session.account_token).is_err());
}

#[test]
fn support_repairs_are_operator_only_and_replay_safe() {
    let player_repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let player = guest(&player_repository, "support-player");
    let denied = player_repository
        .support_repair(
            &player.account_token,
            SupportRepairRequest {
                request_id: "player-repair".to_owned(),
                action: SupportRepairAction::NormalizeInventory,
                account_id: None,
                target_id: None,
                note: "player should not repair state".to_owned(),
            },
        )
        .unwrap_err();
    assert_eq!(denied.status, 403);

    let operator_repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = guest(&operator_repository, "support-operator");
    let request = SupportRepairRequest {
        request_id: "operator-repair".to_owned(),
        action: SupportRepairAction::NormalizeInventory,
        account_id: None,
        target_id: None,
        note: "normalise the operator fixture".to_owned(),
    };
    let repaired = operator_repository
        .support_repair(&operator.account_token, request.clone())
        .unwrap()
        .data;
    let replay = operator_repository
        .support_repair(&operator.account_token, request)
        .unwrap()
        .data;
    assert!(repaired.accepted);
    assert_eq!(replay, repaired);
}

#[test]
fn farming_grows_on_the_shared_clock_and_retries_are_idempotent() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        world_seconds_per_tick: 10.0,
        crop_stage_seconds: 10.0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "farmer");
    let plot_position = crate::content::farm_plot_positions()[2];
    for (index, (dx, dy)) in [(1, 0), (1, 0), (0, 1), (0, 1)].into_iter().enumerate() {
        repo.movement(
            &session.account_token,
            MovementIntent {
                request_id: format!("farm-step-{index}"),
                dx,
                dy,
            },
        )
        .unwrap();
    }
    let request = FarmingRequest {
        request_id: "plant-once".to_owned(),
        action: FarmingAction::Plant,
        position: plot_position,
    };
    let planted = repo
        .farming(&session.account_token, request.clone())
        .unwrap()
        .data;
    let retry = repo.farming(&session.account_token, request).unwrap().data;
    assert!(planted.accepted);
    assert_eq!(retry, planted);
    for _ in 0..3 {
        repo.tick();
    }
    let grown = repo.world(&session.account_token).unwrap().data;
    assert_eq!(
        grown
            .plots
            .iter()
            .find(|plot| plot.position == plot_position)
            .unwrap()
            .crop
            .unwrap()
            .stage,
        3
    );
    let harvested = repo
        .farming(
            &session.account_token,
            FarmingRequest {
                request_id: "harvest-once".to_owned(),
                action: FarmingAction::Harvest,
                position: plot_position,
            },
        )
        .unwrap()
        .data;
    assert!(harvested.accepted);
    let inventory = repo
        .inventory(&session.account_token)
        .unwrap()
        .data
        .inventory;
    assert_eq!(
        inventory.wheat + inventory.turnips + inventory.moonberries,
        1
    );
}

#[test]
fn trade_review_and_accept_exchange_goods_once() {
    let repo = WorldRepository::new(ServerConfig::default());
    let one = guest(&repo, "trader-one");
    let two = guest(&repo, "trader-two");
    let create = TradeRequest {
        request_id: "offer-seed".to_owned(),
        action: TradeAction::Create,
        trade_id: None,
        recipient_account_id: Some(two.account_id.clone()),
        offer: Some(TradeBundle {
            seeds: 1,
            ..TradeBundle::default()
        }),
        request: Some(TradeBundle {
            gold: 2,
            ..TradeBundle::default()
        }),
    };
    let created = repo.trade(&one.account_token, create.clone()).unwrap().data;
    assert!(created.accepted);
    assert_eq!(
        repo.trade(&one.account_token, create).unwrap().data,
        created
    );
    let trade_id = created.trade.as_ref().unwrap().trade_id.clone();
    let reviewed = repo
        .trade(
            &two.account_token,
            TradeRequest {
                request_id: "review-offer".to_owned(),
                action: TradeAction::Review,
                trade_id: Some(trade_id.clone()),
                recipient_account_id: None,
                offer: None,
                request: None,
            },
        )
        .unwrap()
        .data;
    assert!(reviewed.accepted);
    let accept = TradeRequest {
        request_id: "accept-offer".to_owned(),
        action: TradeAction::Accept,
        trade_id: Some(trade_id),
        recipient_account_id: None,
        offer: None,
        request: None,
    };
    assert!(
        repo.trade(&two.account_token, accept.clone())
            .unwrap()
            .data
            .accepted
    );
    assert_eq!(
        repo.trade(&two.account_token, accept)
            .unwrap()
            .data
            .trade
            .unwrap()
            .status,
        TradeStatus::Accepted
    );
    let one_inventory = repo.inventory(&one.account_token).unwrap().data;
    let two_inventory = repo.inventory(&two.account_token).unwrap().data;
    assert_eq!(one_inventory.inventory.seeds, 5);
    assert_eq!(
        one_inventory.gold,
        ServerConfig::default().starting_gold + 2
    );
    assert_eq!(
        two_inventory.inventory.seeds,
        ServerConfig::default().starting_seeds + 1
    );
    assert_eq!(
        two_inventory.gold,
        ServerConfig::default().starting_gold - 2
    );
}

#[test]
fn repository_restart_restores_clock_identity_and_tavern_history() {
    let path = std::env::temp_dir().join(format!("tarrowyn-phase2-{}.json", std::process::id()));
    let path_string = path.to_string_lossy().into_owned();
    let config = ServerConfig {
        persistence_path: Some(path_string.clone()),
        ..ServerConfig::default()
    };
    let first = WorldRepository::new(config.clone());
    let session = guest(&first, "returning-player");
    first.tick();
    first
        .chat(
            &session.account_token,
            ChatRequest {
                request_id: "restart-chat".to_owned(),
                channel: "tavern".to_owned(),
                text: "I will return.".to_owned(),
            },
        )
        .unwrap();
    let tick = first.server_tick();
    drop(first);
    let second = WorldRepository::new(config);
    let resumed = guest(&second, "returning-player");
    assert_eq!(resumed.character_id, session.character_id);
    assert_eq!(second.server_tick(), tick);
    let feed = second.tavern_feed(&resumed.account_token).unwrap().data;
    assert!(feed
        .chat
        .iter()
        .any(|message| message.text == "I will return."));
    let _ = std::fs::remove_file(path);
}

#[test]
fn phase_two_state_without_frontier_fields_loads_safe_phase_three_defaults() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-phase2-migration-{}.json",
        std::process::id()
    ));
    let path_string = path.to_string_lossy().into_owned();
    let config = ServerConfig {
        persistence_path: Some(path_string.clone()),
        ..ServerConfig::default()
    };
    let first = WorldRepository::new(config.clone());
    let original = guest(&first, "legacy-settlement");
    first.tick();
    drop(first);

    let bytes = std::fs::read(&path).unwrap();
    let mut document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    document["storage_version"] = serde_json::json!(1);
    document.as_object_mut().unwrap().remove("phase3");
    std::fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();

    let migrated = WorldRepository::new(config);
    let resumed = guest(&migrated, "legacy-settlement");
    assert_eq!(resumed.character_id, original.character_id);
    let world = migrated.world(&resumed.account_token).unwrap().data;
    assert!(world.wilderness.unwrap().threat_active);
    assert!(migrated
        .opportunities(&resumed.account_token)
        .unwrap()
        .data
        .opportunities
        .iter()
        .any(|opportunity| opportunity.status == HouseholdStatus::Travelling));

    let _ = std::fs::remove_file(path);
}
