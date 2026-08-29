use super::*;
use tarrowyn_protocol::{
    LocalCombatAction, LocalCombatState, ProfessionAction, SkillAction, SkillLesson, SkillStatus,
    SkillView, SkillsResponse, WeaponKind,
};

#[test]
fn crafting_challenge_moves_across_a_wide_target() {
    let mut client = Phase4Client::new();
    client.begin_crafting("service-order-1");
    let before = client.crafting_view().unwrap();
    advance_crafting(&mut client.crafting, 1.0);
    let after = client.crafting_view().unwrap();
    assert!(after.0 > before.0);
    assert_eq!(after.1, 0.38);
    assert_eq!(after.2, 0.66);
}

#[test]
fn phase_four_reset_discards_cached_ledgers() {
    let mut client = Phase4Client::new();
    client.skills = Some(SkillsResponse {
        skills: vec![SkillView {
            skill_id: "fishing".to_owned(),
            name: "Fishing".to_owned(),
            family: tarrowyn_protocol::SkillFamily::Gathering,
            depth: 1,
            mastery: 2,
            status: SkillStatus::Mastered,
            description: "Read water.".to_owned(),
            entry_hint: "Make a first catch.".to_owned(),
        }],
        lessons: Vec::new(),
        cursor: 3,
    });
    client.combat = Some(LocalCombatState {
        encounter_id: "encounter".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 2,
        player_health: 2,
        turn: 1,
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

    client.clear();

    assert!(client.skills.is_none());
    assert!(client.combat.is_none());
}

#[test]
fn crafting_tap_becomes_a_bounded_completion_request() {
    let mut client = Phase4Client::new();
    client.begin_crafting("service-order-2");
    advance_crafting(&mut client.crafting, 1.15);
    assert!(client.submit_crafting("craft-1".to_owned()));
    let Some(Phase4Command::Profession(request)) = client.commands.pop_front() else {
        panic!("crafting should queue a profession completion");
    };
    assert_eq!(request.action, ProfessionAction::CompleteOrder);
    assert_eq!(request.order_id.as_deref(), Some("service-order-2"));
    assert!(request.timing_score.is_some_and(|score| score <= 100));
}

#[test]
fn practice_button_queues_the_next_unstarted_root() {
    let mut client = Phase4Client::new();
    client.skills = Some(SkillsResponse {
        skills: vec![SkillView {
            skill_id: "fishing".to_owned(),
            name: "Fishing".to_owned(),
            family: tarrowyn_protocol::SkillFamily::Gathering,
            depth: 1,
            mastery: 0,
            status: SkillStatus::Available,
            description: "Read water.".to_owned(),
            entry_hint: "Make a first catch.".to_owned(),
        }],
        lessons: Vec::new(),
        cursor: 0,
    });
    client.queue_cycle("practice", "practice-1".to_owned());
    let Some(Phase4Command::Skill(request)) = client.commands.pop_front() else {
        panic!("practice should queue a skill request");
    };
    assert_eq!(request.action, SkillAction::Practice);
    assert_eq!(request.skill_id.as_deref(), Some("fishing"));
}

#[test]
fn local_fight_cycles_through_readable_weapon_families() {
    assert_eq!(next_combat_weapon(None), WeaponKind::IronSword);
    assert_eq!(
        next_combat_weapon(Some(WeaponKind::IronSword)),
        WeaponKind::Spear
    );
    assert_eq!(next_combat_weapon(Some(WeaponKind::Spear)), WeaponKind::Axe);
    assert_eq!(next_combat_weapon(Some(WeaponKind::Axe)), WeaponKind::Bow);
    assert_eq!(
        next_combat_weapon(Some(WeaponKind::Bow)),
        WeaponKind::Shield
    );
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
fn school_button_joins_an_open_lesson_for_the_learner() {
    let mut client = Phase4Client::new();
    client.own_account_id = Some("learner-1".to_owned());
    client.skills = Some(SkillsResponse {
        skills: Vec::new(),
        lessons: vec![SkillLesson {
            lesson_id: "school-lesson-1".to_owned(),
            teacher_account_id: "teacher-1".to_owned(),
            teacher_name: "Teacher".to_owned(),
            learner_account_id: "learner-1".to_owned(),
            learner_name: "Learner".to_owned(),
            skill_id: "sword-fighting".to_owned(),
            skill_name: "Sword Fighting".to_owned(),
            started_tick: 4,
            expires_tick: 24,
        }],
        cursor: 4,
    });
    assert!(client.queue_school("school-join".to_owned(), "teacher-1".to_owned()));
    let Some(Phase4Command::Skill(request)) = client.commands.pop_front() else {
        panic!("the learner should queue the open lesson");
    };
    assert_eq!(request.action, SkillAction::CompleteLesson);
    assert_eq!(request.lesson_id.as_deref(), Some("school-lesson-1"));
    assert_eq!(request.target_account_id.as_deref(), Some("teacher-1"));
}
