use super::*;
use tarrowyn_protocol::{
    AccountDeletionRequest, AuthLinkRequest, AuthRefreshRequest, ChatRequest, ClaimLifecycleAction,
    ClaimLifecycleRequest, GuestSessionRequest, KnowledgeAction, KnowledgeRequest,
    MarketOrderAction, MarketOrderRequest, ModerationReportRequest, ProfessionAction,
    ProfessionKind, ProfessionRequest, SkillAction, SkillRequest, WorldEvent,
};

mod settlement_history;

#[test]
fn account_deletion_records_departure_for_connected_observers() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("deletion-presence-guest".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "deletion-presence-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "deletion-presence-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("linked session")
        .data;
    let cursor_before_delete = repository
        .world(&linked.session.account_token)
        .expect("world before deletion")
        .meta
        .cursor
        .expect("world cursor");

    let deletion = repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "deletion-presence-delete".to_owned(),
                account_id: linked.account_id.clone(),
            },
        )
        .expect("schedule deletion")
        .data;
    assert!(deletion.accepted);
    repository.tick();

    let state = repository.state.lock().expect("state lock");
    assert!(state.events.iter().any(|record| {
        record.cursor > cursor_before_delete
            && matches!(
                &record.event,
                WorldEvent::Presence(presence)
                    if !presence.online && presence.account_id == "former-resident"
            )
    }));
}

#[test]
fn account_deletion_removes_phase4_and_phase5_replay_payloads() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("replay-cleanup-client".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "replay-cleanup-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "replay-cleanup-subject".to_owned(),
                display_name: Some("Replay cleanup resident".to_owned()),
            },
        )
        .unwrap()
        .data;
    let account_id = linked.account_id.clone();
    let identity_key = {
        let state = repository.state.lock().unwrap();
        state
            .phase6
            .accounts
            .get(&account_id)
            .expect("the linked account should be present")
            .identity_key
            .clone()
    };
    {
        let mut state = repository.state.lock().unwrap();
        state.phase3.expedition_credentials.push(account_id.clone());
    }

    repository
        .profession_order(
            &linked.session.account_token,
            ProfessionRequest {
                request_id: "replay-cleanup-phase4".to_owned(),
                action: ProfessionAction::Inspect,
                order_id: None,
                profession: None,
                capability_id: None,
                service: None,
                timing_score: None,
            },
        )
        .unwrap();
    repository
        .practice_skill(
            &linked.session.account_token,
            SkillRequest {
                request_id: "replay-cleanup-skill".to_owned(),
                action: SkillAction::Practice,
                lesson_id: None,
                skill_id: Some("fishing".to_owned()),
                target_account_id: None,
            },
        )
        .unwrap();
    let claim = repository
        .claim_lifecycle(
            &linked.session.account_token,
            ClaimLifecycleRequest {
                request_id: "replay-cleanup-claim-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data
        .claim
        .expect("the account should receive a claim before deletion");
    assert!(
        repository
            .claim_lifecycle(
                &linked.session.account_token,
                ClaimLifecycleRequest {
                    request_id: "replay-cleanup-claim-approve".to_owned(),
                    action: ClaimLifecycleAction::Approve,
                    claim_id: Some(claim.claim_id),
                    target_account_id: None,
                },
            )
            .unwrap()
            .data
            .accepted
    );
    assert!(
        repository
            .knowledge(
                &linked.session.account_token,
                KnowledgeRequest {
                    request_id: "replay-cleanup-knowledge".to_owned(),
                    action: KnowledgeAction::Discover,
                    knowledge_id: Some("moonberry-tending".to_owned()),
                    target_account_id: None,
                },
            )
            .unwrap()
            .data
            .accepted
    );
    repository
        .market_order(
            &linked.session.account_token,
            MarketOrderRequest {
                request_id: "replay-cleanup-phase5".to_owned(),
                action: MarketOrderAction::Cancel,
                order_id: Some("missing-replay-cleanup-order".to_owned()),
                destination_location_id: None,
                commodity: None,
                quantity: None,
            },
        )
        .unwrap();

    let deletion = repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "replay-cleanup-delete".to_owned(),
                account_id: account_id.clone(),
            },
        )
        .unwrap()
        .data;
    assert!(deletion.accepted);
    repository.tick();

    let state = repository.state.lock().unwrap();
    assert!(!state
        .phase4
        .request_results
        .keys()
        .any(|key| key.starts_with(&format!("phase4:{account_id}:"))));
    assert!(!state
        .phase4
        .request_results
        .keys()
        .any(|key| key.starts_with(&format!("skill-practice:{account_id}:"))));
    assert!(state
        .phase4
        .claims
        .iter()
        .all(|claim| { claim.approved_by.as_deref() != Some(account_id.as_str()) }));
    assert!(state
        .phase4
        .knowledge
        .iter()
        .all(|item| !item.discovered_by.iter().any(|id| id == &account_id)));
    assert!(!state
        .phase5
        .request_results
        .keys()
        .any(|key| key.starts_with(&format!("phase5:{identity_key}:"))));
    assert!(state.phase6.auth_link_tokens.is_empty());
    assert!(!state
        .phase3
        .expedition_credentials
        .iter()
        .any(|id| id == &account_id));
}

