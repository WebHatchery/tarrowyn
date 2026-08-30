use super::*;
use tarrowyn_protocol::{
    LawBoundaryResponse, MarketOrder, MarketOrderAction, MarketSnapshot, RouteRecord, RouteStatus,
};

mod account_fixture;
use account_fixture::account_response;
mod account_lifecycle;
mod regional_events;

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
    client.queue_cycle("route-escort");
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Route(request))
            if request.route_id == "north-pack-road"
                && request.action == RouteAction::Escort
    ));

    client.commands.clear();
    client.queue_cycle("route-improve");
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Route(request))
            if request.route_id == "north-pack-road"
                && request.action == RouteAction::Improve
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

    client.commands.clear();
    client
        .region
        .as_mut()
        .expect("regional projection")
        .player_location_id = "saltmere".to_owned();
    client.queue_cycle("travel");
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Travel(request))
            if request.route_id.as_deref() == Some("watch-trail")
                && request.action == TravelAction::Start
    ));

    client.commands.clear();
    client.region.as_mut().expect("regional projection").travel =
        Some(tarrowyn_protocol::TravelState {
            travel_id: "arrived-watch-trail".to_owned(),
            route_id: "watch-trail".to_owned(),
            origin_location_id: "whisperwood-outpost".to_owned(),
            destination_location_id: "saltmere".to_owned(),
            departure_tick: 0,
            eta_tick: 9,
            progress: 100,
            risk_percent: 34,
            status: TravelStatus::Arrived,
            interruption: None,
            recovery_note: None,
        });
    client.queue_cycle("travel");
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Travel(request))
            if request.route_id.as_deref() == Some("watch-trail")
                && request.action == TravelAction::Start
    ));

    client.commands.clear();
    client.region.as_mut().expect("regional projection").travel =
        Some(tarrowyn_protocol::TravelState {
            travel_id: "interrupted-watch-trail".to_owned(),
            route_id: "watch-trail".to_owned(),
            origin_location_id: "whisperwood-outpost".to_owned(),
            destination_location_id: "saltmere".to_owned(),
            departure_tick: 0,
            eta_tick: 9,
            progress: 35,
            risk_percent: 34,
            status: TravelStatus::Interrupted,
            interruption: Some("A fallen marker blocks the trail.".to_owned()),
            recovery_note: None,
        });
    assert_eq!(client.travel_control_details(), ("Travel", false, true));
}

#[test]
fn market_button_waits_for_the_order_destination() {
    let mut client = Phase5Client::new();
    client.own_account_id = Some("account-1".to_owned());
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
        cursor: 0,
    });
    client.market = Some(MarketSnapshot {
        orders: vec![
            MarketOrder {
                order_id: "own-saltmere-seeds".to_owned(),
                owner_account_id: "account-1".to_owned(),
                owner_name: "The traveller".to_owned(),
                origin_location_id: "hearth".to_owned(),
                destination_location_id: "saltmere".to_owned(),
                commodity: tarrowyn_protocol::CommodityKind::Seeds,
                quantity: 1,
                unit_price: 4,
                total_price: 4,
                status: tarrowyn_protocol::MarketOrderStatus::Open,
                created_tick: 1,
                settled_tick: None,
                route_id: "hearth-road".to_owned(),
                fallback_used: false,
            },
            MarketOrder {
                order_id: "other-saltmere-seeds".to_owned(),
                owner_account_id: "account-2".to_owned(),
                owner_name: "A neighbour".to_owned(),
                origin_location_id: "hearth".to_owned(),
                destination_location_id: "saltmere".to_owned(),
                commodity: tarrowyn_protocol::CommodityKind::Seeds,
                quantity: 1,
                unit_price: 4,
                total_price: 4,
                status: tarrowyn_protocol::MarketOrderStatus::Open,
                created_tick: 1,
                settled_tick: None,
                route_id: "hearth-road".to_owned(),
                fallback_used: false,
            },
        ],
        stock_notes: Vec::new(),
        prices: Vec::new(),
        cursor: 1,
    });

    assert!(client.has_open_market_order());
    client.queue_cycle("cancel-market");
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Market(request))
            if request.action == MarketOrderAction::Cancel
                && request.order_id.as_deref() == Some("own-saltmere-seeds")
    ));
    client.commands.clear();

    client.queue_cycle("market-region");
    assert!(client.commands.is_empty());

    client
        .region
        .as_mut()
        .expect("regional projection")
        .player_location_id = "saltmere".to_owned();
    client.queue_cycle("market-region");
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Market(request))
            if request.action == MarketOrderAction::Fulfil
                && request.order_id.as_deref() == Some("other-saltmere-seeds")
    ));
}

#[test]
fn market_button_does_not_fulfil_the_owners_own_order() {
    let mut client = Phase5Client::new();
    client.own_account_id = Some("account-1".to_owned());
    client.region = Some(tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: Vec::new(),
        visible_settlements: Vec::new(),
        player_location_id: "saltmere".to_owned(),
        travel: None,
        interest_radius: 12,
        cursor: 1,
    });
    client.market = Some(MarketSnapshot {
        orders: vec![MarketOrder {
            order_id: "own-saltmere-seeds".to_owned(),
            owner_account_id: "account-1".to_owned(),
            owner_name: "The traveller".to_owned(),
            origin_location_id: "hearth".to_owned(),
            destination_location_id: "saltmere".to_owned(),
            commodity: tarrowyn_protocol::CommodityKind::Seeds,
            quantity: 1,
            unit_price: 4,
            total_price: 4,
            status: tarrowyn_protocol::MarketOrderStatus::Open,
            created_tick: 1,
            settled_tick: None,
            route_id: "hearth-road".to_owned(),
            fallback_used: false,
        }],
        stock_notes: Vec::new(),
        prices: Vec::new(),
        cursor: 1,
    });

    client.queue_cycle("market-region");

    assert!(client.commands.is_empty());
}

