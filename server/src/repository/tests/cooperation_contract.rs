use super::*;
use crate::repository::models::{RepositoryState, StoredState};
use tarrowyn_protocol::{
    FoundationCooperationContribution, FoundationCooperationResult, FoundationCooperationState,
    FoundationForgeAction, FoundationForgeMaterialAmount, FoundationForgeMaterialKind,
    FoundationForgeRequest, FoundationResourceAction, FoundationResourceRequest, Position,
    SkillAction, SkillRequest,
};

#[test]
fn storage_version_twenty_three_defaults_the_fixed_cooperation_goal() {
    let repository = repo();
    let stored = repository.state.lock().unwrap().to_stored();
    let mut json = serde_json::to_value(stored).unwrap();
    json["storage_version"] = serde_json::json!(23);
    json["foundation_activity"]
        .as_object_mut()
        .unwrap()
        .remove("cooperation");

    let legacy: StoredState = serde_json::from_value(json).unwrap();
    let restored = RepositoryState::from_stored_at(legacy, &ServerConfig::default(), 0);

    assert_eq!(
        restored.foundation_activity.cooperation,
        FoundationCooperationState::default()
    );
    assert_eq!(restored.to_stored().storage_version, 26);
}

#[test]
fn storage_version_twenty_four_defaults_cooperation_work_tracking() {
    let repository = repo();
    let stored = repository.state.lock().unwrap().to_stored();
    let mut json = serde_json::to_value(stored).unwrap();
    json["storage_version"] = serde_json::json!(24);
    let cooperation = json["foundation_activity"]["cooperation"]
        .as_object_mut()
        .unwrap();
    cooperation.remove("recent_work");
    cooperation.remove("active_attempts");

    let legacy: StoredState = serde_json::from_value(json).unwrap();
    let restored = RepositoryState::from_stored_at(legacy, &ServerConfig::default(), 0);

    assert!(restored
        .foundation_activity
        .cooperation
        .recent_work
        .is_empty());
    assert!(restored
        .foundation_activity
        .cooperation
        .active_attempts
        .is_empty());
    assert_eq!(restored.to_stored().storage_version, 26);
}

