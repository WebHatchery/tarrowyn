use super::super::*;

#[test]
fn combat_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = Phase4Client::new();
    let request = LocalCombatRequest {
        request_id: "combat-queued".to_owned(),
        action: LocalCombatAction::Prepare,
        weapon: WeaponKind::IronSword,
    };
    client
        .commands
        .push_back(Phase4Command::Combat(request.clone()));

    assert!(client.combat_command_pending());
    assert!(!client.queue_cycle("local-fight", "combat-duplicate".to_owned()));
    assert!(!client.queue_cycle("guard", "combat-action-duplicate".to_owned()));

    client.commands.clear();
    client.in_flight_command = Some(Phase4Command::Combat(request));
    assert!(client.combat_command_pending());
    assert!(!client.queue_cycle("spell", "combat-in-flight".to_owned()));
}

#[test]
fn local_fight_cycles_through_readable_weapon_families() {
    assert_eq!(
        super::super::combat::next_combat_weapon(None),
        WeaponKind::IronSword
    );
    assert_eq!(
        super::super::combat::next_combat_weapon(Some(WeaponKind::IronSword)),
        WeaponKind::Spear
    );
    assert_eq!(
        super::super::combat::next_combat_weapon(Some(WeaponKind::Spear)),
        WeaponKind::Axe
    );
    assert_eq!(
        super::super::combat::next_combat_weapon(Some(WeaponKind::Axe)),
        WeaponKind::Bow
    );
    assert_eq!(
        super::super::combat::next_combat_weapon(Some(WeaponKind::Bow)),
        WeaponKind::Shield
    );
}
