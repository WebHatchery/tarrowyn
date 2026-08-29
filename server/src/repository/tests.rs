use super::*;
use crate::config::ServerConfig;
use tarrowyn_protocol::{
    ChatRequest, ClaimAction, ClaimRequest, ClaimStatus, CombatAction, CombatRequest,
    ContractAction, ContractRequest, ExpeditionAction, ExpeditionRequest, ExpeditionRole,
    FarmingAction, FarmingRequest, GuestSessionRequest, HouseholdStatus, MovementIntent, Position,
    SupportRepairAction, SupportRepairRequest, TileKind, TradeAction, TradeBundle, TradeRequest,
    TradeStatus, WeaponKind, WorldEvent,
};

mod chat_validation;
mod events;
mod movement_validation;
mod persistence;
mod request_validation;
mod reset;
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
    for (index, (dx, dy)) in [(-1, 0), (-1, 0), (-1, 0), (-1, 0), (0, -1)]
        .into_iter()
        .enumerate()
    {
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
        position: Position { x: 4, y: 4 },
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
            .find(|plot| plot.position == Position { x: 4, y: 4 })
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
                position: Position { x: 4, y: 4 },
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
fn phase_three_contract_combat_recovery_and_chronicle_are_authoritative() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "frontier-player");
    let accept = repo
        .contract(
            &session.account_token,
            ContractRequest {
                request_id: "contract-accept".to_owned(),
                action: ContractAction::Accept,
                contract_id: "brambleback-watch".to_owned(),
            },
        )
        .unwrap()
        .data;
    assert!(accept.accepted);
    for (index, (dx, dy)) in [(1, 0), (1, 0), (1, 0), (1, 0), (0, -1), (0, -1)]
        .into_iter()
        .enumerate()
    {
        assert!(
            repo.movement(
                &session.account_token,
                MovementIntent {
                    request_id: format!("frontier-step-{index}"),
                    dx,
                    dy,
                },
            )
            .unwrap()
            .data
            .accepted
        );
    }
    for index in 0..3 {
        assert!(
            repo.contract(
                &session.account_token,
                ContractRequest {
                    request_id: format!("contract-progress-{index}"),
                    action: ContractAction::Progress,
                    contract_id: "brambleback-watch".to_owned(),
                },
            )
            .unwrap()
            .data
            .accepted
        );
    }
    let report = repo
        .contract(
            &session.account_token,
            ContractRequest {
                request_id: "contract-report".to_owned(),
                action: ContractAction::Report,
                contract_id: "brambleback-watch".to_owned(),
            },
        )
        .unwrap()
        .data;
    assert!(report.accepted);

    let seeds_before = repo
        .inventory(&session.account_token)
        .unwrap()
        .data
        .inventory
        .seeds;
    let knockout = repo
        .combat(
            &session.account_token,
            CombatRequest {
                request_id: "club-strike".to_owned(),
                action: CombatAction::Strike,
                weapon: WeaponKind::ImprovisedClub,
            },
        )
        .unwrap()
        .data;
    assert_eq!(
        knockout.outcome,
        Some(tarrowyn_protocol::CombatOutcome::KnockedOut)
    );
    assert!(knockout.player.knocked_out);
    assert_eq!(
        repo.inventory(&session.account_token)
            .unwrap()
            .data
            .inventory
            .seeds,
        seeds_before - 1
    );
    assert_eq!(
        repo.combat(
            &session.account_token,
            CombatRequest {
                request_id: "club-strike".to_owned(),
                action: CombatAction::Strike,
                weapon: WeaponKind::ImprovisedClub,
            },
        )
        .unwrap()
        .data,
        knockout
    );
    let recovery = repo
        .recovery(
            &session.account_token,
            tarrowyn_protocol::RecoveryRequest {
                request_id: "rescued".to_owned(),
                choice: tarrowyn_protocol::RecoveryChoice::AskRescuer,
            },
        )
        .unwrap()
        .data;
    assert!(recovery.accepted);
    assert!(!recovery.player.knocked_out);
    let chronicle = repo.chronicle(&session.account_token, 0).unwrap().data;
    assert!(chronicle
        .entries
        .iter()
        .any(|entry| entry.kind == "knockout"));
    assert!(repo
        .events(&session.account_token, 0)
        .unwrap()
        .data
        .events
        .iter()
        .any(|event| matches!(event.event, WorldEvent::Chronicle(_))));
}

