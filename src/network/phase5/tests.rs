use super::*;
use tarrowyn_protocol::{
    LawBoundaryResponse, MarketOrder, MarketOrderAction, MarketSnapshot, RouteRecord, RouteStatus,
};

mod account_fixture;
use account_fixture::account_response;
mod account_lifecycle;
mod feedback;
mod location_sync;
mod market_controls;
mod movement_controls;
mod regional_commands;
mod regional_events;
mod session_dispatch;

#[test]
fn older_regional_projection_cannot_replace_newer_command_state() {
    let mut cursor = 0;

    assert!(super::accept_projection_cursor(&mut cursor, Some(12)));
    assert!(!super::accept_projection_cursor(&mut cursor, Some(11)));
    assert!(super::accept_projection_cursor(&mut cursor, Some(12)));
}

#[test]
fn account_and_law_reads_share_the_regional_cursor_boundary() {
    let mut cursor = 0;

    assert!(super::accept_projection_cursor(&mut cursor, Some(16)));
    assert!(!super::accept_projection_cursor(&mut cursor, Some(15)));
    assert_eq!(cursor, 16);
}

#[test]
fn account_summary_names_the_character_boundary_without_session_secrets() {
    let mut client = Phase5Client::new();
    client.account = Some(account_response(false));

    let summary = client.account_details();

    assert!(summary.contains("Identity: Linked traveller"));
    assert!(summary.contains("Provider: webhatchery-identity-oidc"));
    assert!(summary.contains("Account ID: account-1"));
    assert!(summary.contains("Character ID: character-1"));
    assert!(summary.contains("Session valid through beat 100"));
    assert!(!summary.contains("refresh-secret"));
    assert!(!summary.contains("account_token"));
    assert!(client
        .summary()
        .contains("Linked account • tap Account for details or Logout to leave safely"));
}

#[test]
fn guest_summary_uses_player_language_for_the_linking_path() {
    let mut client = Phase5Client::new();
    client.account = Some(account_response(true));

    let summary = client.summary();

    assert!(summary.contains("Guest account • tap Account to link"));
    assert!(!summary.contains("Guest fixture"));
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
            fallback_used: true,
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
    assert!(economy.contains("1 fallback"));
    assert!(economy.contains("Protected"));
    assert!(economy.contains("Event escalation"));
    assert!(economy.contains("Service travelling"));
    let inspection = client.inspection();
    assert!(inspection.contains("North Pack Road Operational"));
    assert!(inspection.contains("Saltmere Ferry Threatened"));
    assert!(inspection.contains("Seeds are available at the Hearth."));
    assert!(inspection.contains("Seeds 104%"));
    assert!(inspection.contains("fallback 1"));
    assert!(inspection.contains("A hard thaw"));
    assert!(inspection.contains("repair ferry markers"));
    assert!(inspection.contains("chosen: none"));
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
