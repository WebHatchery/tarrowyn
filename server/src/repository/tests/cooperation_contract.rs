use super::*;
use crate::repository::models::{RepositoryState, StoredState};
use tarrowyn_protocol::{
    FoundationCooperationContribution, FoundationCooperationResult, FoundationCooperationState,
    FoundationForgeMaterialAmount, FoundationForgeMaterialKind,
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
    assert_eq!(restored.to_stored().storage_version, 24);
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
