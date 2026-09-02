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
fn response_deserialization_rejects_a_deployment_protocol_mismatch() {
    let json = serde_json::json!({
        "meta": {
            "protocol_version": "5",
            "server_tick": 12
        },
        "data": {
            "status": "ok",
            "service": "tarrowyn-server",
            "protocol_version": "5"
        }
    });
    let error = serde_json::from_value::<ApiResponse<HealthResponse>>(json)
        .expect_err("a mismatched deployment response must fail closed")
        .to_string();
    assert!(error.contains("unsupported Tarrowyn protocol version `5`"));
}

#[test]
fn operations_health_defaults_new_integrity_details_for_older_payloads() {
    let json = serde_json::json!({
        "status": "ok",
        "ready": true,
        "storage_version": 6,
        "protocol_version": "7",
        "last_backup_tick": null,
        "last_backup_path": null,
        "integrity_ok": true,
        "persistence_error": null,
        "backup_error": null,
        "maintenance_message": null
    });
    let health: OpsHealthResponse = serde_json::from_value(json).unwrap();
    assert!(health.integrity_ok);
    assert!(health.integrity_failures.is_empty());
}

#[test]
fn operations_metrics_default_new_pool_details_for_older_payloads() {
    let json = serde_json::json!({
        "server_tick": 12,
        "connected_sessions": 1,
        "accounts": 1,
        "region_entities_visible": 3,
        "event_cursor": 4,
        "regional_event_backlog": 0,
        "open_market_orders": 0,
        "travelling_players": 0,
        "rejected_commands": 0,
        "completed_commands": 0,
        "average_tick_ms": 1,
        "last_tick_ms": 1,
        "tick_drift_count": 0,
        "average_price_index_percent": 100,
        "scarce_goods_count": 0,
        "npc_fallback_households": 1,
        "abandoned_claims": 0,
        "declining_settlements": 0,
        "newcomer_access": true,
        "alert_flags": []
    });
    let metrics: OpsMetricsResponse = serde_json::from_value(json).unwrap();
    assert_eq!(metrics.http_request_workers, 0);
    assert_eq!(metrics.http_request_queue_capacity, 0);
    assert_eq!(metrics.http_active_requests, 0);
    assert_eq!(metrics.http_queue_depth, 0);
    assert_eq!(metrics.http_queue_peak, 0);
    assert_eq!(metrics.http_queue_full_events, 0);
    assert_eq!(metrics.mysql_pool_max_connections, 0);
}

#[test]
fn position_distance_saturates_malformed_coordinates() {
    assert_eq!(
        Position {
            x: i32::MIN,
            y: i32::MIN,
        }
        .manhattan_distance(Position {
            x: i32::MAX,
            y: i32::MAX,
        }),
        u32::MAX
    );
}

#[test]
fn inventory_total_saturates_malformed_quantities() {
    let inventory = Inventory {
        wheat: u32::MAX,
        turnips: u32::MAX,
        moonberries: u32::MAX,
        seeds: u32::MAX,
        bandages: u32::MAX,
        timber: u32::MAX,
        stone: u32::MAX,
        iron_ore: u32::MAX,
    };

    assert_eq!(inventory.total_items(), u32::MAX);
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
    assert!(serde_json::to_string(&FarmingAction::TendAnimal)
        .unwrap()
        .contains("tend_animal"));
    assert!(encoded.contains("\"action\":\"create\""));
    assert_eq!(PROTOCOL_VERSION, "7");
}

#[test]
fn phase_three_frontier_commands_round_trip_with_stable_wire_names() {
    let combat = CombatRequest {
        request_id: "combat-1".to_owned(),
        action: CombatAction::Strike,
        weapon: WeaponKind::ImprovisedClub,
    };
    let expedition = ExpeditionRequest {
        request_id: "expedition-1".to_owned(),
        action: ExpeditionAction::Supply,
        expedition_id: Some("pioneer-1".to_owned()),
        role: Some(ExpeditionRole::Builder),
        food: 6,
        tools: 3,
        materials: 8,
        safety: 3,
        outpost_name: Some("Lantern Rest".to_owned()),
    };
    let encoded = serde_json::to_string(&(combat, expedition)).unwrap();
    assert!(encoded.contains("\"weapon\":\"improvised_club\""));
    assert!(encoded.contains("\"action\":\"supply\""));
    assert!(encoded.contains("\"role\":\"builder\""));
}

#[test]
fn local_combat_state_defaults_the_action_window_for_older_snapshots() {
    let json = serde_json::json!({
        "encounter_id": "whisperwood-local-1",
        "enemy_name": "Brambleback scout",
        "enemy_health": 3,
        "player_health": 2,
        "turn": 0,
        "status": "ready",
        "weapon": "iron_sword",
        "injury_limit": 3,
        "stored_property_safe": true,
        "carried_risk": "A seed may be risked.",
        "recovery_cost": 4
    });
    let state: LocalCombatState = serde_json::from_value(json).unwrap();
    assert_eq!(state.action_available_at_tick, 0);
}

#[test]
fn shared_clock_names_the_four_day_periods_at_stable_boundaries() {
    let mut clock = WorldClock {
        day: 1,
        seconds: 0.0,
        day_length_seconds: 2400.0,
    };
    assert_eq!(clock.time_of_day(), TimeOfDay::Morning);

    clock.seconds = 600.0;
    assert_eq!(clock.time_of_day(), TimeOfDay::Afternoon);
    clock.seconds = 1100.0;
    assert_eq!(clock.time_of_day(), TimeOfDay::Evening);
    clock.seconds = 1600.0;
    assert_eq!(clock.time_of_day(), TimeOfDay::Night);
    assert!(TimeOfDay::Night.is_night());
    assert!(!TimeOfDay::Morning.is_night());
}
