use super::*;
use tarrowyn_protocol::{
    AccountDeletionRequest, AuthLinkRequest, AuthRefreshRequest, ChatRequest, ClaimLifecycleAction,
    ClaimLifecycleRequest, GuestSessionRequest, KnowledgeAction, KnowledgeRequest,
    MarketOrderAction, MarketOrderRequest, ModerationReportRequest, ProfessionAction,
    ProfessionKind, ProfessionRequest, SkillAction, SkillRequest, WorldEvent,
};

mod deletion_state;
mod identity_link;
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
fn account_deletion_anonymises_market_replays_kept_by_another_identity() {
    let repository = WorldRepository::new(ServerConfig::default());
    let owner_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("market-replay-owner".to_owned()),
            reset: false,
        })
        .expect("owner guest session")
        .data;
    let owner = repository
        .auth_link(
            &owner_guest.account_token,
            AuthLinkRequest {
                request_id: "market-replay-owner-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "market-replay-owner-subject".to_owned(),
                display_name: Some("Market owner".to_owned()),
            },
        )
        .expect("owner link")
        .data;
    let observer = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("market-replay-observer".to_owned()),
            reset: false,
        })
        .expect("observer guest session")
        .data;
    let created = repository
        .market_order(
            &owner.session.account_token,
            MarketOrderRequest {
                request_id: "market-replay-create".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("whisperwood-outpost".to_owned()),
                commodity: Some(tarrowyn_protocol::CommodityKind::Seeds),
                quantity: Some(1),
            },
        )
        .expect("market order creation")
        .data;
    let order_id = created.order.expect("created order").order_id;
    {
        let mut state = repository.state.lock().expect("repository lock");
        let destination = state
            .phase5
            .locations
            .iter()
            .find(|location| location.location_id == "whisperwood-outpost")
            .expect("market destination")
            .position;
        state
            .identities
            .get_mut(&observer.client_key)
            .expect("observer identity")
            .position = destination;
    }
    let fulfil_request = MarketOrderRequest {
        request_id: "market-replay-fulfil".to_owned(),
        action: MarketOrderAction::Fulfil,
        order_id: Some(order_id),
        destination_location_id: None,
        commodity: None,
        quantity: None,
    };
    let fulfilled = repository
        .market_order(&observer.account_token, fulfil_request.clone())
        .expect("market order fulfilment")
        .data;
    assert_eq!(
        fulfilled
            .order
            .as_ref()
            .expect("fulfilled order")
            .owner_account_id,
        owner.account_id
    );

    repository
        .account_delete(
            &owner.session.account_token,
            AccountDeletionRequest {
                request_id: "market-replay-owner-delete".to_owned(),
                account_id: owner.account_id,
            },
        )
        .expect("schedule owner deletion");
    repository.tick();

    let replay = repository
        .market_order(&observer.account_token, fulfil_request)
        .expect("market replay")
        .data;
    let replayed_order = replay.order.expect("replayed order");
    assert_eq!(replayed_order.owner_account_id, "former-resident");
    assert_eq!(replayed_order.owner_name, "Former resident");
    assert!(repository.ops_health().data.ready);
}

