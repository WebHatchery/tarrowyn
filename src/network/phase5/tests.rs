use super::*;
use tarrowyn_protocol::{
    LawBoundaryResponse, MarketOrder, MarketSnapshot, RouteRecord, RouteStatus,
};

#[test]
fn refresh_is_scheduled_before_a_production_session_expires() {
    assert_eq!(refresh_delay(0), 1.0);
    assert_eq!(refresh_delay(20), 15.0);
}

#[test]
fn clear_drops_cached_regional_projections() {
    let mut client = Phase5Client::new();
    client.region = Some(tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: Vec::new(),
        visible_settlements: Vec::new(),
        player_location_id: "hearth".to_owned(),
        travel: None,
        interest_radius: 12,
        cursor: 1,
    });
    client.settlements = Some(tarrowyn_protocol::SettlementsResponse {
        settlements: Vec::new(),
        cursor: 1,
    });
    client.households = Some(tarrowyn_protocol::RegionalHouseholdsResponse {
        households: Vec::new(),
        vacancies: Vec::new(),
        cursor: 1,
    });
    client.market = Some(MarketSnapshot {
        orders: Vec::new(),
        stock_notes: Vec::new(),
        prices: Vec::new(),
        cursor: 1,
    });
    client.events = Some(tarrowyn_protocol::RegionalEventsResponse {
        events: Vec::new(),
        cursor: 1,
    });
    client.law = Some(LawBoundaryResponse {
        pvp_enabled: false,
        theft_enabled: false,
        claims_protected: true,
        trade_protected: true,
        travel_protected: true,
        recovery_path: "Protected recovery".to_owned(),
        policy_version: "phase5-no-pvp-1".to_owned(),
    });

    client.clear();

    assert!(client.region.is_none());
    assert!(client.settlements.is_none());
    assert!(client.households.is_none());
    assert!(client.market.is_none());
    assert!(client.events.is_none());
    assert!(client.law.is_none());
}

#[test]
fn route_repair_button_queues_an_authoritative_repair() {
    let mut client = Phase5Client::new();
    client.region = Some(tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: vec![
            RouteRecord {
                route_id: "north-pack-road".to_owned(),
                name: "North Pack Road".to_owned(),
                origin_location_id: "hearth".to_owned(),
                destination_location_id: "whisperwood-outpost".to_owned(),
                transport: "pack road".to_owned(),
                length: 6,
                risk_percent: 44,
                condition: 42,
                capacity: 3,
                travel_ticks: 6,
                repair_cost: 4,
                status: RouteStatus::Threatened,
                last_action_tick: 0,
                note: "The road needs a repair crew.".to_owned(),
            },
            RouteRecord {
                route_id: "watch-trail".to_owned(),
                name: "Watch Trail".to_owned(),
                origin_location_id: "whisperwood-outpost".to_owned(),
                destination_location_id: "saltmere".to_owned(),
                transport: "pack route".to_owned(),
                length: 9,
                risk_percent: 34,
                condition: 55,
                capacity: 1,
                travel_ticks: 9,
                repair_cost: 5,
                status: RouteStatus::Delayed,
                last_action_tick: 0,
                note: "The trail needs a repair crew.".to_owned(),
            },
        ],
        visible_settlements: Vec::new(),
        player_location_id: "hearth".to_owned(),
        travel: None,
        interest_radius: 12,
        cursor: 0,
    });

    client.queue_cycle("route-repair");

    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Route(request))
            if request.route_id == "north-pack-road"
                && request.action == RouteAction::Repair
    ));

    client.commands.clear();
    client
        .region
        .as_mut()
        .expect("regional projection")
        .player_location_id = "whisperwood-outpost".to_owned();
    client.queue_cycle("route-repair");
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Route(request))
            if request.route_id == "watch-trail"
                && request.action == RouteAction::Repair
    ));
}

#[test]
fn linked_production_session_replaces_the_guest_projection() {
    let mut client = Phase5Client::new();
    client.linked_account = Some(AuthLinkResponse {
        request_id: "link".to_owned(),
        provider: "webhatchery-identity-oidc".to_owned(),
        account_id: "account-1".to_owned(),
        character_id: "dev-character-1".to_owned(),
        display_name: "Linked traveller".to_owned(),
        session: tarrowyn_protocol::AuthSession {
            account_token: "prod-session-1".to_owned(),
            refresh_token: "prod-refresh-1".to_owned(),
            expires_in_seconds: 900,
            expires_at_tick: 3600,
        },
        linked_guest: true,
    });

    let account = client.take_linked_account(Some("guest-key")).unwrap();
    assert_eq!(account.client_key, "guest-key");
    assert_eq!(account.account_id, "account-1");
    assert_eq!(account.display_name, "Linked traveller");
    assert_eq!(account.account_token, "prod-session-1");
    assert!(client.take_linked_account(Some("guest-key")).is_none());
}

