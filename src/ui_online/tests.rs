use super::*;

#[test]
fn recovery_risk_label_stays_compact_without_losing_the_seed_rule() {
    assert_eq!(
        super::panels::recovery_risk_label("At most one carried seed is risked on knockout."),
        "1 carried seed"
    );
    assert_eq!(
        super::panels::recovery_risk_label("A carried tool may be damaged."),
        "carried item"
    );
}

#[test]
fn combat_status_names_the_improvised_weapon_and_action_window() {
    assert_eq!(
        super::panels::combat_weapon_line(
            tarrowyn_protocol::WeaponKind::ImprovisedClub,
            "Action opens in 1 beat",
        ),
        "Weapon: improvised club  •  Action opens in 1 beat"
    );
}

#[test]
fn combat_side_control_exposes_retreat_or_contract_by_state() {
    assert_eq!(combat_side_control(None, false), ("contract", "Contract"));
    assert_eq!(
        combat_side_control(None, true),
        ("frontier-retreat", "Retreat")
    );
    let combat = tarrowyn_protocol::LocalCombatState {
        encounter_id: "encounter".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 2,
        player_health: 2,
        turn: 1,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: tarrowyn_protocol::WeaponKind::IronSword,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: false,
    };
    assert_eq!(
        combat_side_control(Some(&combat), false),
        ("retreat", "Retreat")
    );
}

#[test]
fn frontier_retreat_requires_an_active_threat_within_reach() {
    let mut zone = tarrowyn_protocol::WildernessZone {
        zone_id: "whisperwood-edge".to_owned(),
        name: "Whisperwood Edge".to_owned(),
        monster: tarrowyn_protocol::MonsterKind::Brambleback,
        monster_health: 3,
        threat_active: true,
        road_open: false,
        position: tarrowyn_protocol::Position { x: 12, y: 4 },
        price_modifier_percent: 20,
        resource_demand: "iron".to_owned(),
        rumour: "The road is costly.".to_owned(),
    };

    assert!(super::panels::frontier_threat_is_reachable(
        TilePos::new(10, 4),
        Some(&zone)
    ));
    assert!(!super::panels::frontier_threat_is_reachable(
        TilePos::new(9, 4),
        Some(&zone)
    ));

    zone.threat_active = false;
    assert!(!super::panels::frontier_threat_is_reachable(
        TilePos::new(10, 4),
        Some(&zone)
    ));
}

#[test]
fn skill_selection_keeps_roots_open_and_advanced_arts_hidden() {
    let available = tarrowyn_protocol::SkillView {
        skill_id: "fishing".to_owned(),
        name: "Fishing".to_owned(),
        family: tarrowyn_protocol::SkillFamily::Gathering,
        depth: 1,
        mastery: 0,
        status: tarrowyn_protocol::SkillStatus::Available,
        description: "Read water.".to_owned(),
        entry_hint: "Make a first catch.".to_owned(),
    };
    let practising = tarrowyn_protocol::SkillView {
        status: tarrowyn_protocol::SkillStatus::Practising,
        mastery: 2,
        ..available.clone()
    };
    let advanced = tarrowyn_protocol::SkillView {
        depth: 2,
        status: tarrowyn_protocol::SkillStatus::Resonating,
        ..available.clone()
    };
    let mastered = tarrowyn_protocol::SkillView {
        status: tarrowyn_protocol::SkillStatus::Mastered,
        mastery: 5,
        ..available
    };
    assert!(super::panels::skill_practice_choice(&practising));
    assert!(!super::panels::skill_practice_choice(&advanced));
    assert!(!super::panels::skill_practice_choice(&mastered));
}

#[test]
fn advanced_skill_line_names_only_revealed_discovery_states() {
    let hidden = tarrowyn_protocol::SkillView {
        skill_id: "hidden-art".to_owned(),
        name: "Hidden Art".to_owned(),
        family: tarrowyn_protocol::SkillFamily::Magic,
        depth: 2,
        mastery: 0,
        status: tarrowyn_protocol::SkillStatus::Available,
        description: "A secret pattern.".to_owned(),
        entry_hint: "Not yet.".to_owned(),
    };
    let hidden_available = hidden.clone();
    let resonating = tarrowyn_protocol::SkillView {
        skill_id: "storm-magic".to_owned(),
        name: "Storm Magic".to_owned(),
        status: tarrowyn_protocol::SkillStatus::Resonating,
        ..hidden.clone()
    };
    let discovered = tarrowyn_protocol::SkillView {
        skill_id: "weapon-fighting".to_owned(),
        name: "Weapon Fighting".to_owned(),
        status: tarrowyn_protocol::SkillStatus::Discovered,
        ..hidden
    };

    assert_eq!(
        super::panels::advanced_skill_line(&[resonating, discovered]),
        "Advanced arts in your ledger: Storm Magic (resonating) • Weapon Fighting (discovered)"
    );
    assert!(!super::panels::advanced_skill_line(&[hidden_available]).contains("Hidden Art"));
}