#[test]
fn phase_three_household_and_claim_lifecycles_emit_recovery_events() {
    let repo = WorldRepository::new(ServerConfig {
        claim_reclaim_ticks: 2,
        session_ttl_seconds: 100,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "frontier-lifecycle");
    let claim = repo
        .claim(
            &session.account_token,
            ClaimRequest {
                request_id: "lifecycle-claim".to_owned(),
                action: ClaimAction::Request,
            },
        )
        .unwrap()
        .data;
    assert!(claim.accepted);

    for _ in 0..17 {
        repo.tick();
    }

    let opportunities = repo.opportunities(&session.account_token).unwrap().data;
    assert_eq!(
        opportunities.opportunities[0].status,
        HouseholdStatus::Departed
    );
    let inspected = repo
        .claim(
            &session.account_token,
            ClaimRequest {
                request_id: "lifecycle-inspect".to_owned(),
                action: ClaimAction::Inspect,
            },
        )
        .unwrap()
        .data;
    assert_eq!(inspected.claim.unwrap().status, ClaimStatus::Reclaimed);

    let chronicle = repo.chronicle(&session.account_token, 0).unwrap().data;
    let kinds: Vec<&str> = chronicle
        .entries
        .iter()
        .map(|entry| entry.kind.as_str())
        .collect();
    assert!(kinds.contains(&"arrival candidate"));
    assert!(kinds.contains(&"household arrival"));
    assert!(kinds.contains(&"household departure"));
    assert!(kinds.contains(&"claim reclaimed"));
}

#[test]
fn phase_three_claim_and_expedition_survive_as_durable_world_state() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        claim_reclaim_ticks: 2,
        ..ServerConfig::default()
    });
    let one = guest(&repo, "pioneer-one");
    let two = guest(&repo, "pioneer-two");
    let three = guest(&repo, "pioneer-three");
    assert!(
        repo.claim(
            &one.account_token,
            ClaimRequest {
                request_id: "claim".to_owned(),
                action: ClaimAction::Request,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    let announce = repo
        .expedition(
            &one.account_token,
            ExpeditionRequest {
                request_id: "announce".to_owned(),
                action: ExpeditionAction::Announce,
                expedition_id: None,
                role: Some(ExpeditionRole::Scout),
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: Some("Test Rest".to_owned()),
            },
        )
        .unwrap()
        .data;
    assert!(announce.accepted);
    for (session, role, id) in [
        (&two, ExpeditionRole::Farmer, "join-farmer"),
        (&three, ExpeditionRole::Builder, "join-builder"),
    ] {
        assert!(
            repo.expedition(
                &session.account_token,
                ExpeditionRequest {
                    request_id: id.to_owned(),
                    action: ExpeditionAction::Join,
                    expedition_id: Some("pioneer-1".to_owned()),
                    role: Some(role),
                    food: 0,
                    tools: 0,
                    materials: 0,
                    safety: 0,
                    outpost_name: None,
                },
            )
            .unwrap()
            .data
            .accepted
        );
    }
    assert!(
        repo.expedition(
            &one.account_token,
            ExpeditionRequest {
                request_id: "supply".to_owned(),
                action: ExpeditionAction::Supply,
                expedition_id: Some("pioneer-1".to_owned()),
                role: None,
                food: 6,
                tools: 3,
                materials: 8,
                safety: 3,
                outpost_name: None,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    assert!(
        repo.expedition(
            &one.account_token,
            ExpeditionRequest {
                request_id: "launch".to_owned(),
                action: ExpeditionAction::Launch,
                expedition_id: Some("pioneer-1".to_owned()),
                role: None,
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    let resolved = repo
        .expedition(
            &one.account_token,
            ExpeditionRequest {
                request_id: "resolve".to_owned(),
                action: ExpeditionAction::Resolve,
                expedition_id: Some("pioneer-1".to_owned()),
                role: None,
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .unwrap()
        .data;
    assert!(resolved.accepted);
    assert_eq!(
        resolved.expedition.unwrap().status,
        tarrowyn_protocol::ExpeditionStatus::Succeeded
    );
    assert!(repo
        .world(&two.account_token)
        .unwrap()
        .data
        .outpost
        .is_some());
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
