use super::super::models::{RepositoryState, StoredState};
use super::super::ServerConfig;
use super::super::WorldRepository;
use super::backup::write;
use std::fs;
use tarrowyn_protocol::{
    AccountDeletionRequest, AuthLinkRequest, ChatRequest, ClaimLifecycleAction,
    ClaimLifecycleRequest, GovernanceAction, GovernanceRequest, GuestSessionRequest,
    MovementIntent, TradeAction, TradeBundle, TradeRequest,
};

mod long_session;

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
fn account_deletion_removes_private_state_and_anonymizes_public_history() {
    let state_path = std::env::temp_dir().join(format!(
        "tarrowyn-account-deletion-{}.json",
        std::process::id()
    ));
    let config = ServerConfig {
        persistence_path: Some(state_path.to_string_lossy().into_owned()),
        backup_path: None,
        ..ServerConfig::default()
    };
    let repository = WorldRepository::new(config.clone());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("deletion-client".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "deletion-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "deletion-subject".to_owned(),
                display_name: Some("Leaving traveller".to_owned()),
            },
        )
        .unwrap()
        .data;
    let token = linked.session.account_token.clone();
    let account_id = linked.account_id.clone();
    repository
        .chat(
            &token,
            ChatRequest {
                request_id: "deletion-chat".to_owned(),
                channel: "settlement".to_owned(),
                text: "Please keep the hall open.".to_owned(),
            },
        )
        .unwrap();
    repository
        .claim_lifecycle(
            &token,
            ClaimLifecycleRequest {
                request_id: "deletion-claim".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap();
    let identity_key = {
        let mut state = repository.state.lock().unwrap();
        let key = state
            .phase6
            .accounts
            .get(&account_id)
            .unwrap()
            .identity_key
            .clone();
        state.phase4.profiles.insert(key.clone(), Vec::new());
        state.phase4.governance.offices[0].holder_account_id = Some(account_id.clone());
        state.phase4.governance.offices[0].holder_name = Some("Leaving traveller".to_owned());
        key
    };
    {
        let mut state = repository.state.lock().unwrap();
        for index in 0..(super::super::phase3::MAX_CHRONICLE + 1) {
            super::super::phase3::record(
                &mut state,
                "named achievement",
                &format!("Leaving traveller achievement {index}"),
                "Leaving traveller helped keep the hall open.",
            );
        }
    }

    let request = AccountDeletionRequest {
        request_id: "deletion-request".to_owned(),
        account_id: account_id.clone(),
    };
    let scheduled = repository
        .account_delete(&token, request.clone())
        .unwrap()
        .data;
    assert!(scheduled.accepted);
    assert_eq!(scheduled.status, "scheduled");
    assert_eq!(
        repository.account_delete(&token, request).unwrap().data,
        scheduled
    );

    drop(repository);
    let repository = WorldRepository::new(config);
    repository.tick();

    assert!(repository.account(&token).is_err());
    let state = repository.state.lock().unwrap();
    assert!(!state.identities.contains_key(&identity_key));
    assert!(state.phase6.accounts.is_empty());
    assert!(state.phase6.sessions.is_empty());
    assert!(!state.phase4.profiles.contains_key(&identity_key));
    assert!(state.phase4.governance.offices[0].vacant);
    assert!(state.phase4.claims[0].owner_account_id.is_none());
    assert!(state
        .chat_history
        .iter()
        .any(|message| message.account_id == "former-resident"
            && message.text.contains("removed after account deletion")));
    assert!(state.events.iter().any(|event| matches!(
        &event.event,
        tarrowyn_protocol::WorldEvent::Chat(message)
            if message.account_id == "former-resident"
    )));
    assert!(state
        .phase3
        .chronicle
        .iter()
        .chain(state.phase3.chronicle_archive.iter())
        .all(|entry| !entry.text.contains("Leaving traveller")
            && !entry.title.contains("Leaving traveller")));
    assert!(state.events.iter().all(|event| {
        !matches!(
            &event.event,
            tarrowyn_protocol::WorldEvent::Chronicle(entry)
                if entry.text.contains("Leaving traveller")
                    || entry.title.contains("Leaving traveller")
        )
    }));
    assert!(state
        .phase6
        .audits
        .iter()
        .any(|record| record.action == "account.delete.completed"));
    assert!(state
        .phase6
        .audits
        .iter()
        .all(|record| record.actor_account_id != account_id && record.target != account_id));
    drop(state);
    let _ = std::fs::remove_file(state_path);
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
    assert!(metrics.average_price_index_percent > 0);
    assert!(metrics.scarce_goods_count > 0);
    assert!(metrics.npc_fallback_households > 0);
    assert_eq!(metrics.abandoned_claims, 0);
    assert!(metrics.declining_settlements > 0);
    assert!(metrics.newcomer_access);
}

#[test]
fn support_account_view_is_operator_only_and_secret_free() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("support-view-operator".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let target = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("support-view-target".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    repository
        .chat(
            &target.account_token,
            ChatRequest {
                request_id: "support-view-chat".to_owned(),
                channel: "settlement".to_owned(),
                text: "A public history note.".to_owned(),
            },
        )
        .unwrap();
    repository
        .claim_lifecycle(
            &target.account_token,
            ClaimLifecycleRequest {
                request_id: "support-view-claim".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap();
    repository
        .trade(
            &target.account_token,
            TradeRequest {
                request_id: "support-view-trade".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(operator.account_id.clone()),
                offer: Some(TradeBundle::default()),
                request: Some(TradeBundle::default()),
            },
        )
        .unwrap();

    let view = repository
        .support_account(&operator.account_token, &target.account_id)
        .unwrap()
        .data;
    assert_eq!(view.account.account_id, target.account_id);
    assert_eq!(view.account.character_id, target.character_id);
    assert!(view.account.guest_fixture);
    assert_eq!(view.claims.len(), 1);
    assert_eq!(view.trades.len(), 1);
    assert!(!view.chronicle.is_empty());
    assert!(view.event_cursor > 0);

    let forbidden = repository
        .support_account(&target.account_token, &target.account_id)
        .expect_err("ordinary players must not read support account views");
    assert_eq!(forbidden.status, 403);
    assert_eq!(forbidden.error.code, "support_operator_required");
    let missing = repository
        .support_account(&operator.account_token, "missing-account")
        .expect_err("unknown accounts should not produce an empty support view");
    assert_eq!(missing.status, 404);
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