#[test]
fn account_deletion_anonymises_composite_moderation_targets() {
    let repository = WorldRepository::new(ServerConfig::default());
    let target_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("moderation-target-deletion".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let reporter = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("moderation-reporter-deletion".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let target = repository
        .auth_link(
            &target_guest.account_token,
            AuthLinkRequest {
                request_id: "moderation-target-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "moderation-target-subject".to_owned(),
                display_name: Some("Report target".to_owned()),
            },
        )
        .unwrap()
        .data;
    let message_id = repository
        .chat(
            &target.session.account_token,
            ChatRequest {
                request_id: "moderation-target-message".to_owned(),
                channel: "settlement".to_owned(),
                text: "A message remains as report evidence.".to_owned(),
            },
        )
        .unwrap()
        .data
        .message
        .expect("target message")
        .message_id;
    repository
        .moderation_report(
            &reporter.account_token,
            ModerationReportRequest {
                request_id: "moderation-target-report".to_owned(),
                target_account_id: Some(target.account_id.clone()),
                message_id: Some(message_id),
                category: "harassment".to_owned(),
                note: "The evidence should retain its report audit.".to_owned(),
            },
        )
        .expect("moderation report");

    repository
        .account_delete(
            &target.session.account_token,
            AccountDeletionRequest {
                request_id: "moderation-target-delete".to_owned(),
                account_id: target.account_id.clone(),
            },
        )
        .expect("account deletion");
    repository.tick();

    let state = repository.state.lock().unwrap();
    assert!(state
        .phase6
        .audits
        .iter()
        .all(|audit| !audit.target.contains(&target.account_id)));
    assert!(state
        .phase6
        .audits
        .iter()
        .any(|audit| { audit.target == format!("former-resident (message {message_id})") }));
}

#[test]
fn account_deletion_keeps_moderation_replays_for_identity_prefix_collisions() {
    let repository = WorldRepository::new(ServerConfig::default());
    let deleted_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("moderation-owner".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let retained_reporter = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("moderation-owner:observer".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let original = repository
        .moderation_report(
            &retained_reporter.account_token,
            ModerationReportRequest {
                request_id: "retained-moderation-replay".to_owned(),
                target_account_id: None,
                message_id: None,
                category: "harassment".to_owned(),
                note: "The observer report must survive another identity leaving.".to_owned(),
            },
        )
        .expect("retained moderation report")
        .data;
    let linked = repository
        .auth_link(
            &deleted_guest.account_token,
            AuthLinkRequest {
                request_id: "moderation-owner-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "moderation-owner-subject".to_owned(),
                display_name: Some("Departing moderation owner".to_owned()),
            },
        )
        .unwrap()
        .data;

    repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "moderation-owner-delete".to_owned(),
                account_id: linked.account_id,
            },
        )
        .expect("account deletion");
    repository.tick();

    let replayed = repository
        .moderation_report(
            &retained_reporter.account_token,
            ModerationReportRequest {
                request_id: "retained-moderation-replay".to_owned(),
                target_account_id: None,
                message_id: None,
                category: "harassment".to_owned(),
                note: "The observer report must survive another identity leaving.".to_owned(),
            },
        )
        .expect("retained moderation report should replay")
        .data;
    assert_eq!(replayed, original);
}

#[test]
fn account_deletion_removes_refresh_replay_after_access_expiry() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("refresh-replay-cleanup".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "refresh-replay-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "refresh-replay-subject".to_owned(),
                display_name: None,
            },
        )
        .unwrap()
        .data;
    let refreshed = repository
        .auth_refresh(AuthRefreshRequest {
            request_id: "refresh-replay-refresh".to_owned(),
            refresh_token: linked.session.refresh_token,
        })
        .unwrap()
        .data;
    let account_id = linked.account_id.clone();
    let deletion = repository
        .account_delete(
            &refreshed.session.account_token,
            AccountDeletionRequest {
                request_id: "refresh-replay-delete".to_owned(),
                account_id: account_id.clone(),
            },
        )
        .unwrap()
        .data;
    assert!(deletion.accepted);

    {
        let mut state = repository.state.lock().unwrap();
        state
            .phase6
            .sessions
            .remove(&refreshed.session.account_token);
        state.sessions.remove(&refreshed.session.account_token);
    }
    repository.tick();

    let state = repository.state.lock().unwrap();
    assert!(state
        .phase6
        .auth_refresh_results
        .values()
        .all(|response| response.session.account_token != refreshed.session.account_token));
    assert!(state
        .phase6
        .auth_refresh_accounts
        .values()
        .all(|account| account != &account_id));
}

