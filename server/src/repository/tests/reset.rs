use super::super::WorldRepository;
use crate::ServerConfig;
use tarrowyn_protocol::{
    AuditRecord, AuthRevokeRequest, ChatMessage, ChatRequest, ChronicleEntry, ClaimAction,
    ClaimLifecycleAction, ClaimLifecycleRequest, ClaimRequest, FrontierEvent, GovernanceDecision,
    GuestSessionRequest, MarketOrderAction, MarketOrderRequest, ModerationReportRequest,
    ProfessionAction, ProfessionKind, ProfessionRequest, PublicAction, TradeAction, TradeBundle,
    TradeRequest, TravelAction, TravelRequest, WorldEvent,
};

#[test]
fn restored_state_anonymises_orphaned_audit_targets() {
    let config = ServerConfig::default();
    let mut stored = super::super::models::RepositoryState::fresh(&config).to_stored();
    stored.phase6.audits.push_back(AuditRecord {
        audit_id: "audit-orphan-exact".to_owned(),
        actor_account_id: "orphan-account".to_owned(),
        action: "support.repair".to_owned(),
        target: "orphan-account".to_owned(),
        outcome: "accepted".to_owned(),
        tick: 1,
        note: "An orphaned exact target orphan-account must not survive restore.".to_owned(),
    });
    stored.phase6.audits.push_back(AuditRecord {
        audit_id: "audit-orphan-composite".to_owned(),
        actor_account_id: "operator-account".to_owned(),
        action: "moderation.report:harassment".to_owned(),
        target: "orphan-account (message 7)".to_owned(),
        outcome: "accepted".to_owned(),
        tick: 2,
        note: "An orphaned composite target orphan-account must not survive restore.".to_owned(),
    });
    stored.phase6.audits.push_back(AuditRecord {
        audit_id: "audit-semantic-target".to_owned(),
        actor_account_id: "former-resident".to_owned(),
        action: "governance.tax".to_owned(),
        target: "tax-policy".to_owned(),
        outcome: "accepted".to_owned(),
        tick: 3,
        note: "The tax-policy target keeps its audit meaning after restore.".to_owned(),
    });

    let restored = super::super::models::RepositoryState::from_stored(stored, &config);
    assert_eq!(
        restored.phase6.audits[0].actor_account_id,
        "former-resident"
    );
    assert_eq!(restored.phase6.audits[0].target, "former-resident");
    assert_eq!(
        restored.phase6.audits[1].target,
        "former-resident (message 7)"
    );
    assert_eq!(restored.phase6.audits[2].target, "tax-policy");
    assert!(restored
        .phase6
        .audits
        .iter()
        .all(|audit| !audit.note.contains("orphan-account")));
}

#[test]
fn restored_state_anonymises_orphaned_chronicle_names() {
    let config = ServerConfig::default();
    let mut stored = super::super::models::RepositoryState::fresh(&config).to_stored();
    stored.chat_history.push_back(ChatMessage {
        message_id: 1,
        account_id: "orphan-account".to_owned(),
        display_name: "Orphan resident".to_owned(),
        channel: "settlement".to_owned(),
        text: "A retained public message.".to_owned(),
        cursor: 1,
    });
    let history = ChronicleEntry {
        event_id: "orphan-chronicle".to_owned(),
        kind: "social".to_owned(),
        title: "Orphan resident keeps the road".to_owned(),
        text: "The Hearth remembers Orphan resident beside the road.".to_owned(),
        created_tick: 1,
        cursor: 1,
    };
    stored.phase3.chronicle.push_back(history.clone());
    stored.phase5.settlements[0].chronicle.push(history);

    let restored = super::super::models::RepositoryState::from_stored(stored, &config);
    assert_eq!(restored.chat_history[0].display_name, "Former resident");
    assert!(!restored.phase3.chronicle[0]
        .title
        .contains("Orphan resident"));
    assert!(!restored.phase3.chronicle[0]
        .text
        .contains("Orphan resident"));
    assert!(!restored.phase5.settlements[0].chronicle[0]
        .title
        .contains("Orphan resident"));
    assert!(!restored.phase5.settlements[0].chronicle[0]
        .text
        .contains("Orphan resident"));
}

