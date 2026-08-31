use super::*;

#[test]
fn route_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = Phase5Client::new();
    let request = tarrowyn_protocol::RouteRequest {
        request_id: "route-queued".to_owned(),
        route_id: "north-pack-road".to_owned(),
        action: RouteAction::Repair,
    };
    client
        .commands
        .push_back(Phase5Command::Route(request.clone()));

    assert!(client.route_command_pending());
    assert!(!client.queue_cycle("route-repair"));
    assert!(!client.queue_route_action("route-duplicate".to_owned(), RouteAction::Escort));

    client.commands.clear();
    client.in_flight_command = Some(Phase5Command::Route(request));
    assert!(client.route_command_pending());
    assert!(!client.queue_cycle("route-improve"));
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
fn travel_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = Phase5Client::new();
    let request = tarrowyn_protocol::TravelRequest {
        request_id: "travel-queued".to_owned(),
        action: TravelAction::Start,
        route_id: Some("north-pack-road".to_owned()),
        travel_id: None,
    };
    client
        .commands
        .push_back(Phase5Command::Travel(request.clone()));

    assert!(client.travel_command_pending());
    assert!(!client.queue_cycle("travel"));
    client.queue_travel_action("travel-duplicate".to_owned(), TravelAction::Interrupt);
    assert_eq!(client.commands.len(), 1);

    client.commands.clear();
    client.in_flight_command = Some(Phase5Command::Travel(request));
    assert!(client.travel_command_pending());
    assert!(!client.queue_cycle("recover-travel"));
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
fn travel_control_does_not_offer_interrupt_during_recovery() {
    let mut client = Phase5Client::new();
    client.region = Some(tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: Vec::new(),
        visible_settlements: Vec::new(),
        player_location_id: "hearth".to_owned(),
        travel: Some(tarrowyn_protocol::TravelState {
            travel_id: "recovering-road".to_owned(),
            route_id: "north-pack-road".to_owned(),
            origin_location_id: "hearth".to_owned(),
            destination_location_id: "saltmere".to_owned(),
            departure_tick: 1,
            eta_tick: 4,
            progress: 35,
            risk_percent: 18,
            status: TravelStatus::Recovering,
            interruption: None,
            recovery_note: Some("The route crew is still working.".to_owned()),
        }),
        interest_radius: 12,
        cursor: 0,
    });

    assert_eq!(
        client.travel_control_details(),
        ("Recovering", false, false)
    );
    assert!(client.movement_locked());
    assert!(!client.queue_cycle("travel"));
    assert!(client.commands.is_empty());
}