#[test]
fn account_link_preserves_phase4_and_skill_replay_idempotency() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-replay-link".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let order_request = ProfessionRequest {
        request_id: "phase4-replay-order".to_owned(),
        action: ProfessionAction::CreateOrder,
        order_id: None,
        profession: Some(ProfessionKind::Carpenter),
        capability_id: None,
        service: None,
        timing_score: None,
    };
    let original_order = repository
        .profession_order(&guest.account_token, order_request.clone())
        .unwrap()
        .data
        .order
        .expect("the guest should create a service order");
    let skill_request = SkillRequest {
        request_id: "phase4-replay-skill".to_owned(),
        action: SkillAction::Practice,
        lesson_id: None,
        skill_id: Some("fishing".to_owned()),
        target_account_id: None,
    };
    repository
        .practice_skill(&guest.account_token, skill_request.clone())
        .unwrap();
    {
        let mut state = repository.state.lock().unwrap();
        state
            .phase3
            .expedition_credentials
            .push(guest.account_id.clone());
    }

    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "phase4-replay-link-request".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "phase4-replay-subject".to_owned(),
                display_name: Some("Linked replay resident".to_owned()),
            },
        )
        .unwrap()
        .data;
    let replayed_order = repository
        .profession_order(&linked.session.account_token, order_request)
        .unwrap()
        .data
        .order
        .expect("the pre-link order request should replay after linking");
    assert_eq!(replayed_order.order_id, original_order.order_id);
    assert_eq!(replayed_order.requester_account_id, linked.account_id);
    assert_eq!(replayed_order.requester_name, "Linked replay resident");
    repository
        .practice_skill(&linked.session.account_token, skill_request)
        .unwrap();

    let state = repository.state.lock().unwrap();
    let identity_key = state
        .phase6
        .accounts
        .get(&linked.account_id)
        .expect("the linked account should be present")
        .identity_key
        .clone();
    assert_eq!(
        state
            .identities
            .get(&identity_key)
            .and_then(|identity| identity.skills.practice.get("fishing"))
            .copied(),
        Some(1)
    );
    assert!(state
        .phase3
        .expedition_credentials
        .iter()
        .any(|id| id == &linked.account_id));
    assert!(!state
        .phase3
        .expedition_credentials
        .iter()
        .any(|id| id == &guest.account_id));
}

#[test]
fn account_link_preserves_support_replay_idempotency() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned(), "account-1".to_owned()],
        ..ServerConfig::default()
    });
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("support-replay-link".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let repair_request = SupportRepairRequest {
        request_id: "support-replay-link-request".to_owned(),
        action: SupportRepairAction::NormalizeInventory,
        account_id: None,
        target_id: None,
        note: "The support repair remains idempotent through identity linking.".to_owned(),
    };
    let original = repository
        .support_repair(&guest.account_token, repair_request.clone())
        .unwrap()
        .data;
    assert!(original.accepted);

    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "support-replay-link-auth".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "support-replay-link-subject".to_owned(),
                display_name: Some("Linked support operator".to_owned()),
            },
        )
        .unwrap()
        .data;
    let replayed = repository
        .support_repair(&linked.session.account_token, repair_request)
        .unwrap()
        .data;

    assert_eq!(replayed.audit_id, original.audit_id);
}
