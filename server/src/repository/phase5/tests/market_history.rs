use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    CommodityKind, GuestSessionRequest, MarketOrderAction, MarketOrderRequest,
};

#[test]
fn accepted_market_order_history_reaches_both_route_endpoints() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase5-market-history".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    let order = repository
        .market_order(
            &session.account_token,
            MarketOrderRequest {
                request_id: "market-history-create".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("whisperwood-outpost".to_owned()),
                commodity: Some(CommodityKind::Seeds),
                quantity: Some(1),
            },
        )
        .expect("market order")
        .data;
    assert!(order.accepted);

    let state = repository.state.lock().unwrap();
    for location_id in ["hearth", "whisperwood-outpost"] {
        let settlement = state
            .phase5
            .settlements
            .iter()
            .find(|settlement| settlement.location_id == location_id)
            .expect("route endpoint settlement");
        assert!(settlement
            .chronicle
            .iter()
            .any(|entry| entry.kind == "regional market"));
    }
}
