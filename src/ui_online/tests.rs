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
