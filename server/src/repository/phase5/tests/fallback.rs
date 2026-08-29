use super::super::super::WorldRepository;
use super::guest;
use crate::ServerConfig;
use tarrowyn_protocol::{CommodityKind, MarketOrderAction, MarketOrderRequest, Position};

fn create_request(request_id: &str) -> MarketOrderRequest {
    MarketOrderRequest {
        request_id: request_id.to_owned(),
        action: MarketOrderAction::Create,
        order_id: None,
        destination_location_id: Some("saltmere".to_owned()),
        commodity: Some(CommodityKind::Seeds),
        quantity: Some(1),
    }
}

#[test]
fn travelling_fallback_is_bounded_delayed_and_not_refunded() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase5-fallback");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let identity = state
            .identities
            .get_mut(&session.client_key)
            .expect("identity exists");
        identity.inventory.seeds = 0;
    }

    let created = repository
        .market_order(&session.account_token, create_request("fallback-first"))
        .expect("fallback order")
        .data;
    let order = created.order.expect("fallback order projection");
    assert!(created.accepted);
    assert!(order.fallback_used);

    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .identities
            .get_mut(&session.client_key)
            .expect("identity exists")
            .position = Position { x: 3, y: 9 };
    }
    let early = repository
        .market_order(
            &session.account_token,
            MarketOrderRequest {
                request_id: "fallback-early".to_owned(),
                action: MarketOrderAction::Fulfil,
                order_id: Some(order.order_id.clone()),
                destination_location_id: None,
                commodity: None,
                quantity: None,
            },
        )
        .expect("early fulfilment response")
        .data;
    assert!(!early.accepted);
    assert!(early
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("more time")));

    repository.tick();
    repository.tick();
    let fulfilled = repository
        .market_order(
            &session.account_token,
            MarketOrderRequest {
                request_id: "fallback-fulfil".to_owned(),
                action: MarketOrderAction::Fulfil,
                order_id: Some(order.order_id),
                destination_location_id: None,
                commodity: None,
                quantity: None,
            },
        )
        .expect("delayed fallback fulfilment")
        .data;
    assert!(fulfilled.accepted);

    {
        let mut state = repository.state.lock().expect("repository lock");
        let identity = state
            .identities
            .get_mut(&session.client_key)
            .expect("identity exists");
        identity.inventory.seeds = 0;
        identity.position = Position { x: 8, y: 6 };
    }
    let second = repository
        .market_order(&session.account_token, create_request("fallback-second"))
        .expect("second fallback order")
        .data;
    let second_order = second.order.expect("second order");
    assert!(second.accepted);
    assert!(second_order.fallback_used);

    let cancelled = repository
        .market_order(
            &session.account_token,
            MarketOrderRequest {
                request_id: "fallback-cancel".to_owned(),
                action: MarketOrderAction::Cancel,
                order_id: Some(second_order.order_id),
                destination_location_id: None,
                commodity: None,
                quantity: None,
            },
        )
        .expect("fallback cancellation response")
        .data;
    assert!(cancelled.accepted);
    assert_eq!(
        cancelled.order.expect("cancelled order").status,
        tarrowyn_protocol::MarketOrderStatus::Cancelled
    );
    let state = repository.state.lock().expect("repository lock");
    assert_eq!(state.identities[&session.client_key].inventory.seeds, 0);
    drop(state);

    let blocked = repository
        .market_order(&session.account_token, create_request("fallback-blocked"))
        .expect("fallback capacity response")
        .data;
    assert!(!blocked.accepted);
    assert!(blocked
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("limited travelling fallback")));
}
