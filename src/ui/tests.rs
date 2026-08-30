use super::*;

#[test]
fn online_footer_keeps_wallet_inventory_and_presence_visible() {
    let stats = "Gold 12  Skill 4  Reputation 3\nRank Wayfarer • 1 credentials\nField tool 2/3 • Clear • pests 0/2 • Goat 3/3\nWheat 5  Turnips 2  Moonberries 1  Seeds 4 • Bandages 2";

    assert_eq!(
        online_footer_detail(stats, 3, None),
        "Gold 12  Skill 4  Reputation 3 • 3 players\nWheat 5  Turnips 2  Moonberries 1  Seeds 4 • Bandages 2"
    );
}

#[test]
fn online_footer_exposes_pending_trade_terms_and_direction() {
    let trade = tarrowyn_protocol::TradeOffer {
        trade_id: "trade-1".to_owned(),
        creator_account_id: "farmer".to_owned(),
        creator_name: "Farmer".to_owned(),
        recipient_account_id: "adventurer".to_owned(),
        recipient_name: "Adventurer".to_owned(),
        offer: tarrowyn_protocol::TradeBundle {
            seeds: 1,
            ..Default::default()
        },
        request: tarrowyn_protocol::TradeBundle {
            gold: 2,
            ..Default::default()
        },
        status: tarrowyn_protocol::TradeStatus::Pending,
        created_tick: 4,
        expires_tick: 24,
    };
    let detail = pending_trade_detail(&[trade], Some("adventurer")).unwrap();
    assert_eq!(detail, "Trade from Farmer: 1 seeds for 2 gold");
    assert_eq!(
        online_footer_detail("Gold 12\nRank\nField\nWheat 1", 2, Some(&detail)),
        "Gold 12 • 2 players\nWheat 1\nTrade from Farmer: 1 seeds for 2 gold"
    );
}

#[test]
fn map_player_marker_waits_for_authority_unless_using_the_offline_fixture() {
    assert!(super::ui_map::should_draw_player_marker(true, false));
    assert!(super::ui_map::should_draw_player_marker(false, true));
    assert!(!super::ui_map::should_draw_player_marker(false, false));
}

#[test]
fn modal_filters_keep_recovery_controls_touchable() {
    assert!(is_recovery_action(&UiAction::Reconnect));
    assert!(is_recovery_action(&UiAction::UseOffline));
    assert!(!is_recovery_action(&UiAction::Interact(
        "account-close".to_owned()
    )));
    assert!(!is_recovery_action(&UiAction::Practice(
        "farming".to_owned()
    )));
}