#[test]
fn logout_signal_is_consumed_once() {
    let mut client = Phase5Client::new();
    client.logged_out = true;
    client.refresh_token = Some("refresh-secret".to_owned());
    client.refreshed_session = Some(tarrowyn_protocol::AuthSession {
        account_token: "access".to_owned(),
        refresh_token: "refresh".to_owned(),
        expires_in_seconds: 10,
        expires_at_tick: 10,
    });
    client.clear();
    assert!(client.refresh_token.is_none());
    assert!(client.refreshed_session.is_none());
    client.logged_out = true;
    assert!(client.take_logged_out());
    assert!(!client.take_logged_out());
}

#[test]
fn account_deletion_requires_two_taps_for_a_linked_account() {
    let mut client = Phase5Client::new();
    client.account = Some(account_response(false));

    client.queue_cycle("delete-account");
    assert!(client.deletion_armed);
    assert!(client.commands.is_empty());

    client.queue_cycle("delete-account");
    assert!(!client.deletion_armed);
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Delete(request)) if request.account_id == "account-1"
    ));
}

#[test]
fn guest_account_cannot_arm_deletion() {
    let mut client = Phase5Client::new();
    client.account = Some(account_response(true));

    client.queue_cycle("delete-account");
    assert!(!client.deletion_armed);
    assert!(client.commands.is_empty());
}

#[test]
fn account_deletion_response_selects_its_own_command_variant() {
    let response = serde_json::from_value::<Phase5CommandResponse>(serde_json::json!({
        "request_id": "delete-1",
        "account_id": "account-1",
        "character_id": "character-1",
        "accepted": true,
        "status": "scheduled",
        "reason": null
    }))
    .expect("account deletion response should decode");
    assert!(matches!(response, Phase5CommandResponse::Delete(_)));
}

#[test]
fn regional_event_cursor_merges_updates_without_dropping_known_events() {
    let mut current = Some(RegionalEventsResponse {
        events: vec![regional_event(
            "event-1",
            tarrowyn_protocol::RegionalEventStage::Signal,
            4,
        )],
        cursor: 4,
    });
    merge_regional_events(
        &mut current,
        RegionalEventsResponse {
            events: vec![
                regional_event(
                    "event-1",
                    tarrowyn_protocol::RegionalEventStage::Escalation,
                    7,
                ),
                regional_event("event-2", tarrowyn_protocol::RegionalEventStage::Signal, 8),
            ],
            cursor: 8,
        },
    );

    let current = current.expect("regional events should remain cached");
    assert_eq!(current.cursor, 8);
    assert_eq!(current.events.len(), 2);
    assert_eq!(current.events[0].event_id, "event-1");
    assert_eq!(
        current.events[0].stage,
        tarrowyn_protocol::RegionalEventStage::Escalation
    );
    assert_eq!(current.events[1].event_id, "event-2");
}

#[test]
fn regional_cursor_reset_discards_stale_events_and_restarts_refresh() {
    let mut client = Phase5Client::new();
    client.events = Some(RegionalEventsResponse {
        events: vec![regional_event(
            "event-1",
            tarrowyn_protocol::RegionalEventStage::Aftermath,
            9,
        )],
        cursor: 9,
    });
    client.refresh_timer = 3.0;

    client.reset_event_cursor();

    assert!(client.events.is_none());
    assert_eq!(client.refresh_timer, 0.0);
}

