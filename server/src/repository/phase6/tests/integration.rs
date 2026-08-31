use super::super::super::models::{RepositoryState, StoredState};
use super::super::super::ServerConfig;
use super::super::super::WorldRepository;
use super::super::backup::write;
use std::fs;
use tarrowyn_protocol::{
    AccountDeletionRequest, AuthLinkRequest, ChatRequest, ClaimLifecycleAction,
    ClaimLifecycleRequest, CommodityKind, GovernanceAction, GovernanceRequest, GuestSessionRequest,
    MarketOrderAction, MarketOrderRequest, MarketOrderStatus, PublicAction, SupportRepairAction,
    SupportRepairRequest, TradeAction, TradeBundle, TradeRequest,
};

mod backup;
mod support_account;

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
    let market_order = repository
        .market_order(
            &token,
            MarketOrderRequest {
                request_id: "deletion-market-order".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("whisperwood-outpost".to_owned()),
                commodity: Some(CommodityKind::Seeds),
                quantity: Some(2),
            },
        )
        .unwrap()
        .data;
    let market_order_id = market_order.order.unwrap().order_id;
    let failed_market_order = repository
        .market_order(
            &token,
            MarketOrderRequest {
                request_id: "deletion-failed-market-order".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("whisperwood-outpost".to_owned()),
                commodity: Some(CommodityKind::Seeds),
                quantity: Some(2),
            },
        )
        .unwrap()
        .data;
    let failed_market_order_id = failed_market_order.order.unwrap().order_id;
    let origin_seed_stock_before = {
        let mut state = repository.state.lock().unwrap();
        state
            .phase5
            .market_orders
            .iter_mut()
            .find(|order| order.order_id == failed_market_order_id)
            .expect("failed market order remains recorded")
            .status = MarketOrderStatus::Failed;
        state.phase5.stock.get("hearth:seeds").copied().unwrap_or(0)
    };
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
        for index in 0..(super::super::super::phase3::MAX_CHRONICLE + 1) {
            super::super::super::phase3::record(
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
    let market_order = state
        .phase5
        .market_orders
        .iter()
        .find(|order| order.order_id == market_order_id)
        .expect("deleted account market order remains as history");
    assert_eq!(market_order.owner_account_id, "former-resident");
    assert_eq!(market_order.owner_name, "Former resident");
    assert_eq!(market_order.status, MarketOrderStatus::Cancelled);
    assert_eq!(market_order.settled_tick, Some(state.tick));
    let failed_market_order = state
        .phase5
        .market_orders
        .iter()
        .find(|order| order.order_id == failed_market_order_id)
        .expect("failed deleted account order remains as history");
    assert_eq!(failed_market_order.owner_account_id, "former-resident");
    assert_eq!(failed_market_order.owner_name, "Former resident");
    assert_eq!(failed_market_order.status, MarketOrderStatus::Cancelled);
    assert_eq!(failed_market_order.settled_tick, Some(state.tick));
    assert_eq!(
        state.phase5.stock.get("hearth:seeds").copied().unwrap_or(0),
        origin_seed_stock_before + 4
    );
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
fn account_deletion_anonymises_governance_replays_kept_by_another_identity() {
    let repository = WorldRepository::new(ServerConfig::default());
    let owner_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("governance-replay-owner".to_owned()),
            reset: false,
        })
        .expect("owner guest session")
        .data;
    let owner = repository
        .auth_link(
            &owner_guest.account_token,
            AuthLinkRequest {
                request_id: "governance-replay-owner-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "governance-replay-owner-subject".to_owned(),
                display_name: Some("Departing steward".to_owned()),
            },
        )
        .expect("owner link")
        .data;
    let observer = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("governance-replay-observer".to_owned()),
            reset: false,
        })
        .expect("observer guest session")
        .data;
    let claimed = repository
        .governance(
            &owner.session.account_token,
            GovernanceRequest {
                request_id: "governance-replay-claim".to_owned(),
                action: GovernanceAction::ClaimOffice,
                office_id: Some("steward".to_owned()),
                proposal_id: None,
                public_action: None,
                target: None,
                cost: None,
                tax_rate_percent: None,
            },
        )
        .expect("claim office")
        .data;
    assert!(claimed.accepted);
    let proposal_request = GovernanceRequest {
        request_id: "governance-replay-propose".to_owned(),
        action: GovernanceAction::Propose,
        office_id: None,
        proposal_id: None,
        public_action: Some(PublicAction::HostFestival),
        target: None,
        cost: Some(1),
        tax_rate_percent: None,
    };
    let proposed = repository
        .governance(&observer.account_token, proposal_request.clone())
        .expect("proposal")
        .data;
    assert!(proposed
        .governance
        .offices
        .iter()
        .any(|office| office.holder_account_id.as_deref() == Some(owner.account_id.as_str())));

    repository
        .account_delete(
            &owner.session.account_token,
            AccountDeletionRequest {
                request_id: "governance-replay-owner-delete".to_owned(),
                account_id: owner.account_id,
            },
        )
        .expect("schedule owner deletion");
    repository.tick();

    let replay = repository
        .governance(&observer.account_token, proposal_request)
        .expect("governance replay")
        .data;
    let steward = replay
        .governance
        .offices
        .iter()
        .find(|office| office.office_id == "steward")
        .expect("steward office");
    assert!(steward.holder_account_id.is_none());
    assert!(steward.holder_name.is_none());
    assert!(steward.vacant);
    assert!(repository.ops_health().data.ready);
}

