use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    CommodityKind, GuestSessionRequest, MarketOrderAction, MarketOrderRequest,
};

#[test]
fn operational_metrics_count_open_fallback_market_orders() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("fallback-metrics-operator".to_owned()),
            reset: false,
        })
        .expect("operator session")
        .data;
    let player = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("fallback-metrics-player".to_owned()),
            reset: false,
        })
        .expect("player session")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .identities
            .get_mut(&player.client_key)
            .expect("player identity")
            .inventory
            .seeds = 0;
    }

    let order = repository
        .market_order(
            &player.account_token,
            MarketOrderRequest {
                request_id: "fallback-metrics-order".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("saltmere".to_owned()),
                commodity: Some(CommodityKind::Seeds),
                quantity: Some(1),
            },
        )
        .expect("fallback market order")
        .data;
    assert!(order.accepted);
    assert!(
        order
            .order
            .expect("fallback order projection")
            .fallback_used
    );

    let metrics = repository
        .ops_metrics(&operator.account_token)
        .expect("operator metrics")
        .data;
    assert_eq!(metrics.open_market_fallback_orders, 1);
}
