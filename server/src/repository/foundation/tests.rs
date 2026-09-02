use super::*;
use crate::config::ServerConfig;
use crate::repository::models::{RepositoryState, StoredState};
use tarrowyn_protocol::{
    FoundationCacheAction, FoundationCacheRequest, FoundationResourceAction,
    FoundationResourceKind, FoundationResourceRequest, GuestSessionRequest,
};

#[test]
fn fresh_state_exposes_shared_crude_tools_and_bounded_nodes() {
    let activity = fresh();

    assert_eq!(activity.resource_nodes.len(), 2);
    assert!(activity.resource_nodes.iter().all(|node| {
        node.recovery_interval_ticks > 0
            && node
                .deposits
                .iter()
                .all(|deposit| deposit.remaining == deposit.capacity && deposit.capacity > 0)
    }));
    assert_eq!(activity.crude_tool_access.len(), 1);
    assert!(activity.crude_tool_access[0].available_to_all);
}

#[test]
fn depleted_resources_recover_deterministically_without_exceeding_capacity() {
    let mut state = RepositoryState::fresh(&ServerConfig::default());
    let timber = &mut state.foundation_activity.resource_nodes[0].deposits[0];
    timber.remaining = 2;
    state.tick = 18;

    recover_resource_nodes(&mut state);

    let node = &state.foundation_activity.resource_nodes[0];
    assert_eq!(node.deposits[0].remaining, 5);
    assert_eq!(node.last_recovered_tick, 18);

    state.tick = 600;
    recover_resource_nodes(&mut state);
    assert_eq!(
        state.foundation_activity.resource_nodes[0].deposits[0].remaining,
        12
    );
}

#[test]
fn foundation_activity_survives_the_stored_state_boundary() {
    let config = ServerConfig::default();
    let mut state = RepositoryState::fresh(&config);
    state.foundation_activity.resource_nodes[1].deposits[0].remaining = 3;
    state.foundation_activity.resource_nodes[1].last_recovered_tick = 12;

    let encoded = serde_json::to_vec(&state.to_stored()).unwrap();
    let stored: StoredState = serde_json::from_slice(&encoded).unwrap();
    let restored = RepositoryState::from_stored(stored, &config);

    assert_eq!(restored.foundation_activity, state.foundation_activity);
}

