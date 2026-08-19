use super::*;

#[test]
fn response_metadata_is_versioned_and_can_carry_request_and_cursor() {
    let mut meta = ApiMeta::at(42);
    meta.request_id = Some("move-7".to_owned());
    meta.cursor = Some(19);
    let response = ApiResponse {
        meta,
        data: HealthResponse {
            status: "ok".to_owned(),
            service: "tarrowyn-server".to_owned(),
            protocol_version: PROTOCOL_VERSION.to_owned(),
        },
    };

    let encoded = serde_json::to_string(&response).unwrap();
    let decoded: ApiResponse<HealthResponse> = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.meta.protocol_version, PROTOCOL_VERSION);
    assert_eq!(decoded.meta.server_tick, 42);
    assert_eq!(decoded.meta.request_id.as_deref(), Some("move-7"));
    assert_eq!(decoded.meta.cursor, Some(19));
}

#[test]
fn chat_contract_preserves_bounded_message_fields() {
    let request = ChatRequest {
        request_id: "chat-1".to_owned(),
        channel: "settlement".to_owned(),
        text: "Meet at the Hearth".to_owned(),
    };
    let round_trip: ChatRequest =
        serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
    assert_eq!(round_trip, request);
    assert_eq!(MAX_CHAT_MESSAGE_LENGTH, 160);
}

#[test]
fn phase_two_farming_and_trade_requests_round_trip_with_stable_tags() {
    let farming = FarmingRequest {
        request_id: "farm-1".to_owned(),
        action: FarmingAction::Plant,
        position: Position { x: 4, y: 4 },
    };
    let trade = TradeRequest {
        request_id: "trade-1".to_owned(),
        action: TradeAction::Create,
        trade_id: None,
        recipient_account_id: Some("dev-account-2".to_owned()),
        offer: Some(TradeBundle {
            seeds: 1,
            ..TradeBundle::default()
        }),
        request: Some(TradeBundle {
            gold: 2,
            ..TradeBundle::default()
        }),
    };
    let encoded = serde_json::to_string(&(farming, trade)).unwrap();
    assert!(encoded.contains("\"action\":\"plant\""));
    assert!(encoded.contains("\"action\":\"create\""));
    assert_eq!(PROTOCOL_VERSION, "2");
}