#[test]
fn regional_route_actions_close_when_every_local_route_is_closed() {
    let region = tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: vec![tarrowyn_protocol::RouteRecord {
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
            status: tarrowyn_protocol::RouteStatus::Closed,
            last_action_tick: 0,
            note: "The road is closed.".to_owned(),
        }],
        visible_settlements: Vec::new(),
        player_location_id: "hearth".to_owned(),
        travel: None,
        interest_radius: 12,
        cursor: 0,
    };

    assert!(super::panels::has_local_route(
        Some(&region),
        tarrowyn_protocol::RouteAction::Repair
    ));
    assert!(super::panels::has_local_route(
        Some(&region),
        tarrowyn_protocol::RouteAction::Escort
    ));
    assert!(!super::panels::has_local_route(
        Some(&region),
        tarrowyn_protocol::RouteAction::Improve
    ));
}

#[test]
fn pioneer_status_line_keeps_an_active_party_visible() {
    let expedition = tarrowyn_protocol::Expedition {
        expedition_id: "pioneer-1".to_owned(),
        outpost_name: "Lantern Rest".to_owned(),
        leader_account_id: "account-1".to_owned(),
        members: vec![tarrowyn_protocol::ExpeditionMember {
            account_id: "account-1".to_owned(),
            display_name: "The traveller".to_owned(),
            role: tarrowyn_protocol::ExpeditionRole::Scout,
        }],
        food: 6,
        tools: 3,
        materials: 8,
        safety: 3,
        status: tarrowyn_protocol::ExpeditionStatus::Launched,
        outcome: None,
        outpost_position: tarrowyn_protocol::Position { x: 14, y: 8 },
    };

    let line = super::pioneer_status_line(
        &expedition,
        tarrowyn_protocol::ExpeditionRequirements {
            food: 10,
            tools: 5,
            materials: 12,
            safety: 7,
        },
    );
    assert!(line.contains("Pioneer on the road"));
    assert!(line.contains("1 companions"));
    assert!(line.contains("F6/10 T3/5 M8/12 S3/7"));
    assert!(line.contains("Lantern Rest"));
}

#[test]
fn travel_controls_close_while_knocked_out() {
    assert!(super::travel_control_enabled(true, false, false));
    assert!(!super::travel_control_enabled(true, true, false));
    assert!(!super::travel_control_enabled(true, false, true));
    assert!(!super::travel_control_enabled(false, false, false));
}

#[test]
fn local_combat_actions_wait_for_an_engaged_ready_encounter() {
    let ready = tarrowyn_protocol::LocalCombatState {
        encounter_id: "encounter".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 2,
        player_health: 2,
        turn: 0,
        status: tarrowyn_protocol::LocalCombatStatus::Ready,
        weapon: tarrowyn_protocol::WeaponKind::IronSword,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: true,
    };
    let mut engaged = ready.clone();
    engaged.status = tarrowyn_protocol::LocalCombatStatus::Engaged;
    engaged.action_available_at_tick = 11;

    assert!(!super::panels::local_combat_action_enabled(None, 10));
    assert!(!super::panels::local_combat_action_enabled(
        Some(&ready),
        10
    ));
    assert!(!super::panels::local_combat_action_enabled(
        Some(&engaged),
        10
    ));

    engaged.action_available_at_tick = 10;
    assert!(super::panels::local_combat_action_enabled(
        Some(&engaged),
        10
    ));
}

#[test]
fn regional_journey_locks_walking_until_arrival_or_recovery() {
    let mut region = tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: Vec::new(),
        visible_settlements: Vec::new(),
        player_location_id: "hearth".to_owned(),
        travel: Some(tarrowyn_protocol::TravelState {
            travel_id: "travel-1".to_owned(),
            route_id: "north-pack-road".to_owned(),
            origin_location_id: "hearth".to_owned(),
            destination_location_id: "whisperwood-outpost".to_owned(),
            departure_tick: 1,
            eta_tick: 7,
            progress: 20,
            risk_percent: 28,
            status: tarrowyn_protocol::TravelStatus::Travelling,
            interruption: None,
            recovery_note: None,
        }),
        interest_radius: 12,
        cursor: 0,
    };

    assert!(super::panels::regional_travel_blocks_movement(Some(
        &region
    )));
    region.travel.as_mut().unwrap().status = tarrowyn_protocol::TravelStatus::Interrupted;
    assert!(super::panels::regional_travel_blocks_movement(Some(
        &region
    )));
    region.travel.as_mut().unwrap().status = tarrowyn_protocol::TravelStatus::Recovering;
    assert!(super::panels::regional_travel_blocks_movement(Some(
        &region
    )));
    region.travel.as_mut().unwrap().status = tarrowyn_protocol::TravelStatus::Arrived;
    assert!(!super::panels::regional_travel_blocks_movement(Some(
        &region
    )));
    assert!(!super::panels::regional_travel_blocks_movement(None));
}

