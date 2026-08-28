use super::*;
use tarrowyn_protocol::{
    ProfessionAction, SkillAction, SkillStatus, SkillView, SkillsResponse, WeaponKind,
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