#[test]
fn logging_is_proximity_checked_and_replay_safe() {
    let repository = super::super::WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("f2-logging".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let request = FoundationResourceRequest {
        request_id: "log-once".to_owned(),
        node_id: "whisperwood-edge-node".to_owned(),
        action: FoundationResourceAction::Log,
    };
    let too_far = repository
        .foundation_resource(&session.account_token, request.clone())
        .unwrap()
        .data;
    assert!(!too_far.accepted);
    assert_eq!(too_far.player.inventory.timber, 0);

    let request = FoundationResourceRequest {
        request_id: "log-nearby".to_owned(),
        ..request
    };
    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("f2-logging")
        .unwrap()
        .position = tarrowyn_protocol::Position { x: 12, y: 3 };
    let first = repository
        .foundation_resource(&session.account_token, request.clone())
        .unwrap()
        .data;
    let replay = repository
        .foundation_resource(&session.account_token, request)
        .unwrap()
        .data;

    assert!(first.accepted);
    assert_eq!(first.player.inventory.timber, 2);
    assert_eq!(replay, first);
    assert_eq!(
        repository
            .inventory(&session.account_token)
            .unwrap()
            .data
            .inventory
            .timber,
        2
    );
}

#[test]
fn mining_yields_bounded_stone_and_ore_then_recovers_on_ticks() {
    let repository = super::super::WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("f2-mining".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("f2-mining")
        .unwrap()
        .position = tarrowyn_protocol::Position { x: 10, y: 4 };

    let first = repository
        .foundation_resource(
            &session.account_token,
            FoundationResourceRequest {
                request_id: "mine-first".to_owned(),
                node_id: "shallow-stone-seam-node".to_owned(),
                action: FoundationResourceAction::Mine,
            },
        )
        .unwrap()
        .data;

    assert!(first.accepted);
    assert_eq!(first.player.inventory.stone, 2);
    assert_eq!(first.player.inventory.iron_ore, 1);
    assert_eq!(first.node.deposits[0].remaining, 9);
    assert_eq!(first.node.deposits[1].remaining, 3);

    for _ in 0..6 {
        repository.tick();
    }
    let state = repository.state.lock().unwrap();
    assert_eq!(
        state.foundation_activity.resource_nodes[1].deposits[0].remaining,
        10
    );
    assert_eq!(
        state.foundation_activity.resource_nodes[1].deposits[1].remaining,
        4
    );
}

#[test]
fn gathering_depletion_and_replay_survive_repository_restart() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-foundation-resource-{}.json",
        std::process::id()
    ));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        backup_path: None,
        ..ServerConfig::default()
    };
    let first_repository = super::super::WorldRepository::new(config.clone());
    let first_session = first_repository
        .guest_session(GuestSessionRequest {
            client_key: Some("f2-resource-restart".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    first_repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("f2-resource-restart")
        .unwrap()
        .position = tarrowyn_protocol::Position { x: 12, y: 3 };
    let request = FoundationResourceRequest {
        request_id: "restart-safe-log".to_owned(),
        node_id: "whisperwood-edge-node".to_owned(),
        action: FoundationResourceAction::Log,
    };
    let original = first_repository
        .foundation_resource(&first_session.account_token, request.clone())
        .unwrap()
        .data;
    assert_eq!(original.node.deposits[0].remaining, 11);
    drop(first_repository);

    let second_repository = super::super::WorldRepository::new(config);
    let resumed = second_repository
        .guest_session(GuestSessionRequest {
            client_key: Some("f2-resource-restart".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let replay = second_repository
        .foundation_resource(&resumed.account_token, request)
        .unwrap()
        .data;

    assert_eq!(replay, original);
    let state = second_repository.state.lock().unwrap();
    assert_eq!(
        state.foundation_activity.resource_nodes[0].deposits[0].remaining,
        11
    );
    assert_eq!(
        state
            .identities
            .get("f2-resource-restart")
            .unwrap()
            .inventory
            .timber,
        2
    );
    drop(state);
    drop(second_repository);
    let _ = std::fs::remove_file(path);
}

#[test]
fn shared_cache_transfer_and_replay_survive_repository_restart() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-foundation-cache-{}.json",
        std::process::id()
    ));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        backup_path: None,
        ..ServerConfig::default()
    };
    let first_repository = super::super::WorldRepository::new(config.clone());
    let first_session = first_repository
        .guest_session(GuestSessionRequest {
            client_key: Some("f2-cache-restart".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    first_repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("f2-cache-restart")
        .unwrap()
        .inventory
        .timber = 3;
    let deposit = FoundationCacheRequest {
        request_id: "cache-deposit".to_owned(),
        action: FoundationCacheAction::Deposit,
        resource: Some(FoundationResourceKind::Timber),
        amount: 2,
    };
    let stored = first_repository
        .foundation_cache(&first_session.account_token, deposit.clone())
        .unwrap()
        .data;
    assert!(stored.accepted);
    assert_eq!(stored.player.inventory.timber, 1);
    assert_eq!(stored.cache.inventory.timber, 2);
    drop(first_repository);

    let second_repository = super::super::WorldRepository::new(config);
    let resumed = second_repository
        .guest_session(GuestSessionRequest {
            client_key: Some("f2-cache-restart".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let replay = second_repository
        .foundation_cache(&resumed.account_token, deposit)
        .unwrap()
        .data;
    assert_eq!(replay, stored);

    let withdrawn = second_repository
        .foundation_cache(
            &resumed.account_token,
            FoundationCacheRequest {
                request_id: "cache-withdraw".to_owned(),
                action: FoundationCacheAction::Withdraw,
                resource: Some(FoundationResourceKind::Timber),
                amount: 1,
            },
        )
        .unwrap()
        .data;
    assert!(withdrawn.accepted);
    assert_eq!(withdrawn.player.inventory.timber, 2);
    assert_eq!(withdrawn.cache.inventory.timber, 1);
    drop(second_repository);
    let _ = std::fs::remove_file(path);
}

#[test]
fn shared_cache_rejects_over_capacity_and_unowned_goods_atomically() {
    let repository = super::super::WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("f2-cache-capacity".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    {
        let mut state = repository.state.lock().unwrap();
        state
            .identities
            .get_mut("f2-cache-capacity")
            .unwrap()
            .inventory
            .stone = 2;
        state.foundation_activity.shared_cache.inventory.timber = 64;
    }
    let rejected = repository
        .foundation_cache(
            &session.account_token,
            FoundationCacheRequest {
                request_id: "cache-over-capacity".to_owned(),
                action: FoundationCacheAction::Deposit,
                resource: Some(FoundationResourceKind::Stone),
                amount: 1,
            },
        )
        .unwrap()
        .data;

    assert!(!rejected.accepted);
    assert_eq!(rejected.player.inventory.stone, 2);
    assert_eq!(rejected.cache.inventory.timber, 64);
    assert_eq!(rejected.cache.inventory.stone, 0);

    let unavailable = repository
        .foundation_cache(
            &session.account_token,
            FoundationCacheRequest {
                request_id: "cache-unavailable".to_owned(),
                action: FoundationCacheAction::Withdraw,
                resource: Some(FoundationResourceKind::IronOre),
                amount: 1,
            },
        )
        .unwrap()
        .data;
    assert!(!unavailable.accepted);
    assert_eq!(unavailable.player.inventory.iron_ore, 0);
    assert_eq!(unavailable.cache.inventory.iron_ore, 0);
}
