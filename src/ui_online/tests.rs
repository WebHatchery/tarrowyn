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
    assert!(super::travel_control_enabled(true, false));
    assert!(!super::travel_control_enabled(true, true));
    assert!(!super::travel_control_enabled(false, false));
}
