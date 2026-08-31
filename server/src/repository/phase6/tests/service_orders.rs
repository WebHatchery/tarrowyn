use super::*;
use tarrowyn_protocol::{
    AccountDeletionRequest, AuthLinkRequest, GuestSessionRequest, MarketOrderAction,
    MarketOrderRequest, ProfessionAction, ProfessionKind, ProfessionRequest,
};

#[test]
fn provider_deletion_returns_surviving_requester_service_escrow() {
    let repository = WorldRepository::new(ServerConfig::default());
    let requester = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("service-escrow-requester".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let provider = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("service-escrow-provider".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let order = repository
        .profession_order(
            &requester.account_token,
            ProfessionRequest {
                request_id: "service-escrow-create".to_owned(),
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
        .expect("the requester should create a service order");
    let learned = repository
        .profession_order(
            &provider.account_token,
            ProfessionRequest {
                request_id: "service-escrow-learn".to_owned(),
                action: ProfessionAction::LearnCapability,
                order_id: None,
                profession: Some(ProfessionKind::Carpenter),
                capability_id: None,
                service: None,
                timing_score: None,
            },
        )
        .unwrap()
        .data;
    assert!(learned.accepted);
    assert!(
        repository
            .profession_order(
                &provider.account_token,
                ProfessionRequest {
                    request_id: "service-escrow-accept".to_owned(),
                    action: ProfessionAction::AcceptOrder,
                    order_id: Some(order.order_id.clone()),
                    profession: None,
                    capability_id: None,
                    service: None,
                    timing_score: None,
                },
            )
            .unwrap()
            .data
            .accepted
    );
    let before_delete = repository
        .professions(&requester.account_token)
        .unwrap()
        .data
        .materials;
    assert_eq!(before_delete.wood, 2);
    assert_eq!(before_delete.iron, 1);
    assert_eq!(before_delete.tools, 0);

    let linked = repository
        .auth_link(
            &provider.account_token,
            AuthLinkRequest {
                request_id: "service-escrow-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "service-escrow-provider-subject".to_owned(),
                display_name: Some("Departing provider".to_owned()),
            },
        )
        .unwrap()
        .data;
    let deletion = repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "service-escrow-delete".to_owned(),
                account_id: linked.account_id,
            },
        )
        .unwrap()
        .data;
    assert!(deletion.accepted);
    repository.tick();

    let after_delete = repository
        .professions(&requester.account_token)
        .unwrap()
        .data
        .materials;
    assert_eq!(after_delete.wood, 3);
    assert_eq!(after_delete.iron, 2);
    assert_eq!(after_delete.tools, 1);
    let state = repository.state.lock().unwrap();
    let order = state
        .phase4
        .orders
        .iter()
        .find(|candidate| candidate.order_id == order.order_id)
        .expect("the cancelled order remains as history");
    assert_eq!(
        order.status,
        tarrowyn_protocol::ServiceOrderStatus::Cancelled
    );
    assert!(order.provider_account_id.is_none());
}

#[test]
fn account_link_migrates_identity_keyed_market_replays() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("market-replay-link".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let request = MarketOrderRequest {
        request_id: "market-replay".to_owned(),
        action: MarketOrderAction::Create,
        order_id: None,
        destination_location_id: Some("whisperwood-outpost".to_owned()),
        commodity: Some(tarrowyn_protocol::CommodityKind::Seeds),
        quantity: Some(1),
    };
    let original = repository
        .market_order(&session.account_token, request.clone())
        .unwrap()
        .data
        .order
        .expect("the guest should create a market order");
    let linked = repository
        .auth_link(
            &session.account_token,
            AuthLinkRequest {
                request_id: "market-replay-link-request".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "market-replay-subject".to_owned(),
                display_name: Some("Linked market resident".to_owned()),
            },
        )
        .unwrap()
        .data;
    let replay = repository
        .market_order(&linked.session.account_token, request)
        .unwrap()
        .data
        .order
        .expect("the identity-keyed replay should remain available");
    assert_eq!(replay.order_id, original.order_id);
    assert_eq!(replay.owner_account_id, linked.account_id);
    assert_eq!(replay.owner_name, "Linked market resident");
}

#[test]
fn account_deletion_anonymises_service_replays_kept_by_another_identity() {
    let repository = WorldRepository::new(ServerConfig::default());
    let requester_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("service-replay-requester".to_owned()),
            reset: false,
        })
        .expect("requester guest session")
        .data;
    let requester = repository
        .auth_link(
            &requester_guest.account_token,
            AuthLinkRequest {
                request_id: "service-replay-requester-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "service-replay-requester-subject".to_owned(),
                display_name: Some("Departing craftsperson".to_owned()),
            },
        )
        .expect("requester link")
        .data;
    let observer = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("service-replay-observer".to_owned()),
            reset: false,
        })
        .expect("observer guest session")
        .data;
    let created = repository
        .profession_order(
            &requester.session.account_token,
            ProfessionRequest {
                request_id: "service-replay-create".to_owned(),
                action: ProfessionAction::CreateOrder,
                order_id: None,
                profession: Some(ProfessionKind::Carpenter),
                capability_id: None,
                service: None,
                timing_score: None,
            },
        )
        .expect("service order creation")
        .data;
    let order_id = created.order.expect("created service order").order_id;
    let inspect_request = ProfessionRequest {
        request_id: "service-replay-inspect".to_owned(),
        action: ProfessionAction::Inspect,
        order_id: None,
        profession: None,
        capability_id: None,
        service: None,
        timing_score: None,
    };
    let inspected = repository
        .profession_order(&observer.account_token, inspect_request.clone())
        .expect("service order inspection")
        .data;
    assert!(inspected.professions.orders.iter().any(|order| {
        order.order_id == order_id
            && order.requester_account_id == requester.account_id
            && order.requester_name == "Departing craftsperson"
    }));

    repository
        .account_delete(
            &requester.session.account_token,
            AccountDeletionRequest {
                request_id: "service-replay-requester-delete".to_owned(),
                account_id: requester.account_id,
            },
        )
        .expect("schedule requester deletion");
    repository.tick();

    let replay = repository
        .profession_order(&observer.account_token, inspect_request)
        .expect("service order replay")
        .data;
    let replayed = replay
        .professions
        .orders
        .iter()
        .find(|order| order.order_id == order_id)
        .expect("replayed service order");
    assert_eq!(replayed.requester_account_id, "former-resident");
    assert_eq!(replayed.requester_name, "Former resident");
    assert_eq!(
        replayed.status,
        tarrowyn_protocol::ServiceOrderStatus::Cancelled
    );
    assert!(repository.ops_health().data.ready);
}