#[test]
fn regional_summary_shows_local_condition_and_recovery_signal() {
    let mut client = Phase5Client::new();
    client.region = Some(tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: vec![
            RouteRecord {
                route_id: "north-pack-road".to_owned(),
                name: "North Pack Road".to_owned(),
                origin_location_id: "saltmere".to_owned(),
                destination_location_id: "hearth".to_owned(),
                transport: "pack road".to_owned(),
                length: 6,
                risk_percent: 18,
                condition: 72,
                capacity: 8,
                travel_ticks: 4,
                repair_cost: 3,
                status: RouteStatus::Operational,
                last_action_tick: 0,
                note: "The road is open to carts.".to_owned(),
            },
            RouteRecord {
                route_id: "saltmere-ferry".to_owned(),
                name: "Saltmere Ferry".to_owned(),
                origin_location_id: "saltmere".to_owned(),
                destination_location_id: "whisperwood-outpost".to_owned(),
                transport: "ferry".to_owned(),
                length: 8,
                risk_percent: 44,
                condition: 42,
                capacity: 4,
                travel_ticks: 5,
                repair_cost: 5,
                status: RouteStatus::Threatened,
                last_action_tick: 2,
                note: "The crossing needs a watch.".to_owned(),
            },
        ],
        visible_settlements: Vec::new(),
        player_location_id: "saltmere".to_owned(),
        travel: None,
        interest_radius: 12,
        cursor: 7,
    });
    client.households = Some(tarrowyn_protocol::RegionalHouseholdsResponse {
        households: vec![tarrowyn_protocol::RegionalHousehold {
            household_id: "household-maren-region".to_owned(),
            household_name: "The Maren household".to_owned(),
            origin_location_id: "hearth".to_owned(),
            destination_location_id: Some("saltmere".to_owned()),
            status: "travelling".to_owned(),
            reason: "Saltmere needs a carrier service.".to_owned(),
            service: "carrier".to_owned(),
            departure_tick: Some(5),
            arrival_tick: None,
            history: vec!["The household chose the open route.".to_owned()],
        }],
        vacancies: vec!["ferry hand".to_owned()],
        cursor: 7,
    });
    client.settlements = Some(tarrowyn_protocol::SettlementsResponse {
        settlements: vec![
            tarrowyn_protocol::SettlementProjection {
                settlement_id: "saltmere-settlement".to_owned(),
                name: "Saltmere Landing".to_owned(),
                location_id: "saltmere".to_owned(),
                population: 18,
                food: 40,
                safety: 42,
                infrastructure: 45,
                industry: 38,
                governance: 40,
                player_activity: 10,
                claim_count: 2,
                available_plot_count: 3,
                public_works: vec!["Quay".to_owned(), "Hall".to_owned()],
                condition: tarrowyn_protocol::SettlementCondition::Strained,
                milestones: Vec::new(),
                vacancies: vec!["ferry hand".to_owned()],
                demand: vec!["timber".to_owned()],
                abundant_goods: Vec::new(),
                scarce_goods: Vec::new(),
                price_index_percent: 120,
                chronicle: Vec::new(),
                recovery_opportunity: Some("Repair the ferry route.".to_owned()),
            },
            tarrowyn_protocol::SettlementProjection {
                settlement_id: "whisperwood-settlement".to_owned(),
                name: "Whisperwood Watch".to_owned(),
                location_id: "whisperwood-outpost".to_owned(),
                population: 8,
                food: 42,
                safety: 36,
                infrastructure: 58,
                industry: 74,
                governance: 42,
                player_activity: 14,
                claim_count: 0,
                available_plot_count: 0,
                public_works: vec!["Watchtower".to_owned()],
                condition: tarrowyn_protocol::SettlementCondition::Quiet,
                milestones: Vec::new(),
                vacancies: vec!["bridge warden".to_owned(), "healer".to_owned()],
                demand: vec!["food".to_owned()],
                abundant_goods: Vec::new(),
                scarce_goods: Vec::new(),
                price_index_percent: 140,
                chronicle: vec![tarrowyn_protocol::ChronicleEntry {
                    event_id: "settlement-history-1".to_owned(),
                    kind: "settlement".to_owned(),
                    title: "The watchtower keeps its lamp".to_owned(),
                    text: "The outpost remembers its first wardens.".to_owned(),
                    created_tick: 3,
                    cursor: 3,
                }],
                recovery_opportunity: Some("Bring food to the watch.".to_owned()),
            },
        ],
        cursor: 7,
    });
    client.market = Some(MarketSnapshot {
        orders: vec![MarketOrder {
            order_id: "order-1".to_owned(),
            owner_account_id: "account-1".to_owned(),
            owner_name: "Linked traveller".to_owned(),
            origin_location_id: "hearth".to_owned(),
            destination_location_id: "saltmere".to_owned(),
            commodity: tarrowyn_protocol::CommodityKind::Seeds,
            quantity: 2,
            unit_price: 4,
            total_price: 8,
            status: tarrowyn_protocol::MarketOrderStatus::Open,
            created_tick: 3,
            settled_tick: None,
            route_id: "north-pack-road".to_owned(),
            fallback_used: false,
        }],
        stock_notes: vec!["Seeds are available at the Hearth.".to_owned()],
        prices: vec!["Seeds 104%".to_owned()],
        cursor: 7,
    });
    client.law = Some(LawBoundaryResponse {
        pvp_enabled: false,
        theft_enabled: false,
        claims_protected: true,
        trade_protected: true,
        travel_protected: true,
        recovery_path: "Protected recovery".to_owned(),
        policy_version: "phase5-no-pvp-1".to_owned(),
    });
    client.events = Some(RegionalEventsResponse {
        events: vec![regional_event(
            "event-1",
            tarrowyn_protocol::RegionalEventStage::Escalation,
            7,
        )],
        cursor: 7,
    });

    assert_eq!(client.season(), Some("thaw"));
    let first_line = client.summary().lines().next().unwrap().to_owned();
    assert!(first_line.contains("saltmere"));
    assert!(first_line.contains("Strained"));
    assert!(first_line.contains("recovery open"));
    assert!(first_line.contains("2 claims"));
    assert!(first_line.contains("3 free plots"));
    assert!(first_line.contains("2 works"));
    let comparison = client.summary().lines().nth(1).unwrap().to_owned();
    assert!(comparison.contains("Saltmere Landing Strained • 1 opening"));
    assert!(comparison.contains("Whisperwood Watch Quiet • 2 openings"));
    let economy = client.summary().lines().nth(2).unwrap().to_owned();
    assert!(economy.contains("Roads 2/2 • risk 1"));
    assert!(economy.contains("Orders 1"));
    assert!(economy.contains("Protected"));
    assert!(economy.contains("Event escalation"));
    assert!(economy.contains("Service travelling"));
}