#[test]
fn reconnect_control_waits_for_a_failed_connection() {
    assert!(!super::panels::reconnect_control_enabled(
        ConnectionState::Connecting
    ));
    assert!(!super::panels::reconnect_control_enabled(
        ConnectionState::Online
    ));
    assert!(super::panels::reconnect_control_enabled(
        ConnectionState::Degraded
    ));
    assert!(super::panels::reconnect_control_enabled(
        ConnectionState::Offline
    ));
}

#[test]
fn recovery_controls_close_after_one_choice_is_pending() {
    assert!(!super::recovery_control_enabled(true, true));
    assert!(super::recovery_control_enabled(true, false));
    assert!(!super::recovery_control_enabled(false, false));
}

#[test]
fn market_controls_wait_for_the_previous_order_command() {
    assert!(!super::market_control_enabled(true));
    assert!(super::market_control_enabled(false));
    assert!(!super::cancel_market_control_enabled(true, true));
    assert!(super::cancel_market_control_enabled(true, false));
    assert!(!super::cancel_market_control_enabled(false, false));
}

#[test]
fn trade_controls_wait_for_the_previous_trade_command() {
    assert!(!super::trade_control_enabled(true, true));
    assert!(super::trade_control_enabled(true, false));
    assert!(!super::trade_control_enabled(false, false));
}

#[test]
fn event_controls_wait_for_the_previous_resolution_command() {
    assert!(!super::event_control_enabled(true));
    assert!(super::event_control_enabled(false));
}

#[test]
fn identity_controls_wait_for_the_previous_account_command() {
    assert!(!super::identity_control_enabled(true));
    assert!(super::identity_control_enabled(false));
}

#[test]
fn claim_controls_wait_for_the_previous_lease_command() {
    assert!(!super::claim_control_enabled(true, true));
    assert!(super::claim_control_enabled(true, false));
    assert!(!super::claim_control_enabled(false, false));
}

#[test]
fn route_controls_wait_for_the_previous_logistics_command() {
    assert!(!super::route_control_enabled(true, true));
    assert!(super::route_control_enabled(true, false));
    assert!(!super::route_control_enabled(false, false));
}

#[test]
fn governance_controls_wait_for_the_previous_settlement_command() {
    assert!(!super::governance_control_enabled(true));
    assert!(super::governance_control_enabled(false));
}

#[test]
fn skill_controls_wait_for_the_previous_ledger_command() {
    assert!(!super::skill_control_enabled(true, true));
    assert!(super::skill_control_enabled(true, false));
    assert!(!super::skill_control_enabled(false, false));
}

#[test]
fn chronicle_panel_text_keeps_archive_context_and_recent_records() {
    let entries = vec![
        tarrowyn_protocol::ChronicleEntry {
            event_id: "old-event".to_owned(),
            kind: "harvest".to_owned(),
            title: "The first sheaf rises".to_owned(),
            text: "A neighbour brought in wheat before dusk.".to_owned(),
            created_tick: 3,
            cursor: 1,
        },
        tarrowyn_protocol::ChronicleEntry {
            event_id: "new-event".to_owned(),
            kind: "storm".to_owned(),
            title: "The storm answers".to_owned(),
            text: "The eastern road glows under blue rain.".to_owned(),
            created_tick: 9,
            cursor: 2,
        },
    ];
    let summary = tarrowyn_protocol::ChronicleSummary {
        from_tick: 1,
        to_tick: 9,
        from_cursor: 1,
        to_cursor: 2,
        entry_count: 17,
        kinds: vec!["harvest".to_owned(), "storm".to_owned()],
        highlights: vec!["The eastern road glows.".to_owned()],
    };

    let text = super::panels::chronicle_panel_text(&entries, Some(&summary));

    assert!(text.contains("Archive: 17 records across beats 1–9."));
    assert!(text.contains("Last highlight: The eastern road glows."));
    assert!(text.contains("The storm answers — The eastern road glows under blue rain."));
    assert!(text.contains("The first sheaf rises — A neighbour brought in wheat before dusk."));
}

#[test]
fn chronicle_search_panel_text_distinguishes_empty_results() {
    let text = super::panels::chronicle_search_panel_text("lost road", &[], None);

    assert!(text.contains("Search results for “lost road”:"));
    assert!(text.contains("No matching records were found in the durable chronicle."));
}
