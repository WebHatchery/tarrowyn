use super::*;
use tarrowyn_protocol::{
    AuthLinkRequest, GuestSessionRequest, ProfessionAction, ProfessionKind, ProfessionRequest,
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