fn regional_event(
    event_id: &str,
    stage: tarrowyn_protocol::RegionalEventStage,
    cursor: u64,
) -> tarrowyn_protocol::RegionalEvent {
    tarrowyn_protocol::RegionalEvent {
        event_id: event_id.to_owned(),
        title: "The thaw road".to_owned(),
        kind: "weather".to_owned(),
        stage,
        affected_location_ids: vec!["hearth".to_owned()],
        effects: vec!["supply".to_owned()],
        cause: "A hard thaw".to_owned(),
        intervention_options: vec!["repair ferry markers".to_owned()],
        chosen_intervention: None,
        outcome: None,
        started_tick: 1,
        updated_tick: 2,
        cursor,
    }
}

fn account_response(guest_fixture: bool) -> tarrowyn_protocol::AccountResponse {
    let account_id = if guest_fixture {
        "guest-1"
    } else {
        "account-1"
    };
    let character_id = if guest_fixture {
        "guest-character-1"
    } else {
        "character-1"
    };
    let display_name = if guest_fixture {
        "Guest"
    } else {
        "Linked traveller"
    };
    tarrowyn_protocol::AccountResponse {
        account_id: account_id.to_owned(),
        provider: if guest_fixture {
            "development-guest"
        } else {
            "webhatchery-identity-oidc"
        }
        .to_owned(),
        character_id: character_id.to_owned(),
        display_name: display_name.to_owned(),
        guest_fixture,
        privacy_policy_version: "2026-01".to_owned(),
        retention_note: "retained until deletion".to_owned(),
        session_expires_at_tick: 100,
        character: tarrowyn_protocol::PlayerProjection {
            account_id: account_id.to_owned(),
            character_id: character_id.to_owned(),
            display_name: display_name.to_owned(),
            position: tarrowyn_protocol::Position { x: 8, y: 6 },
            gold: 10,
            field_tool_condition: 20,
            field_weather: tarrowyn_protocol::FieldWeather::Clear,
            field_pest_pressure: 0,
            animal_condition: 10,
            animal_max_condition: 10,
            skill: 1,
            reputation: 0,
            adventurer_rank: tarrowyn_protocol::AdventurerRank::Unproven,
            adventurer_credentials: Vec::new(),
            inventory: tarrowyn_protocol::Inventory::default(),
            weapon: tarrowyn_protocol::WeaponKind::IronSword,
            knocked_out: false,
            injuries: 0,
            recovery_cost: 0,
        },
    }
}