#[test]
fn restored_state_anonymises_orphaned_governance_chronicle_names() {
    let config = ServerConfig::default();
    let mut stored = super::super::models::RepositoryState::fresh(&config).to_stored();
    stored.phase4.governance.decisions.push(GovernanceDecision {
        decision_id: "decision-orphan".to_owned(),
        actor_account_id: "orphan-account".to_owned(),
        actor_name: "Orphan registrar".to_owned(),
        action: PublicAction::RepairRoad,
        proposal_id: "proposal-orphan".to_owned(),
        cost: 4,
        service_affected: "north road".to_owned(),
        created_tick: 1,
    });
    stored.phase6.audits.push_back(AuditRecord {
        audit_id: "audit-orphan-governance".to_owned(),
        actor_account_id: "orphan-account".to_owned(),
        action: "governance.tax".to_owned(),
        target: "tax-policy".to_owned(),
        outcome: "accepted".to_owned(),
        tick: 1,
        note: "Orphan registrar recorded the public repair.".to_owned(),
    });
    stored.phase3.chronicle.push_back(ChronicleEntry {
        event_id: "orphan-governance-chronicle".to_owned(),
        kind: "governance".to_owned(),
        title: "Orphan registrar steadies the road".to_owned(),
        text: "The ledger remembers Orphan registrar's public repair.".to_owned(),
        created_tick: 1,
        cursor: 1,
    });

    let restored = super::super::models::RepositoryState::from_stored(stored, &config);
    assert!(!restored.phase3.chronicle[0]
        .title
        .contains("Orphan registrar"));
    assert!(!restored.phase3.chronicle[0]
        .text
        .contains("Orphan registrar"));
    assert!(!restored.phase6.audits[0].note.contains("Orphan registrar"));
}

