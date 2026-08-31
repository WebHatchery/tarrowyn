use super::*;

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
    assert!(client.market_command_pending());
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Market(request))
            if request.action == MarketOrderAction::Cancel
                && request.order_id.as_deref() == Some("own-saltmere-seeds")
    ));
    client.commands.clear();
    assert!(!client.market_command_pending());

    client.queue_cycle("market-region");
    assert!(!client.market_command_pending());
    assert!(client.commands.is_empty());

    client
        .region
        .as_mut()
        .expect("regional projection")
        .player_location_id = "saltmere".to_owned();
    client.queue_cycle("market-region");
    assert!(client.market_command_pending());
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
fn market_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = Phase5Client::new();
    let request = tarrowyn_protocol::MarketOrderRequest {
        request_id: "market-queued".to_owned(),
        action: MarketOrderAction::Create,
        order_id: None,
        destination_location_id: Some("saltmere".to_owned()),
        commodity: Some(tarrowyn_protocol::CommodityKind::Seeds),
        quantity: Some(1),
    };
    client
        .commands
        .push_back(Phase5Command::Market(request.clone()));

    assert!(client.market_command_pending());
    assert!(!client.queue_cycle("market-region"));
    assert!(!client.queue_cycle("cancel-market"));

    client.commands.clear();
    client.in_flight_command = Some(Phase5Command::Market(request));
    assert!(client.market_command_pending());
    assert!(!client.queue_cycle("market-region"));
}
