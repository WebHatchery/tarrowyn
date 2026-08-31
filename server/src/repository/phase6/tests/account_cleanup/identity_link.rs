use super::*;
use tarrowyn_protocol::{ChatRequest, ModerationReportRequest};

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

#[test]
fn account_link_migrates_trade_replays_kept_by_another_identity() {
    let repository = WorldRepository::new(ServerConfig::default());
    let creator_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("trade-link-creator".to_owned()),
            reset: false,
        })
        .expect("creator guest session")
        .data;
    let recipient = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("trade-link-recipient".to_owned()),
            reset: false,
        })
        .expect("recipient guest session")
        .data;
    let created = repository
        .trade(
            &creator_guest.account_token,
            TradeRequest {
                request_id: "trade-link-create".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(recipient.account_id.clone()),
                offer: Some(TradeBundle {
                    seeds: 1,
                    ..TradeBundle::default()
                }),
                request: Some(TradeBundle {
                    gold: 1,
                    ..TradeBundle::default()
                }),
            },
        )
        .expect("trade creation")
        .data;
    let trade_id = created.trade.expect("created trade").trade_id;
    let review_request = TradeRequest {
        request_id: "trade-link-review".to_owned(),
        action: TradeAction::Review,
        trade_id: Some(trade_id),
        recipient_account_id: None,
        offer: None,
        request: None,
    };
    repository
        .trade(&recipient.account_token, review_request.clone())
        .expect("trade review");

    let linked = repository
        .auth_link(
            &creator_guest.account_token,
            AuthLinkRequest {
                request_id: "trade-link-request".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "trade-link-subject".to_owned(),
                display_name: Some("Linked trade creator".to_owned()),
            },
        )
        .expect("creator link")
        .data;
    let replay = repository
        .trade(&recipient.account_token, review_request)
        .expect("trade replay")
        .data
        .trade
        .expect("replayed trade");

    assert_eq!(replay.creator_account_id, linked.account_id);
    assert_eq!(replay.creator_name, "Linked trade creator");
    assert!(repository.ops_health().data.ready);
}

#[test]
fn account_link_migrates_composite_moderation_audit_targets() {
    let repository = WorldRepository::new(ServerConfig::default());
    let target = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("audit-link-target".to_owned()),
            reset: false,
        })
        .expect("target guest session")
        .data;
    let reporter = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("audit-link-reporter".to_owned()),
            reset: false,
        })
        .expect("reporter guest session")
        .data;
    let message_id = repository
        .chat(
            &target.account_token,
            ChatRequest {
                request_id: "audit-link-message".to_owned(),
                channel: "settlement".to_owned(),
                text: "A message retained for the audit target fixture.".to_owned(),
            },
        )
        .expect("target chat")
        .data
        .message
        .expect("target message")
        .message_id;
    repository
        .moderation_report(
            &reporter.account_token,
            ModerationReportRequest {
                request_id: "audit-link-report".to_owned(),
                target_account_id: Some(target.account_id.clone()),
                message_id: Some(message_id),
                category: "harassment".to_owned(),
                note: format!(
                    "The audit target {} named {} should follow the account link.",
                    target.account_id, target.display_name
                ),
            },
        )
        .expect("moderation report");

    let linked = repository
        .auth_link(
            &target.account_token,
            AuthLinkRequest {
                request_id: "audit-link-auth".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "audit-link-subject".to_owned(),
                display_name: Some("Linked audit resident".to_owned()),
            },
        )
        .expect("account link")
        .data;
    let state = repository.state.lock().expect("repository lock");
    let audit = state
        .phase6
        .audits
        .iter()
        .find(|audit| audit.action == "moderation.report:harassment")
        .expect("moderation audit");

    assert_eq!(
        audit.target,
        format!("{} (message {message_id})", linked.account_id)
    );
    assert!(!audit.note.contains(&target.account_id));
    assert!(!audit.note.contains(&target.display_name));
    assert!(audit.note.contains(&linked.account_id));
    assert!(audit.note.contains("Linked audit resident"));
    drop(state);
    assert!(repository.ops_health().data.ready);
}
