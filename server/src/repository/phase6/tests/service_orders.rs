use super::*;
use tarrowyn_protocol::{
    AuthLinkRequest, GuestSessionRequest, MarketOrderAction, MarketOrderRequest, ProfessionAction,
    ProfessionKind, ProfessionRequest,
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
