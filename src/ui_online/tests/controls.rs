use super::super::*;

#[test]
fn reconnect_control_waits_for_a_failed_connection() {
    assert!(!super::super::panels::reconnect_control_enabled(
        ConnectionState::Connecting
    ));
    assert!(!super::super::panels::reconnect_control_enabled(
        ConnectionState::Online
    ));
    assert!(super::super::panels::reconnect_control_enabled(
        ConnectionState::Degraded
    ));
    assert!(super::super::panels::reconnect_control_enabled(
        ConnectionState::Offline
    ));
}

#[test]
fn walking_controls_wait_for_an_authoritative_connection() {
    assert!(super::super::walking_connection_enabled(
        ConnectionState::Online,
        false
    ));
    assert!(super::super::walking_connection_enabled(
        ConnectionState::Offline,
        true
    ));
    assert!(!super::super::walking_connection_enabled(
        ConnectionState::Connecting,
        false
    ));
    assert!(!super::super::walking_connection_enabled(
        ConnectionState::Degraded,
        false
    ));
}

#[test]
fn walking_controls_wait_for_an_authoritative_player_projection() {
    assert!(super::super::walking_projection_enabled(true, false));
    assert!(super::super::walking_projection_enabled(false, true));
    assert!(!super::super::walking_projection_enabled(false, false));
}

#[test]
fn movement_tooltip_names_the_next_visible_recovery_path() {
    assert!(super::super::movement_tooltip_for(
        ConnectionState::Degraded,
        false,
        false,
        false,
        false
    )
    .contains("Reconnect"));
    assert!(super::super::movement_tooltip_for(
        ConnectionState::Online,
        false,
        false,
        false,
        false
    )
    .contains("position is still loading"));
    assert!(super::super::movement_tooltip_for(
        ConnectionState::Online,
        false,
        false,
        false,
        false
    )
    .contains("shared road snapshot"));
    assert!(
        super::super::movement_tooltip_for(ConnectionState::Online, false, true, true, false)
            .contains("recovery prompt")
    );
    assert!(
        super::super::movement_tooltip_for(ConnectionState::Online, false, false, true, true)
            .contains("Travel or Recover")
    );
}

#[test]
fn map_tooltip_names_the_modal_gate_before_movement() {
    assert_eq!(
        super::super::movement_tooltip_for_overlay(true, "Tap a walkable tile"),
        "Close the open panel to use road controls"
    );
    assert_eq!(
        super::super::movement_tooltip_for_overlay(false, "Tap a walkable tile"),
        "Tap a walkable tile"
    );
}

#[test]
fn companion_count_ignores_own_stale_and_offline_presence() {
    let players = vec![
        RemotePlayer {
            account_id: "self".to_owned(),
            character_id: "self-character".to_owned(),
            display_name: "Self".to_owned(),
            position: TilePos::new(8, 6),
            last_seen_tick: 22,
            online: true,
        },
        RemotePlayer {
            account_id: "active".to_owned(),
            character_id: "active-character".to_owned(),
            display_name: "Active".to_owned(),
            position: TilePos::new(9, 6),
            last_seen_tick: 22,
            online: true,
        },
        RemotePlayer {
            account_id: "offline".to_owned(),
            character_id: "offline-character".to_owned(),
            display_name: "Offline".to_owned(),
            position: TilePos::new(7, 6),
            last_seen_tick: 22,
            online: false,
        },
        RemotePlayer {
            account_id: "aged".to_owned(),
            character_id: "aged-character".to_owned(),
            display_name: "Aged".to_owned(),
            position: TilePos::new(6, 6),
            last_seen_tick: 1,
            online: true,
        },
    ];

    assert_eq!(
        super::super::visible_companion_count(&players, Some("self"), 22),
        1
    );
    assert_eq!(super::super::visible_player_count(&players, 22), 2);
}

#[test]
fn online_buttons_wait_for_authoritative_player_projection() {
    assert!(super::super::panels::button_enabled(
        true,
        ConnectionState::Online,
        true
    ));
    assert!(!super::super::panels::button_enabled(
        true,
        ConnectionState::Online,
        false
    ));
    assert!(!super::super::panels::button_enabled(
        true,
        ConnectionState::Degraded,
        true
    ));
}

#[test]
fn account_control_stays_available_during_player_projection_reload() {
    assert!(super::super::panels::account_control_enabled(
        true,
        ConnectionState::Online
    ));
    assert!(!super::super::panels::account_control_enabled(
        false,
        ConnectionState::Online
    ));
    assert!(!super::super::panels::account_control_enabled(
        true,
        ConnectionState::Connecting
    ));
}

