use super::{
    actions::{
        apply_chronicle_key, parse_cooperation_trade_command, parse_foundation_cache_command,
        parse_foundation_forge_command,
    },
    input::{
        keyboard_gameplay_blocked, keyboard_movement_direction, normalized_movement_direction,
        rendered_tile,
    },
    online_crafting_view, online_gameplay_modal_visible,
};
use crate::network::{ConnectionState, CraftingView};
use macroquad::prelude::{vec2, KeyCode};
use macroquad_toolkit::grid::TilePos;
use tarrowyn_protocol::{FoundationCacheAction, FoundationResourceKind};

#[test]
fn held_arrow_keys_form_a_free_two_dimensional_direction() {
    let direction = keyboard_movement_direction(|key| matches!(key, KeyCode::Up | KeyCode::Right));
    let normalized = normalized_movement_direction(direction);

    assert_eq!(direction, vec2(1.0, -1.0));
    assert!((normalized.length() - 1.0).abs() < 0.0001);
    assert!((normalized.x + normalized.y).abs() < 0.0001);
}

#[test]
fn forge_commands_only_accept_typed_nearby_actions() {
    assert_eq!(
        parse_foundation_forge_command("foundation-forge:burn-charcoal"),
        Some(tarrowyn_protocol::FoundationForgeAction::BurnCharcoal)
    );
    assert_eq!(
        parse_foundation_forge_command("foundation-forge:forge-field-tool"),
        Some(tarrowyn_protocol::FoundationForgeAction::ForgeFieldTool)
    );
    assert_eq!(
        parse_foundation_forge_command("foundation-forge:repair"),
        None
    );
}

#[test]
fn freeform_position_changes_tile_only_after_crossing_its_boundary() {
    assert_eq!(rendered_tile(vec2(8.49, 6.49)), TilePos::new(8, 6));
    assert_eq!(rendered_tile(vec2(8.51, 6.51)), TilePos::new(9, 7));
}

#[test]
fn chronicle_touch_keys_build_and_edit_a_query() {
    let mut query = String::new();
    apply_chronicle_key(&mut query, "S");
    apply_chronicle_key(&mut query, "T");
    apply_chronicle_key(&mut query, "space");
    apply_chronicle_key(&mut query, "R");
    apply_chronicle_key(&mut query, "delete");

    assert_eq!(query, "ST ");

    apply_chronicle_key(&mut query, "clear");
    assert!(query.is_empty());
}

#[test]
fn shared_cache_touch_commands_route_to_typed_actions() {
    assert_eq!(
        parse_foundation_cache_command("foundation-cache:deposit:stone"),
        Some((
            FoundationCacheAction::Deposit,
            Some(FoundationResourceKind::Stone)
        ))
    );
    assert_eq!(
        parse_foundation_cache_command("foundation-cache:inspect:none"),
        Some((FoundationCacheAction::Inspect, None))
    );
    assert_eq!(
        parse_foundation_cache_command("foundation-cache:deposit:none"),
        None
    );
}

#[test]
fn cooperation_touch_commands_build_exact_atomic_trade_requests() {
    let offer = parse_cooperation_trade_command("cooperation-offer-ore:account-2").unwrap();
    assert_eq!(offer.action, tarrowyn_protocol::TradeAction::Create);
    assert_eq!(offer.recipient_account_id.as_deref(), Some("account-2"));
    assert_eq!(offer.offer.unwrap().iron_ore, 2);
    assert!(offer.request.unwrap().is_empty());

    let accept = parse_cooperation_trade_command("cooperation-accept-ore:trade-4").unwrap();
    assert_eq!(accept.action, tarrowyn_protocol::TradeAction::Accept);
    assert_eq!(accept.trade_id.as_deref(), Some("trade-4"));
    assert!(parse_cooperation_trade_command("cooperation-offer-ore:").is_none());
}

#[test]
fn gameplay_keyboard_input_stops_behind_non_textual_modals() {
    assert!(!keyboard_gameplay_blocked(
        false, false, false, false, false
    ));
    assert!(keyboard_gameplay_blocked(true, false, false, false, false));
    assert!(keyboard_gameplay_blocked(false, true, false, false, false));
    assert!(keyboard_gameplay_blocked(false, false, true, false, false));
    assert!(keyboard_gameplay_blocked(false, false, false, true, false));
    assert!(keyboard_gameplay_blocked(false, false, false, false, true));
}

#[test]
fn crafting_overlay_waits_for_the_shared_road_before_hiding_reconnect() {
    let crafting = Some(CraftingView {
        progress: 0.5,
        target_start: 0.4,
        target_end: 0.6,
    });

    assert!(online_crafting_view(ConnectionState::Online, true, crafting).is_some());
    assert!(online_crafting_view(ConnectionState::Online, false, crafting).is_none());
    assert!(online_crafting_view(ConnectionState::Connecting, true, crafting).is_none());
    assert!(online_crafting_view(ConnectionState::Degraded, true, crafting).is_none());
    assert!(online_crafting_view(ConnectionState::Offline, true, crafting).is_none());
}

#[test]
fn gameplay_overlays_wait_for_authoritative_position_during_reload() {
    assert!(online_gameplay_modal_visible(ConnectionState::Online, true));
    assert!(!online_gameplay_modal_visible(
        ConnectionState::Online,
        false
    ));
    assert!(!online_gameplay_modal_visible(
        ConnectionState::Degraded,
        true
    ));
}