#[test]
fn support_repairs_restore_claim_access_and_merge_household_history() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("repair-operator".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let requested = repository
        .claim_lifecycle(
            &operator.account_token,
            ClaimLifecycleRequest {
                request_id: "repair-claim-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    let claim_id = requested.claim.unwrap().claim_id;
    let approved = repository
        .claim_lifecycle(
            &operator.account_token,
            ClaimLifecycleRequest {
                request_id: "repair-claim-approve".to_owned(),
                action: ClaimLifecycleAction::Approve,
                claim_id: Some(claim_id.clone()),
                target_account_id: None,
            },
        )
        .unwrap()
        .data
        .claim
        .unwrap();
    let lease_expiry = approved.expires_at_unix_seconds;
    {
        let mut state = repository.state.lock().unwrap();
        let claim = state
            .phase4
            .claims
            .iter_mut()
            .find(|claim| claim.claim_id == claim_id)
            .unwrap();
        claim.building_access = false;
    }
    let restore_request = SupportRepairRequest {
        request_id: "repair-claim-access".to_owned(),
        action: SupportRepairAction::RestoreClaim,
        account_id: Some(operator.account_id.clone()),
        target_id: Some(claim_id.clone()),
        note: "Restore an active claim whose access flag was lost.".to_owned(),
    };
    let restored = repository
        .support_repair(&operator.account_token, restore_request.clone())
        .unwrap()
        .data;
    assert!(restored.accepted);
    {
        let state = repository.state.lock().unwrap();
        let claim = state
            .phase4
            .claims
            .iter()
            .find(|claim| claim.claim_id == claim_id)
            .unwrap();
        assert!(claim.building_access);
        assert_eq!(claim.expires_at_unix_seconds, lease_expiry);
    }
    assert_eq!(
        repository
            .support_repair(&operator.account_token, restore_request)
            .unwrap()
            .data,
        restored
    );

    let household_id = {
        let mut state = repository.state.lock().unwrap();
        let mut duplicate = state.phase5.households[0].clone();
        duplicate
            .history
            .push("Support preserved a duplicate's arrival note.".to_owned());
        let household_id = duplicate.household_id.clone();
        state.phase5.households.push(duplicate);
        household_id
    };
    let merge_request = SupportRepairRequest {
        request_id: "repair-household-merge".to_owned(),
        action: SupportRepairAction::MergeHousehold,
        account_id: None,
        target_id: Some(household_id.clone()),
        note: "Merge a duplicated regional household record.".to_owned(),
    };
    let merged = repository
        .support_repair(&operator.account_token, merge_request.clone())
        .unwrap()
        .data;
    assert!(merged.accepted);
    let state = repository.state.lock().unwrap();
    assert_eq!(
        state
            .phase5
            .households
            .iter()
            .filter(|household| household.household_id == household_id)
            .count(),
        1
    );
    assert!(state.phase5.households[0]
        .history
        .iter()
        .any(|entry| entry.contains("duplicate's arrival note")));
    drop(state);
    assert_eq!(
        repository
            .support_repair(&operator.account_token, merge_request)
            .unwrap()
            .data,
        merged
    );
}

#[test]
fn support_repair_restores_failed_market_escrow_once() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("repair-trade-operator".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let before_escrow = {
        let state = repository.state.lock().unwrap();
        state
            .identities
            .get("repair-trade-operator")
            .expect("operator identity exists")
            .inventory
            .seeds
    };
    let created = repository
        .market_order(
            &operator.account_token,
            MarketOrderRequest {
                request_id: "repair-trade-create".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("whisperwood-outpost".to_owned()),
                commodity: Some(CommodityKind::Seeds),
                quantity: Some(2),
            },
        )
        .unwrap()
        .data;
    let order_id = created.order.expect("market order created").order_id;
    {
        let mut state = repository.state.lock().unwrap();
        state
            .phase5
            .market_orders
            .iter_mut()
            .find(|order| order.order_id == order_id)
            .expect("market order remains recorded")
            .status = MarketOrderStatus::Failed;
    }

    let repair_request = SupportRepairRequest {
        request_id: "repair-failed-trade".to_owned(),
        action: SupportRepairAction::ReconcileTrade,
        account_id: Some(operator.account_id.clone()),
        target_id: Some(order_id.clone()),
        note: "Restore escrow from an expired failed shipment.".to_owned(),
    };
    let repaired = repository
        .support_repair(&operator.account_token, repair_request.clone())
        .unwrap()
        .data;
    assert!(repaired.accepted);
    {
        let state = repository.state.lock().unwrap();
        let identity = state
            .identities
            .get("repair-trade-operator")
            .expect("operator identity remains present");
        assert_eq!(identity.inventory.seeds, before_escrow);
        let order = state
            .phase5
            .market_orders
            .iter()
            .find(|order| order.order_id == order_id)
            .expect("repaired order remains recorded");
        assert_eq!(order.status, MarketOrderStatus::Cancelled);
        assert_eq!(order.settled_tick, Some(state.tick));
        assert_eq!(state.phase5.cursor, state.cursor);
    }
    assert_eq!(
        repository
            .support_repair(&operator.account_token, repair_request)
            .unwrap()
            .data,
        repaired
    );

    let second_attempt = repository
        .support_repair(
            &operator.account_token,
            SupportRepairRequest {
                request_id: "repair-failed-trade-second-attempt".to_owned(),
                action: SupportRepairAction::ReconcileTrade,
                account_id: Some(operator.account_id),
                target_id: Some(order_id),
                note: "Confirm a closed shipment cannot be refunded twice.".to_owned(),
            },
        )
        .unwrap()
        .data;
    assert!(!second_attempt.accepted);
    assert!(second_attempt
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("open or failed")));
    let state = repository.state.lock().unwrap();
    assert_eq!(
        state
            .identities
            .get("repair-trade-operator")
            .expect("operator identity remains present")
            .inventory
            .seeds,
        before_escrow
    );
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
                offer: Some(TradeBundle {
                    seeds: 1,
                    ..TradeBundle::default()
                }),
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