#[test]
fn guest_reset_anonymises_shared_replays_and_composite_audits() {
    let repository = WorldRepository::new(ServerConfig::default());
    let reset_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("reset-replay-owner".to_owned()),
            reset: false,
        })
        .expect("reset guest")
        .data;
    let observer = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("reset-replay-observer".to_owned()),
            reset: false,
        })
        .expect("observer guest")
        .data;

    repository
        .claim(
            &reset_guest.account_token,
            ClaimRequest {
                request_id: "reset-replay-claim-request".to_owned(),
                action: ClaimAction::Request,
            },
        )
        .expect("the reset guest should claim frontier land");
    let claim_inspect = ClaimRequest {
        request_id: "reset-replay-claim-inspect".to_owned(),
        action: ClaimAction::Inspect,
    };
    let original_claim = repository
        .claim(&observer.account_token, claim_inspect.clone())
        .expect("the observer should cache the frontier claim")
        .data
        .claim
        .expect("the cached claim should be present");
    assert_eq!(original_claim.owner_account_id, reset_guest.account_id);

    let market = repository
        .market_order(
            &reset_guest.account_token,
            MarketOrderRequest {
                request_id: "reset-replay-market-create".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("whisperwood-outpost".to_owned()),
                commodity: Some(tarrowyn_protocol::CommodityKind::Seeds),
                quantity: Some(1),
            },
        )
        .expect("the reset guest should create a market order")
        .data
        .order
        .expect("market order");
    let market_review = MarketOrderRequest {
        request_id: "reset-replay-market-review".to_owned(),
        action: MarketOrderAction::Fulfil,
        order_id: Some(market.order_id),
        destination_location_id: None,
        commodity: None,
        quantity: None,
    };
    repository
        .market_order(&observer.account_token, market_review.clone())
        .expect("the observer should cache the market response");

    let trade = repository
        .trade(
            &reset_guest.account_token,
            TradeRequest {
                request_id: "reset-replay-trade-create".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(observer.account_id.clone()),
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
        .expect("the reset guest should create a trade")
        .data
        .trade
        .expect("trade offer");
    let trade_review = TradeRequest {
        request_id: "reset-replay-trade-review".to_owned(),
        action: TradeAction::Review,
        trade_id: Some(trade.trade_id),
        recipient_account_id: None,
        offer: None,
        request: None,
    };
    repository
        .trade(&observer.account_token, trade_review.clone())
        .expect("the observer should cache the trade response");

    let message_id = repository
        .chat(
            &reset_guest.account_token,
            ChatRequest {
                request_id: "reset-replay-message".to_owned(),
                channel: "settlement".to_owned(),
                text: "This audit target should lose its old account.".to_owned(),
            },
        )
        .expect("the reset guest should create a message")
        .data
        .message
        .expect("message")
        .message_id;
    repository
        .moderation_report(
            &observer.account_token,
            ModerationReportRequest {
                request_id: "reset-replay-report".to_owned(),
                target_account_id: Some(reset_guest.account_id.clone()),
                message_id: Some(message_id),
                category: "harassment".to_owned(),
                note: format!(
                    "The target {} named {} should remain safe after a development reset.",
                    reset_guest.account_id, reset_guest.display_name
                ),
            },
        )
        .expect("the observer should create a moderation audit");

    repository
        .guest_session(GuestSessionRequest {
            client_key: Some(reset_guest.client_key),
            reset: true,
        })
        .expect("guest reset");

    let replayed_claim = repository
        .claim(&observer.account_token, claim_inspect)
        .expect("the claim replay should remain available")
        .data
        .claim
        .expect("replayed claim");
    assert_eq!(replayed_claim.owner_account_id, "former-resident");
    assert_eq!(replayed_claim.owner_name, "Former resident");
    assert_eq!(
        replayed_claim.status,
        tarrowyn_protocol::ClaimStatus::Reclaimed
    );

    let replayed_market = repository
        .market_order(&observer.account_token, market_review)
        .expect("the market replay should remain available")
        .data
        .order
        .expect("replayed market order");
    assert_eq!(replayed_market.owner_account_id, "former-resident");
    assert_eq!(replayed_market.owner_name, "Former resident");

    let replayed_trade = repository
        .trade(&observer.account_token, trade_review)
        .expect("the trade replay should remain available")
        .data;
    assert!(!replayed_trade.accepted);
    assert!(replayed_trade.trade.is_none());

    let state = repository.state.lock().expect("repository state");
    let audit = state
        .phase6
        .audits
        .iter()
        .find(|audit| audit.action == "moderation.report:harassment")
        .expect("moderation audit");
    assert_eq!(
        audit.target,
        format!("former-resident (message {message_id})")
    );
    assert!(!audit.note.contains("dev-account-1"));
    assert!(!audit.note.contains("Guest 1"));
    assert!(audit.note.contains("former-resident"));
    assert!(audit.note.contains("Former resident"));
}

#[test]
fn guest_reset_keeps_moderation_replays_for_identity_prefix_collisions() {
    let repository = WorldRepository::new(ServerConfig::default());
    let reset_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("reset-owner".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let retained_reporter = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("reset-owner:observer".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let original = repository
        .moderation_report(
            &retained_reporter.account_token,
            ModerationReportRequest {
                request_id: "reset-retained-moderation".to_owned(),
                target_account_id: None,
                message_id: None,
                category: "harassment".to_owned(),
                note: "The observer report must survive a guest reset.".to_owned(),
            },
        )
        .unwrap()
        .data;

    repository
        .guest_session(GuestSessionRequest {
            client_key: Some(reset_guest.client_key),
            reset: true,
        })
        .expect("guest reset");

    let state = repository.state.lock().unwrap();
    assert!(state.phase6.auth_revoke_results.is_empty());
    drop(state);

    let replayed = repository
        .moderation_report(
            &retained_reporter.account_token,
            ModerationReportRequest {
                request_id: "reset-retained-moderation".to_owned(),
                target_account_id: None,
                message_id: None,
                category: "harassment".to_owned(),
                note: "The observer report must survive a guest reset.".to_owned(),
            },
        )
        .expect("retained moderation report should replay")
        .data;
    assert_eq!(replayed, original);
}

#[test]
fn guest_reset_records_departure_before_replacing_identity() {
    let repository = WorldRepository::new(ServerConfig::default());
    let first = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("reset-presence".to_owned()),
            reset: false,
        })
        .expect("first guest")
        .data;

    let second = repository
        .guest_session(GuestSessionRequest {
            client_key: Some(first.client_key),
            reset: true,
        })
        .expect("guest reset")
        .data;

    let state = repository.state.lock().expect("state lock");
    assert!(state.events.iter().any(|record| matches!(
        &record.event,
        WorldEvent::Presence(presence)
            if !presence.online && presence.account_id == "former-resident"
    )));
    assert!(state.events.iter().any(|record| matches!(
        &record.event,
        WorldEvent::Presence(presence)
            if presence.online && presence.account_id == second.account_id
    )));
}

#[test]
fn guest_reset_replaces_private_state_and_releases_world_ownership() {
    let repository = WorldRepository::new(ServerConfig::default());
    let first = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("reset-private-state".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    {
        let mut state = repository.state.lock().unwrap();
        state
            .phase3
            .expedition_credentials
            .push(first.account_id.clone());
    }
    let order = repository
        .profession_order(
            &first.account_token,
            ProfessionRequest {
                request_id: "reset-order".to_owned(),
                action: ProfessionAction::CreateOrder,
                order_id: None,
                profession: Some(ProfessionKind::Carpenter),
                capability_id: None,
                service: None,
                timing_score: None,
            },
        )
        .unwrap()
        .data
        .order
        .expect("the first guest should create an order");
    let claim = repository
        .claim_lifecycle(
            &first.account_token,
            ClaimLifecycleRequest {
                request_id: "reset-claim".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data
        .claim
        .expect("the first guest should receive a claim");
    let travel = repository
        .travel(
            &first.account_token,
            TravelRequest {
                request_id: "reset-travel".to_owned(),
                action: TravelAction::Start,
                route_id: Some("north-pack-road".to_owned()),
                travel_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(travel.accepted);
    let market = repository
        .market_order(
            &first.account_token,
            MarketOrderRequest {
                request_id: "reset-market".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("whisperwood-outpost".to_owned()),
                commodity: Some(tarrowyn_protocol::CommodityKind::Seeds),
                quantity: Some(1),
            },
        )
        .unwrap()
        .data;
    assert!(market.accepted);
    repository
        .chat(
            &first.account_token,
            ChatRequest {
                request_id: "reset-chat".to_owned(),
                channel: "settlement".to_owned(),
                text: "This history must lose its reset identity.".to_owned(),
            },
        )
        .expect("the guest should create a public message");

    let revoke = repository
        .auth_revoke(
            &first.account_token,
            AuthRevokeRequest {
                request_id: "reset-revoke".to_owned(),
                revoke_all: false,
            },
        )
        .unwrap();
    assert_eq!(revoke.data.revoked_sessions, 1);
    assert!(repository.world(&first.account_token).is_err());
    let replayed_revoke = repository
        .auth_revoke(
            &first.account_token,
            AuthRevokeRequest {
                request_id: "reset-revoke".to_owned(),
                revoke_all: false,
            },
        )
        .expect("the same guest revoke should replay after removal");
    assert_eq!(replayed_revoke.data, revoke.data);

    let second = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("reset-private-state".to_owned()),
            reset: true,
        })
        .unwrap()
        .data;
    assert_ne!(first.account_id, second.account_id);
    let professions = repository.professions(&second.account_token).unwrap().data;
    assert_eq!(professions.materials.wood, 3);
    assert_eq!(professions.materials.iron, 2);
    assert_eq!(professions.materials.tools, 1);
    assert!(repository
        .region(&second.account_token)
        .unwrap()
        .data
        .travel
        .is_none());
    let state = repository.state.lock().unwrap();
    assert!(!state.phase3.contracts.contains_key(&first.client_key));
    assert!(!state
        .phase3
        .expedition_credentials
        .iter()
        .any(|id| id == &first.account_id));
    assert!(!state.phase5.travel.contains_key(&first.client_key));
    assert!(!state.phase4.request_results.keys().any(|key| {
        key.starts_with(&format!("phase4:{}:", first.account_id))
            || key.starts_with(&format!("skill-practice:{}:", first.account_id))
    }));
    assert!(!state
        .phase5
        .request_results
        .keys()
        .any(|key| key.starts_with(&format!("phase5:{}:", first.client_key))));
    assert!(state.phase6.auth_revoke_results.is_empty());
    assert!(state.phase6.auth_revoke_guest_tokens.is_empty());
    assert!(state.events.iter().all(|record| match &record.event {
        WorldEvent::Presence(presence) => presence.account_id != first.account_id,
        WorldEvent::Chat(message) => message.account_id != first.account_id,
        WorldEvent::Trade(trade) => {
            trade.creator_account_id != first.account_id
                && trade.recipient_account_id != first.account_id
        }
        WorldEvent::Frontier(FrontierEvent::Claim(claim)) => {
            claim.owner_account_id != first.account_id
        }
        WorldEvent::Frontier(FrontierEvent::Expedition(expedition)) => {
            expedition.leader_account_id != first.account_id
                && expedition
                    .members
                    .iter()
                    .all(|member| member.account_id != first.account_id)
        }
        WorldEvent::Clock(_)
        | WorldEvent::Farming(_)
        | WorldEvent::TavernNotice(_)
        | WorldEvent::Chronicle(_)
        | WorldEvent::Frontier(FrontierEvent::Threat(_))
        | WorldEvent::Frontier(FrontierEvent::Opportunity(_)) => true,
    }));
    assert!(state.chat_history.iter().any(|message| {
        message.account_id == "former-resident"
            && message.text.contains("development identity reset")
    }));
    let claim = state
        .phase4
        .claims
        .iter()
        .find(|candidate| candidate.claim_id == claim.claim_id)
        .expect("the released claim remains as registry history");
    assert_eq!(
        claim.status,
        tarrowyn_protocol::ClaimLifecycleStatus::Reclaimed
    );
    assert!(claim.owner_account_id.is_none());
    let order = state
        .phase4
        .orders
        .iter()
        .find(|candidate| candidate.order_id == order.order_id)
        .expect("the reset order remains as history");
    assert_eq!(
        order.status,
        tarrowyn_protocol::ServiceOrderStatus::Cancelled
    );
    let market = state
        .phase5
        .market_orders
        .iter()
        .find(|candidate| {
            market
                .order
                .as_ref()
                .is_some_and(|created| created.order_id == candidate.order_id)
        })
        .expect("the reset market order remains as history");
    assert_eq!(market.owner_account_id, "former-resident");
    assert_eq!(
        state
            .identities
            .get(&second.client_key)
            .expect("the replacement identity should remain")
            .account_id,
        second.account_id
    );
    drop(state);
    assert!(repository.ops_health().data.integrity_ok);
}