#[test]
fn foundational_material_trade_is_atomic_replay_safe_and_persistent() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-cooperation-contract-{}.json",
        std::process::id()
    ));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    };
    let repository = WorldRepository::new(config.clone());
    let logger = guest(&repository, "contract-logger");
    let miner = guest(&repository, "contract-miner");
    {
        let mut state = repository.state.lock().unwrap();
        state
            .identities
            .get_mut("contract-logger")
            .unwrap()
            .inventory
            .timber = 2;
        state
            .identities
            .get_mut("contract-miner")
            .unwrap()
            .inventory
            .iron_ore = 2;
    }
    let create = repository
        .trade(
            &logger.account_token,
            TradeRequest {
                request_id: "contract-material-offer".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(miner.account_id.clone()),
                offer: Some(TradeBundle {
                    timber: 2,
                    ..Default::default()
                }),
                request: Some(TradeBundle {
                    iron_ore: 2,
                    ..Default::default()
                }),
            },
        )
        .unwrap()
        .data;
    let trade_id = create.trade.unwrap().trade_id;
    let accept_request = TradeRequest {
        request_id: "contract-material-accept".to_owned(),
        action: TradeAction::Accept,
        trade_id: Some(trade_id),
        recipient_account_id: None,
        offer: None,
        request: None,
    };
    let accepted = repository
        .trade(&miner.account_token, accept_request.clone())
        .unwrap()
        .data;
    let replay = repository
        .trade(&miner.account_token, accept_request)
        .unwrap()
        .data;
    assert_eq!(replay, accepted);

    let restarted = WorldRepository::new(config);
    let logger = guest(&restarted, "contract-logger");
    let miner = guest(&restarted, "contract-miner");
    assert_eq!(
        restarted
            .inventory(&logger.account_token)
            .unwrap()
            .data
            .inventory
            .iron_ore,
        2
    );
    assert_eq!(
        restarted
            .inventory(&miner.account_token)
            .unwrap()
            .data
            .inventory
            .timber,
        2
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_well_formed_cooperation_result_survives_the_repository_boundary() {
    let repository = repo();
    let coordinator = guest(&repository, "contract-coordinator");
    let helper = guest(&repository, "contract-helper");
    {
        let mut state = repository.state.lock().unwrap();
        state.foundation_activity.cooperation.latest_result = Some(FoundationCooperationResult {
            coordinator_account_id: coordinator.account_id.clone(),
            participant_account_ids: vec![
                coordinator.account_id.clone(),
                helper.account_id.clone(),
            ],
            contributions: vec![
                FoundationCooperationContribution {
                    account_id: coordinator.account_id,
                    materials: vec![FoundationForgeMaterialAmount {
                        kind: FoundationForgeMaterialKind::IronOre,
                        amount: 2,
                    }],
                    work_actions: 2,
                },
                FoundationCooperationContribution {
                    account_id: helper.account_id,
                    materials: vec![FoundationForgeMaterialAmount {
                        kind: FoundationForgeMaterialKind::Timber,
                        amount: 2,
                    }],
                    work_actions: 3,
                },
            ],
            trade_id: "trade-1".to_owned(),
            work_actions: 5,
            saved_work_actions: 1,
            completed_tick: state.tick,
        });
    }

    assert!(repository.ops_health().data.integrity_ok);
    let stored = repository.state.lock().unwrap().to_stored();
    let restored = RepositoryState::from_stored(stored, &ServerConfig::default());
    assert_eq!(
        restored.foundation_activity.cooperation,
        repository
            .state
            .lock()
            .unwrap()
            .foundation_activity
            .cooperation
    );
}

#[test]
fn uncommitted_player_retains_the_six_action_crude_self_supply_fallback() {
    let repository = repo();
    let solo = guest(&repository, "cooperation-solo");
    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("cooperation-solo")
        .unwrap()
        .position = Position { x: 10, y: 4 };
    for index in 0..2 {
        let mined = repository
            .foundation_resource(
                &solo.account_token,
                FoundationResourceRequest {
                    request_id: format!("solo-mine-{index}"),
                    node_id: "shallow-stone-seam-node".to_owned(),
                    action: FoundationResourceAction::Mine,
                },
            )
            .unwrap()
            .data;
        assert!(mined.accepted);
    }
    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("cooperation-solo")
        .unwrap()
        .position = Position { x: 12, y: 3 };
    assert!(
        repository
            .foundation_resource(
                &solo.account_token,
                FoundationResourceRequest {
                    request_id: "solo-log".to_owned(),
                    node_id: "whisperwood-edge-node".to_owned(),
                    action: FoundationResourceAction::Log,
                },
            )
            .unwrap()
            .data
            .accepted
    );
    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("cooperation-solo")
        .unwrap()
        .position = Position { x: 10, y: 5 };
    for (request_id, action) in [
        ("solo-charcoal", FoundationForgeAction::BurnCharcoal),
        ("solo-handle", FoundationForgeAction::ShapeHandle),
        ("solo-tool", FoundationForgeAction::ForgeFieldTool),
    ] {
        assert!(
            repository
                .foundation_forge(
                    &solo.account_token,
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
    let state = repository.state.lock().unwrap();
    assert_eq!(state.foundation_activity.cooperation.recent_work.len(), 6);
    assert!(state
        .foundation_activity
        .cooperation
        .latest_result
        .is_none());
    assert_eq!(
        state
            .identities
            .get("cooperation-solo")
            .unwrap()
            .field_tool_kind,
        tarrowyn_protocol::FoundationFieldToolKind::Iron
    );
}

#[test]
fn voluntary_mining_practice_and_atomic_barter_save_one_measured_work_action() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-cooperation-result-{}-{}.json",
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
    let miner = guest(&repository, "cooperation-miner");
    let smith = guest(&repository, "cooperation-smith");

    for index in 0..4 {
        let practiced = repository
            .practice_skill(
                &miner.account_token,
                SkillRequest {
                    request_id: format!("commit-mining-{index}"),
                    action: SkillAction::Practice,
                    lesson_id: None,
                    skill_id: Some("mining".to_owned()),
                    target_account_id: None,
                },
            )
            .unwrap()
            .data;
        assert!(practiced.accepted);
    }
    {
        let mut state = repository.state.lock().unwrap();
        state
            .identities
            .get_mut("cooperation-miner")
            .unwrap()
            .position = Position { x: 10, y: 4 };
        state
            .identities
            .get_mut("cooperation-smith")
            .unwrap()
            .position = Position { x: 12, y: 3 };
    }
    let mined = repository
        .foundation_resource(
            &miner.account_token,
            FoundationResourceRequest {
                request_id: "efficient-ore".to_owned(),
                node_id: "shallow-stone-seam-node".to_owned(),
                action: FoundationResourceAction::Mine,
            },
        )
        .unwrap()
        .data;
    assert!(mined.accepted);
    assert_eq!(mined.player.inventory.iron_ore, 2);
    let logged = repository
        .foundation_resource(
            &smith.account_token,
            FoundationResourceRequest {
                request_id: "goal-timber".to_owned(),
                node_id: "whisperwood-edge-node".to_owned(),
                action: FoundationResourceAction::Log,
            },
        )
        .unwrap()
        .data;
    assert!(logged.accepted);
    assert_eq!(logged.player.inventory.timber, 2);

    let created = repository
        .trade(
            &miner.account_token,
            TradeRequest {
                request_id: "offer-efficient-ore".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(smith.account_id.clone()),
                offer: Some(TradeBundle {
                    iron_ore: 2,
                    ..Default::default()
                }),
                request: Some(TradeBundle::default()),
            },
        )
        .unwrap()
        .data;
    let trade_id = created.trade.unwrap().trade_id;
    let accept = TradeRequest {
        request_id: "accept-efficient-ore".to_owned(),
        action: TradeAction::Accept,
        trade_id: Some(trade_id.clone()),
        recipient_account_id: None,
        offer: None,
        request: None,
    };
    let accepted = repository
        .trade(&smith.account_token, accept.clone())
        .unwrap()
        .data;
    assert!(accepted.accepted);
    assert_eq!(accepted.trade.as_ref().unwrap().trade_id, trade_id);
    assert_eq!(
        repository
            .inventory(&smith.account_token)
            .unwrap()
            .data
            .inventory
            .iron_ore,
        2
    );
    assert_eq!(
        repository
            .state
            .lock()
            .unwrap()
            .foundation_activity
            .cooperation
            .active_attempts
            .len(),
        1
    );

    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("cooperation-smith")
        .unwrap()
        .position = Position { x: 10, y: 5 };
    for (request_id, action) in [
        ("goal-charcoal", FoundationForgeAction::BurnCharcoal),
        ("goal-handle", FoundationForgeAction::ShapeHandle),
        ("goal-tool", FoundationForgeAction::ForgeFieldTool),
    ] {
        assert!(
            repository
                .foundation_forge(
                    &smith.account_token,
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
    let result = repository
        .state
        .lock()
        .unwrap()
        .foundation_activity
        .cooperation
        .latest_result
        .clone()
        .unwrap();
    assert_eq!(result.trade_id, trade_id);
    assert_eq!(result.work_actions, 5);
    assert_eq!(result.saved_work_actions, 1);
    assert_eq!(result.participant_account_ids.len(), 2);
    assert!(repository.ops_health().data.integrity_ok);

    let replayed_accept = repository.trade(&smith.account_token, accept).unwrap().data;
    assert_eq!(replayed_accept, accepted);
    drop(repository);

    let restarted = WorldRepository::new(config);
    let resumed_smith = guest(&restarted, "cooperation-smith");
    let persisted = restarted
        .state
        .lock()
        .unwrap()
        .foundation_activity
        .cooperation
        .latest_result
        .clone()
        .unwrap();
    assert_eq!(persisted, result);
    let replayed_forge = restarted
        .foundation_forge(
            &resumed_smith.account_token,
            FoundationForgeRequest {
                request_id: "goal-tool".to_owned(),
                action: FoundationForgeAction::ForgeFieldTool,
            },
        )
        .unwrap()
        .data;
    assert!(replayed_forge.accepted);
    assert_eq!(
        restarted
            .state
            .lock()
            .unwrap()
            .foundation_activity
            .cooperation
            .latest_result,
        Some(result)
    );
    drop(restarted);
    let _ = std::fs::remove_file(path);
}