#[test]
fn account_deletion_anonymises_claim_replays_kept_by_another_identity() {
    let repository = WorldRepository::new(ServerConfig::default());
    let owner_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("claim-replay-owner".to_owned()),
            reset: false,
        })
        .expect("owner guest session")
        .data;
    let owner = repository
        .auth_link(
            &owner_guest.account_token,
            AuthLinkRequest {
                request_id: "claim-replay-owner-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "claim-replay-owner-subject".to_owned(),
                display_name: Some("Claim owner".to_owned()),
            },
        )
        .expect("owner link")
        .data;
    let observer = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("claim-replay-observer".to_owned()),
            reset: false,
        })
        .expect("observer guest session")
        .data;
    let claim = repository
        .claim_lifecycle(
            &owner.session.account_token,
            ClaimLifecycleRequest {
                request_id: "claim-replay-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request")
        .data
        .claim
        .expect("requested claim");
    let approved = repository
        .claim_lifecycle(
            &owner.session.account_token,
            ClaimLifecycleRequest {
                request_id: "claim-replay-approve".to_owned(),
                action: ClaimLifecycleAction::Approve,
                claim_id: Some(claim.claim_id.clone()),
                target_account_id: None,
            },
        )
        .expect("claim approval")
        .data
        .claim
        .expect("approved claim");
    let inspect_request = ClaimLifecycleRequest {
        request_id: "claim-replay-inspect".to_owned(),
        action: ClaimLifecycleAction::Inspect,
        claim_id: None,
        target_account_id: None,
    };
    let inspected = repository
        .claim_lifecycle(&observer.account_token, inspect_request.clone())
        .expect("claim inspection")
        .data;
    assert!(inspected.claims.claims.iter().any(|record| {
        record.claim_id == approved.claim_id
            && record.owner_account_id.as_deref() == Some(owner.account_id.as_str())
    }));

    repository
        .account_delete(
            &owner.session.account_token,
            AccountDeletionRequest {
                request_id: "claim-replay-owner-delete".to_owned(),
                account_id: owner.account_id,
            },
        )
        .expect("schedule owner deletion");
    repository.tick();

    let replay = repository
        .claim_lifecycle(&observer.account_token, inspect_request)
        .expect("claim replay")
        .data;
    let replayed = replay
        .claims
        .claims
        .iter()
        .find(|record| record.claim_id == approved.claim_id)
        .expect("replayed claim");
    assert!(replayed.owner_account_id.is_none());
    assert!(replayed.owner_name.is_none());
    assert_eq!(
        replayed.status,
        tarrowyn_protocol::ClaimLifecycleStatus::Reclaimed
    );
    assert!(replay.claims.available_plots.contains(&replayed.position));
    assert!(repository.ops_health().data.ready);
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
                note: format!(
                    "The evidence for {} named {} should retain its report audit.",
                    target.account_id, target.display_name
                ),
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
    let audit = state
        .phase6
        .audits
        .iter()
        .find(|audit| audit.action == "moderation.report:harassment")
        .expect("moderation audit");
    assert!(!audit.note.contains(&target.account_id));
    assert!(!audit.note.contains(&target.display_name));
    assert!(audit.note.contains("former-resident"));
    assert!(audit.note.contains("Former resident"));
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
fn account_deletion_anonymises_knowledge_replays_kept_by_another_identity() {
    let repository = WorldRepository::new(ServerConfig::default());
    let owner_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("knowledge-replay-owner".to_owned()),
            reset: false,
        })
        .expect("owner guest session")
        .data;
    let owner = repository
        .auth_link(
            &owner_guest.account_token,
            AuthLinkRequest {
                request_id: "knowledge-replay-owner-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "knowledge-replay-owner-subject".to_owned(),
                display_name: Some("Departing archivist".to_owned()),
            },
        )
        .expect("owner link")
        .data;
    let owner_account_id = owner.account_id.clone();
    let observer = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("knowledge-replay-observer".to_owned()),
            reset: false,
        })
        .expect("observer guest session")
        .data;
    repository
        .knowledge(
            &owner.session.account_token,
            KnowledgeRequest {
                request_id: "knowledge-replay-discover".to_owned(),
                action: KnowledgeAction::Discover,
                knowledge_id: Some("moonberry-tending".to_owned()),
                target_account_id: None,
            },
        )
        .expect("knowledge discovery");
    repository
        .knowledge(
            &owner.session.account_token,
            KnowledgeRequest {
                request_id: "knowledge-replay-record".to_owned(),
                action: KnowledgeAction::Record,
                knowledge_id: Some("moonberry-tending".to_owned()),
                target_account_id: None,
            },
        )
        .expect("knowledge recording");
    let apply_request = KnowledgeRequest {
        request_id: "knowledge-replay-apply".to_owned(),
        action: KnowledgeAction::Apply,
        knowledge_id: Some("moonberry-tending".to_owned()),
        target_account_id: None,
    };
    let applied = repository
        .knowledge(&observer.account_token, apply_request.clone())
        .expect("knowledge application")
        .data;
    assert!(applied.knowledge.items.iter().any(|item| {
        item.knowledge_id == "moonberry-tending"
            && item.discovered_by.iter().any(|id| id == &owner_account_id)
    }));

    repository
        .account_delete(
            &owner.session.account_token,
            AccountDeletionRequest {
                request_id: "knowledge-replay-owner-delete".to_owned(),
                account_id: owner.account_id,
            },
        )
        .expect("schedule owner deletion");
    repository.tick();

    let replay = repository
        .knowledge(&observer.account_token, apply_request)
        .expect("knowledge replay")
        .data;
    let replayed = replay
        .knowledge
        .items
        .iter()
        .find(|item| item.knowledge_id == "moonberry-tending")
        .expect("replayed knowledge");
    assert!(!replayed
        .discovered_by
        .iter()
        .any(|id| id == &owner_account_id));
    assert!(repository.ops_health().data.ready);
}
