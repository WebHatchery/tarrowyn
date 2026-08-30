use super::*;

#[test]
fn retreat_button_queues_an_explicit_local_exit() {
    let mut client = Phase4Client::new();
    client.combat = Some(LocalCombatState {
        encounter_id: "whisperwood-local-1".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 2,
        player_health: 2,
        turn: 1,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: WeaponKind::Shield,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: false,
    });
    client.queue_cycle("retreat", "retreat-1".to_owned());
    let Some(Phase4Command::Combat(request)) = client.commands.pop_front() else {
        panic!("retreat should queue a local combat request");
    };
    assert_eq!(request.action, LocalCombatAction::Retreat);
    assert_eq!(request.weapon, WeaponKind::Shield);
}

#[test]
fn guard_button_queues_an_explicit_local_defense() {
    let mut client = Phase4Client::new();
    client.combat = Some(LocalCombatState {
        encounter_id: "whisperwood-local-1".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 3,
        player_health: 2,
        turn: 1,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: WeaponKind::Spear,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: false,
    });
    client.queue_cycle("guard", "guard-1".to_owned());
    let Some(Phase4Command::Combat(request)) = client.commands.pop_front() else {
        panic!("guard should queue a local combat request");
    };
    assert_eq!(request.action, LocalCombatAction::Guard);
    assert_eq!(request.weapon, WeaponKind::Spear);
}

#[test]
fn technique_button_queues_an_explicit_opening() {
    let mut client = Phase4Client::new();
    client.combat = Some(LocalCombatState {
        encounter_id: "whisperwood-local-1".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 3,
        player_health: 2,
        turn: 0,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: WeaponKind::IronSword,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: false,
    });
    client.queue_cycle("technique", "technique-1".to_owned());
    let Some(Phase4Command::Combat(request)) = client.commands.pop_front() else {
        panic!("technique should queue a local combat request");
    };
    assert_eq!(request.action, LocalCombatAction::Technique);
    assert_eq!(request.weapon, WeaponKind::IronSword);
}

#[test]
fn bandage_button_queues_an_explicit_item_use() {
    let mut client = Phase4Client::new();
    client.combat = Some(LocalCombatState {
        encounter_id: "whisperwood-local-1".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 2,
        player_health: 1,
        turn: 1,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: WeaponKind::Shield,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: false,
    });
    client.queue_cycle("item", "item-1".to_owned());
    let Some(Phase4Command::Combat(request)) = client.commands.pop_front() else {
        panic!("bandage should queue a local combat request");
    };
    assert_eq!(request.action, LocalCombatAction::UseItem);
    assert_eq!(request.weapon, WeaponKind::Shield);
}

#[test]
fn reposition_button_queues_an_explicit_movement_action() {
    let mut client = Phase4Client::new();
    client.combat = Some(LocalCombatState {
        encounter_id: "whisperwood-local-1".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 3,
        player_health: 2,
        turn: 0,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: WeaponKind::IronSword,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: false,
    });
    client.queue_cycle("reposition", "reposition-1".to_owned());
    let Some(Phase4Command::Combat(request)) = client.commands.pop_front() else {
        panic!("reposition should queue a local combat request");
    };
    assert_eq!(request.action, LocalCombatAction::Reposition);
    assert_eq!(request.weapon, WeaponKind::IronSword);
}

#[test]
fn spell_button_queues_an_explicit_cast() {
    let mut client = Phase4Client::new();
    client.combat = Some(LocalCombatState {
        encounter_id: "whisperwood-local-1".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 3,
        player_health: 2,
        turn: 0,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: WeaponKind::IronSword,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: true,
    });
    client.queue_cycle("spell", "spell-1".to_owned());
    let Some(Phase4Command::Combat(request)) = client.commands.pop_front() else {
        panic!("spell should queue a local combat request");
    };
    assert_eq!(request.action, LocalCombatAction::CastSpell);
    assert_eq!(request.weapon, WeaponKind::IronSword);
}

#[test]
fn discovered_storm_magic_changes_the_visible_spell_capability() {
    let mut client = Phase4Client::new();
    client.skills = Some(SkillsResponse {
        skills: vec![SkillView {
            skill_id: "storm-magic".to_owned(),
            name: "Storm Magic".to_owned(),
            family: tarrowyn_protocol::SkillFamily::Magic,
            depth: 2,
            mastery: 0,
            usable: true,
            status: SkillStatus::Discovered,
            description: "A deliberate storm working.".to_owned(),
            entry_hint: "The three currents answer one another.".to_owned(),
        }],
        lessons: Vec::new(),
        cursor: 0,
    });

    assert!(client.storm_magic_unlocked());
}

#[test]
fn discovered_but_unready_storm_magic_keeps_the_basic_spell_capability() {
    let mut client = Phase4Client::new();
    client.skills = Some(SkillsResponse {
        skills: vec![SkillView {
            skill_id: "storm-magic".to_owned(),
            name: "Storm Magic".to_owned(),
            family: tarrowyn_protocol::SkillFamily::Magic,
            depth: 2,
            mastery: 0,
            usable: false,
            status: SkillStatus::Discovered,
            description: "A deliberate storm working.".to_owned(),
            entry_hint: "The three currents answer one another.".to_owned(),
        }],
        lessons: Vec::new(),
        cursor: 0,
    });

    assert!(!client.storm_magic_unlocked());
}