#[test]
fn session_controls_stay_available_during_player_projection_reload() {
    for control in ["logout", "report", "delete-account"] {
        assert!(
            super::super::panels::sidebar_button_enabled(
                control,
                true,
                ConnectionState::Online,
                false
            ),
            "{control} should not depend on the gameplay position projection"
        );
    }
    assert!(!super::super::panels::sidebar_button_enabled(
        "logout",
        true,
        ConnectionState::Degraded,
        false
    ));
    assert!(!super::super::panels::sidebar_button_enabled(
        "plant",
        true,
        ConnectionState::Online,
        false
    ));
}

#[test]
fn recovery_controls_close_after_one_choice_is_pending() {
    assert!(!super::super::recovery_control_enabled(true, true));
    assert!(super::super::recovery_control_enabled(true, false));
    assert!(!super::super::recovery_control_enabled(false, false));
}

#[test]
fn market_controls_wait_for_the_previous_order_command() {
    assert!(!super::super::market_control_enabled(true));
    assert!(super::super::market_control_enabled(false));
    assert!(!super::super::cancel_market_control_enabled(true, true));
    assert!(super::super::cancel_market_control_enabled(true, false));
    assert!(!super::super::cancel_market_control_enabled(false, false));
}

#[test]
fn trade_controls_wait_for_the_previous_trade_command() {
    assert!(!super::super::trade_control_enabled(true, true));
    assert!(super::super::trade_control_enabled(true, false));
    assert!(!super::super::trade_control_enabled(false, false));
}

#[test]
fn farming_controls_wait_for_the_previous_field_command() {
    assert!(!super::super::farming_control_enabled(true, true));
    assert!(super::super::farming_control_enabled(true, false));
    assert!(!super::super::farming_control_enabled(false, false));
}

#[test]
fn event_controls_wait_for_the_previous_resolution_command() {
    assert!(!super::super::event_control_enabled(true));
    assert!(super::super::event_control_enabled(false));
}

#[test]
fn identity_controls_wait_for_the_previous_account_command() {
    assert!(!super::super::identity_control_enabled(true));
    assert!(super::super::identity_control_enabled(false));
}

#[test]
fn report_controls_wait_for_the_previous_moderation_command() {
    assert!(!super::super::report_control_enabled(true));
    assert!(super::super::report_control_enabled(false));
}

#[test]
fn claim_controls_wait_for_the_previous_lease_command() {
    assert!(!super::super::claim_control_enabled(true, true));
    assert!(super::super::claim_control_enabled(true, false));
    assert!(!super::super::claim_control_enabled(false, false));
}

#[test]
fn route_controls_wait_for_the_previous_logistics_command() {
    assert!(!super::super::route_control_enabled(true, true));
    assert!(super::super::route_control_enabled(true, false));
    assert!(!super::super::route_control_enabled(false, false));
}

#[test]
fn governance_controls_wait_for_the_previous_settlement_command() {
    assert!(!super::super::governance_control_enabled(true));
    assert!(super::super::governance_control_enabled(false));
}

#[test]
fn skill_controls_wait_for_the_previous_ledger_command() {
    assert!(!super::super::skill_control_enabled(true, true));
    assert!(super::super::skill_control_enabled(true, false));
    assert!(!super::super::skill_control_enabled(false, false));
}

#[test]
fn knowledge_controls_wait_for_the_previous_archive_command() {
    assert!(!super::super::knowledge_control_enabled(true));
    assert!(super::super::knowledge_control_enabled(false));
}

#[test]
fn order_controls_wait_for_the_previous_service_command() {
    assert!(!super::super::order_control_enabled(true, true));
    assert!(!super::super::order_control_enabled(false, false));
    assert!(super::super::order_control_enabled(true, false));
}

#[test]
fn combat_controls_wait_for_the_previous_encounter_command() {
    assert!(!super::super::combat_control_enabled(true, true));
    assert!(super::super::combat_control_enabled(true, false));
    assert!(!super::super::combat_control_enabled(false, false));
}

#[test]
fn contract_controls_wait_for_the_previous_frontier_command() {
    assert!(!super::super::contract_control_enabled(true, true));
    assert!(super::super::contract_control_enabled(true, false));
    assert!(!super::super::contract_control_enabled(false, false));
}

#[test]
fn expedition_controls_wait_for_the_previous_frontier_command() {
    assert!(!super::super::expedition_control_enabled(true, true));
    assert!(super::super::expedition_control_enabled(true, false));
    assert!(!super::super::expedition_control_enabled(false, false));
}

#[test]
fn frontier_combat_controls_wait_for_the_previous_threat_command() {
    assert!(!super::super::frontier_combat_control_enabled(true, true));
    assert!(super::super::frontier_combat_control_enabled(true, false));
    assert!(!super::super::frontier_combat_control_enabled(false, false));
}

#[test]
fn frontier_claim_controls_wait_for_the_previous_lease_command() {
    assert!(!super::super::claim_control_enabled(true, true));
    assert!(super::super::claim_control_enabled(true, false));
}
