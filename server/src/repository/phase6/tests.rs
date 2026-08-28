use super::super::models::{RepositoryState, StoredState};
use super::super::ServerConfig;
use super::super::WorldRepository;
use super::backup::write;
use std::fs;
use tarrowyn_protocol::{
    ChatRequest, ClaimLifecycleAction, ClaimLifecycleRequest, GovernanceAction, GovernanceRequest,
    GuestSessionRequest, MovementIntent, TradeAction, TradeBundle, TradeRequest,
};

#[test]
fn backup_replaces_the_snapshot_as_one_complete_json_file() {
    let root = std::env::temp_dir().join(format!(
        "tarrowyn-backup-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos()
    ));
    let path = root.join("nested").join("snapshot.json");
    let config = ServerConfig {
        backup_path: Some(path.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    };
    let mut state = RepositoryState::fresh(&config);
    state.tick = 7;

    assert!(write(&mut state, &config));

    let bytes = fs::read(&path).expect("backup should be written");
    let stored: StoredState = serde_json::from_slice(&bytes).expect("backup should be complete");
    assert_eq!(stored.storage_version, super::super::STORAGE_VERSION);
    assert_eq!(state.phase6.last_backup_tick, Some(7));
    assert_eq!(state.phase6.last_backup_path.as_deref(), path.to_str());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn failed_scheduled_backup_degrades_readiness_until_recovery() {
    let backup_path =
        std::env::temp_dir().join(format!("tarrowyn-backup-failure-{}", std::process::id()));
    fs::create_dir(&backup_path).expect("backup failure fixture should be creatable");
    let repository = WorldRepository::new(ServerConfig {
        backup_path: Some(backup_path.to_string_lossy().into_owned()),
        backup_interval_ticks: 1,
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("backup-failure-operator".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;

    repository.tick();

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert_eq!(health.status, "degraded");
    assert!(health.persistence_error.is_none());
    assert!(health
        .backup_error
        .as_deref()
        .is_some_and(|message| message.contains("scheduled backup failed")));
    let metrics = repository
        .ops_metrics(&operator.account_token)
        .unwrap()
        .data;
    assert!(metrics
        .alert_flags
        .iter()
        .any(|flag| flag == "backup_write_failed"));

    fs::remove_dir(&backup_path).expect("failed backup fixture should be removable");
    repository.tick();

    let recovered = repository.ops_health().data;
    assert!(recovered.ready);
    assert!(recovered.backup_error.is_none());
    assert_eq!(recovered.last_backup_tick, Some(2));
    let _ = fs::remove_file(backup_path);
}

#[test]
fn operational_metrics_require_a_configured_support_operator() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("metrics-operator".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let player = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("metrics-player".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;

    let error = repository
        .ops_metrics(&player.account_token)
        .expect_err("ordinary players must not read operational metrics");
    assert_eq!(error.status, 403);
    assert_eq!(error.error.code, "support_operator_required");
    assert!(repository.ops_metrics(&operator.account_token).is_ok());

    repository
        .movement(
            &player.account_token,
            MovementIntent {
                request_id: "metrics-accepted".to_owned(),
                dx: 0,
                dy: 1,
            },
        )
        .unwrap();
    repository
        .movement(
            &player.account_token,
            MovementIntent {
                request_id: "metrics-rejected".to_owned(),
                dx: 2,
                dy: 0,
            },
        )
        .unwrap();
    let metrics = repository
        .ops_metrics(&operator.account_token)
        .unwrap()
        .data;
    assert!(metrics.completed_commands >= 1);
    assert!(metrics.rejected_commands >= 1);
}

#[test]
fn player_social_economy_and_governance_commands_are_audited() {
    let repository = WorldRepository::new(ServerConfig::default());
    let actor = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("audit-actor".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let recipient = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("audit-recipient".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;

    repository
        .chat(
            &actor.account_token,
            ChatRequest {
                request_id: "audit-chat".to_owned(),
                channel: "settlement".to_owned(),
                text: "A useful meeting note.".to_owned(),
            },
        )
        .unwrap();
    repository
        .trade(
            &actor.account_token,
            TradeRequest {
                request_id: "audit-trade".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(recipient.account_id.clone()),
                offer: Some(TradeBundle::default()),
                request: Some(TradeBundle::default()),
            },
        )
        .unwrap();
    repository
        .claim_lifecycle(
            &actor.account_token,
            ClaimLifecycleRequest {
                request_id: "audit-claim".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap();
    repository
        .governance(
            &actor.account_token,
            GovernanceRequest {
                request_id: "audit-governance".to_owned(),
                action: GovernanceAction::ClaimOffice,
                office_id: Some("steward".to_owned()),
                proposal_id: None,
                public_action: None,
                target: None,
                cost: None,
                tax_rate_percent: None,
            },
        )
        .unwrap();

    let state = repository.state.lock().unwrap();
    let actions: Vec<_> = state
        .phase6
        .audits
        .iter()
        .map(|record| record.action.as_str())
        .collect();
    assert!(actions.contains(&"chat.send"));
    assert!(actions.contains(&"trade.create"));
    assert!(actions.contains(&"claim.lifecycle"));
    assert!(actions.contains(&"governance.action"));
}
