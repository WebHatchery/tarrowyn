use super::*;

#[test]
fn recovery_risk_label_stays_compact_without_losing_the_seed_rule() {
    assert_eq!(
        recovery_risk_label("At most one carried seed is risked on knockout."),
        "1 carried seed"
    );
    assert_eq!(
        recovery_risk_label("A carried tool may be damaged."),
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
