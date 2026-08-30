use super::super::*;
use tarrowyn_protocol::{LawBoundaryResponse, MarketSnapshot, RouteRecord, RouteStatus};

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
    assert!(client.route_command_pending());

    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Route(request))
            if request.route_id == "north-pack-road"
                && request.action == RouteAction::Repair
    ));

    client.commands.clear();
    assert!(!client.route_command_pending());
    client.queue_cycle("route-escort");
    assert!(client.route_command_pending());
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Route(request))
            if request.route_id == "north-pack-road"
                && request.action == RouteAction::Escort
    ));

    client.commands.clear();
    assert!(!client.route_command_pending());
    client.queue_cycle("route-improve");
    assert!(client.route_command_pending());
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Route(request))
            if request.route_id == "north-pack-road"
                && request.action == RouteAction::Improve
    ));

    client.commands.clear();
    assert!(!client.route_command_pending());
    client
        .region
        .as_mut()
        .expect("regional projection")
        .player_location_id = "whisperwood-outpost".to_owned();
    client.queue_cycle("route-repair");
    assert!(client.route_command_pending());
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
    assert!(client.travel_command_pending());
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Travel(request))
            if request.route_id.as_deref() == Some("watch-trail")
                && request.action == TravelAction::Start
    ));

    client.commands.clear();
    assert!(!client.travel_command_pending());
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