#[test]
fn route_repair_can_select_a_closed_route_for_recovery() {
    let mut client = Phase5Client::new();
    client.region = Some(tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: vec![RouteRecord {
            route_id: "closed-road".to_owned(),
            name: "Closed Road".to_owned(),
            origin_location_id: "hearth".to_owned(),
            destination_location_id: "saltmere".to_owned(),
            transport: "caravan".to_owned(),
            length: 4,
            risk_percent: 60,
            condition: 20,
            capacity: 1,
            travel_ticks: 4,
            repair_cost: 8,
            status: RouteStatus::Closed,
            last_action_tick: 0,
            note: "The road is closed until a repair crew arrives.".to_owned(),
        }],
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
            if request.route_id == "closed-road" && request.action == RouteAction::Repair
    ));
}

#[test]
fn regional_cycle_reports_when_no_projection_action_is_ready() {
    let mut client = Phase5Client::new();

    assert!(!client.queue_cycle("travel"));
    assert!(client.commands.is_empty());
}

#[test]
fn travel_control_waits_for_a_route_at_the_current_location() {
    let mut client = Phase5Client::new();
    client.region = Some(tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: vec![RouteRecord {
            route_id: "saltmere-watch-trail".to_owned(),
            name: "Saltmere Watch Trail".to_owned(),
            origin_location_id: "saltmere".to_owned(),
            destination_location_id: "whisperwood-outpost".to_owned(),
            transport: "pack route".to_owned(),
            length: 9,
            risk_percent: 34,
            condition: 55,
            capacity: 1,
            travel_ticks: 9,
            repair_cost: 5,
            status: RouteStatus::Operational,
            last_action_tick: 0,
            note: "The trail is open beyond the Hearth.".to_owned(),
        }],
        visible_settlements: Vec::new(),
        player_location_id: "hearth".to_owned(),
        travel: None,
        interest_radius: 12,
        cursor: 0,
    });

    assert_eq!(client.travel_control_details(), ("Travel", false, false));
}

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
fn market_success_notice_describes_the_requested_action() {
    assert_eq!(
        super::market_success_message(Some(MarketOrderAction::Create), false),
        "The shipment is on the regional ledger."
    );
    assert_eq!(
        super::market_success_message(Some(MarketOrderAction::Fulfil), false),
        "The shipment reached its destination and settled."
    );
    assert_eq!(
        super::market_success_message(Some(MarketOrderAction::Cancel), false),
        "The shipment was cancelled and its escrow returned."
    );
    assert_eq!(
        super::market_success_message(Some(MarketOrderAction::Cancel), true),
        "The fallback shipment was cancelled; no player goods were escrowed."
    );
}

#[test]
fn market_result_message_names_the_shipment_details() {
    let order = MarketOrder {
        order_id: "seed-shipment".to_owned(),
        owner_account_id: "account-1".to_owned(),
        owner_name: "The traveller".to_owned(),
        origin_location_id: "hearth".to_owned(),
        destination_location_id: "saltmere".to_owned(),
        commodity: tarrowyn_protocol::CommodityKind::Seeds,
        quantity: 1,
        unit_price: 4,
        total_price: 4,
        status: tarrowyn_protocol::MarketOrderStatus::Open,
        created_tick: 2,
        settled_tick: None,
        route_id: "north-pack-road".to_owned(),
        fallback_used: false,
    };

    assert_eq!(
        super::market_result_message(Some(MarketOrderAction::Create), false, Some(&order)),
        "The shipment is on the regional ledger. Details: 1 seed from hearth to saltmere • 4 gold."
    );
}

#[test]
fn travel_success_message_explains_progress_and_risk() {
    let travel = tarrowyn_protocol::TravelState {
        travel_id: "travel-1".to_owned(),
        route_id: "north-pack-road".to_owned(),
        origin_location_id: "hearth".to_owned(),
        destination_location_id: "saltmere".to_owned(),
        departure_tick: 4,
        eta_tick: 8,
        progress: 50,
        risk_percent: 18,
        status: TravelStatus::Travelling,
        interruption: None,
        recovery_note: None,
    };

    assert_eq!(
        super::commands::travel_success_message(Some(&travel), "hearth"),
        "Journey underway to saltmere • 50% complete • 18% risk."
    );
}

#[test]
fn travel_success_message_names_arrival_and_recovery_control() {
    let mut travel = tarrowyn_protocol::TravelState {
        travel_id: "travel-1".to_owned(),
        route_id: "north-pack-road".to_owned(),
        origin_location_id: "hearth".to_owned(),
        destination_location_id: "saltmere".to_owned(),
        departure_tick: 4,
        eta_tick: 8,
        progress: 50,
        risk_percent: 18,
        status: TravelStatus::Interrupted,
        interruption: Some("A fallen marker blocks the trail.".to_owned()),
        recovery_note: None,
    };
    assert_eq!(
        super::commands::travel_success_message(Some(&travel), "hearth"),
        "Journey interrupted before saltmere; tap Recover to continue."
    );

    travel.status = TravelStatus::Arrived;
    travel.progress = 100;
    assert_eq!(
        super::commands::travel_success_message(Some(&travel), "saltmere"),
        "Arrived at saltmere."
    );
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
