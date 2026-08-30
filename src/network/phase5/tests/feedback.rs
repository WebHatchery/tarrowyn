use super::*;
use tarrowyn_protocol::{MarketOrder, MarketOrderAction, RouteRecord, RouteStatus};

#[test]
fn market_success_notice_describes_the_requested_action() {
    assert_eq!(
        super::super::market_success_message(Some(MarketOrderAction::Create), false),
        "The shipment is on the regional ledger."
    );
    assert_eq!(
        super::super::market_success_message(Some(MarketOrderAction::Fulfil), false),
        "The shipment reached its destination and settled."
    );
    assert_eq!(
        super::super::market_success_message(Some(MarketOrderAction::Cancel), false),
        "The shipment was cancelled and its escrow returned."
    );
    assert_eq!(
        super::super::market_success_message(Some(MarketOrderAction::Cancel), true),
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
        super::super::market_result_message(Some(MarketOrderAction::Create), false, Some(&order)),
        "The shipment is on the regional ledger. Details: 1 seed from hearth to saltmere • 4 gold."
    );
}

#[test]
fn route_success_message_names_condition_and_risk() {
    let route = RouteRecord {
        route_id: "north-pack-road".to_owned(),
        name: "North Pack Road".to_owned(),
        origin_location_id: "hearth".to_owned(),
        destination_location_id: "saltmere".to_owned(),
        transport: "pack road".to_owned(),
        length: 6,
        risk_percent: 18,
        condition: 82,
        capacity: 8,
        travel_ticks: 4,
        repair_cost: 3,
        status: RouteStatus::Operational,
        last_action_tick: 4,
        note: "The road is open to carts.".to_owned(),
    };

    assert_eq!(
        super::super::commands::route_success_message(&route),
        "North Pack Road is open • condition 82 • 18% risk."
    );
}

#[test]
fn event_success_message_names_intervention_and_outcome() {
    let mut event = regional_event(
        "event-message",
        tarrowyn_protocol::RegionalEventStage::Intervention,
        1,
    );
    event.chosen_intervention = Some("protect grain stores".to_owned());
    assert_eq!(
        super::super::commands::event_success_message(Some(&event)),
        "The thaw road recorded: protect grain stores."
    );

    event.stage = tarrowyn_protocol::RegionalEventStage::Resolution;
    event.outcome = Some("The roads reopen before winter.".to_owned());
    assert_eq!(
        super::super::commands::event_success_message(Some(&event)),
        "The thaw road resolved: The roads reopen before winter."
    );
}

#[test]
fn moderation_success_message_keeps_the_report_reference() {
    let response = tarrowyn_protocol::ModerationReportResponse {
        request_id: "report-request".to_owned(),
        accepted: true,
        report_id: "report-42".to_owned(),
        status: "queued".to_owned(),
        reason: None,
    };

    assert_eq!(
        super::super::commands::moderation_success_message(&response),
        "Moderation report report-42 is queued; the report is recorded for review."
    );
}

#[test]
fn regional_rejection_without_a_reason_still_leaves_a_visible_notice() {
    let mut notices = Vec::new();

    super::super::phase5_notice(false, None, "unused success", &mut notices);

    assert!(matches!(
        notices.first(),
        Some(NetworkNotice::Warning(message))
            if message == "The regional action was not accepted."
    ));
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
        super::super::commands::travel_success_message(Some(&travel), "hearth"),
        "Journey underway to saltmere • 50% complete • 18% risk."
    );
}

#[test]
fn travel_success_message_keeps_the_recovery_note_after_resuming() {
    let travel = tarrowyn_protocol::TravelState {
        travel_id: "travel-recovered".to_owned(),
        route_id: "north-pack-road".to_owned(),
        origin_location_id: "hearth".to_owned(),
        destination_location_id: "saltmere".to_owned(),
        departure_tick: 4,
        eta_tick: 8,
        progress: 35,
        risk_percent: 18,
        status: TravelStatus::Travelling,
        interruption: Some("A route warning stopped the caravan safely.".to_owned()),
        recovery_note: Some("The route crew found a safe continuation.".to_owned()),
    };

    assert_eq!(
        super::super::commands::travel_success_message(Some(&travel), "hearth"),
        "Journey underway to saltmere • 35% complete • 18% risk. The route crew found a safe continuation."
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
        super::super::commands::travel_success_message(Some(&travel), "hearth"),
        "Journey interrupted before saltmere: A fallen marker blocks the trail. Tap Recover to continue."
    );

    travel.status = TravelStatus::Arrived;
    travel.progress = 100;
    assert_eq!(
        super::super::commands::travel_success_message(Some(&travel), "saltmere"),
        "Arrived at saltmere."
    );
}
