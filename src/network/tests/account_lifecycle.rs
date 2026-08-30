use super::*;
use tarrowyn_protocol::{PlayerProjection, TradeStatus};

#[test]
fn linked_account_handoff_discards_stale_world_and_trade_projections() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.pending_state = Some(Pending::failed("guest state still in flight"));
    client.pending_events = Some(Pending::failed("guest events still in flight"));
    client.pending_trades = Some(Pending::failed("guest trades still in flight"));
    client.pending_trade_action = Some(TradeAction::Review);
    client.trades.push(tarrowyn_protocol::TradeOffer {
        trade_id: "guest-trade".to_owned(),
        creator_account_id: "guest-account".to_owned(),
        creator_name: "Guest".to_owned(),
        recipient_account_id: "other-account".to_owned(),
        recipient_name: "Other".to_owned(),
        offer: tarrowyn_protocol::TradeBundle {
            seeds: 1,
            ..Default::default()
        },
        request: tarrowyn_protocol::TradeBundle {
            wheat: 1,
            ..Default::default()
        },
        status: TradeStatus::Pending,
        created_tick: 1,
        expires_tick: 9,
    });
    client.projection.player = Some(PlayerProjection {
        account_id: "guest-account".to_owned(),
        character_id: "guest-character".to_owned(),
        display_name: "Guest".to_owned(),
        position: Position { x: 8, y: 6 },
        gold: 12,
        field_tool_condition: 3,
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
    });
    client.projection.trades = client.trades.clone();
    client.state_refresh = 4.0;

    client.apply_linked_account(GuestSessionResponse {
        client_key: "linked-client".to_owned(),
        account_id: "account-1".to_owned(),
        character_id: "character-1".to_owned(),
        display_name: "Linked traveller".to_owned(),
        account_token: "prod-session-1".to_owned(),
        expires_in_seconds: 900,
    });

    assert_eq!(
        client
            .account
            .as_ref()
            .map(|account| account.account_id.as_str()),
        Some("account-1")
    );
    assert!(client.pending_state.is_none());
    assert!(client.pending_events.is_none());
    assert!(client.pending_trades.is_none());
    assert!(client.pending_trade_action.is_none());
    assert!(client.trades.is_empty());
    assert!(client.projection.player.is_none());
    assert!(client.projection.trades.is_empty());
    assert_eq!(client.state_refresh, 0.0);
}
