use super::super::super::{ServerConfig, WorldRepository};
use super::guest;
use tarrowyn_protocol::{LocalCombatAction, LocalCombatRequest, WeaponKind};

#[test]
fn local_combat_accepts_one_opening_weapon_technique() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-technique");
    for (index, (dx, dy)) in [(1, 0), (1, 0), (0, -1), (0, -1)].into_iter().enumerate() {
        repo.movement(
            &session.account_token,
            tarrowyn_protocol::MovementIntent {
                request_id: format!("technique-move-{index}"),
                dx,
                dy,
            },
        )
        .unwrap();
    }
    repo.local_combat(
        &session.account_token,
        LocalCombatRequest {
            request_id: "technique-prepare".to_owned(),
            action: LocalCombatAction::Prepare,
            weapon: WeaponKind::IronSword,
        },
    )
    .unwrap();
    let technique = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "technique-opening".to_owned(),
                action: LocalCombatAction::Technique,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(technique.accepted);
    assert_eq!(technique.combat.enemy_health, 1);
    assert_eq!(technique.combat.turn, 1);
    assert!(technique.prompt.contains("technique"));

    let repeated = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "technique-repeated".to_owned(),
                action: LocalCombatAction::Technique,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(!repeated.accepted);
    assert_eq!(repeated.combat.turn, 1);
    assert!(repeated.reason.unwrap().contains("opening"));
}

#[test]
fn local_combat_can_spend_a_bandage_after_a_bounded_injury() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-bandage");
    for (index, (dx, dy)) in [(1, 0), (1, 0), (0, -1), (0, -1)].into_iter().enumerate() {
        repo.movement(
            &session.account_token,
            tarrowyn_protocol::MovementIntent {
                request_id: format!("bandage-move-{index}"),
                dx,
                dy,
            },
        )
        .unwrap();
    }
    repo.local_combat(
        &session.account_token,
        LocalCombatRequest {
            request_id: "bandage-prepare".to_owned(),
            action: LocalCombatAction::Prepare,
            weapon: WeaponKind::Shield,
        },
    )
    .unwrap();
    let injured = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "bandage-injury".to_owned(),
                action: LocalCombatAction::Strike,
                weapon: WeaponKind::Shield,
            },
        )
        .unwrap()
        .data;
    assert_eq!(injured.combat.player_health, 1);
    let treated = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "bandage-use".to_owned(),
                action: LocalCombatAction::UseItem,
                weapon: WeaponKind::Shield,
            },
        )
        .unwrap()
        .data;
    assert!(treated.accepted);
    assert_eq!(treated.combat.player_health, 2);
    assert_eq!(treated.combat.turn, 2);
    assert_eq!(treated.player.inventory.bandages, 0);
}

#[test]
fn local_combat_reposition_protects_the_next_strike() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-reposition");
    for (index, (dx, dy)) in [(1, 0), (1, 0), (0, -1), (0, -1)].into_iter().enumerate() {
        repo.movement(
            &session.account_token,
            tarrowyn_protocol::MovementIntent {
                request_id: format!("reposition-move-{index}"),
                dx,
                dy,
            },
        )
        .unwrap();
    }
    repo.local_combat(
        &session.account_token,
        LocalCombatRequest {
            request_id: "reposition-prepare".to_owned(),
            action: LocalCombatAction::Prepare,
            weapon: WeaponKind::Shield,
        },
    )
    .unwrap();
    let repositioned = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "reposition-step".to_owned(),
                action: LocalCombatAction::Reposition,
                weapon: WeaponKind::Shield,
            },
        )
        .unwrap()
        .data;
    assert!(repositioned.accepted);
    assert!(repositioned.combat.reposition_ready);
    let protected_strike = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "reposition-strike".to_owned(),
                action: LocalCombatAction::Strike,
                weapon: WeaponKind::Shield,
            },
        )
        .unwrap()
        .data;
    assert!(protected_strike.accepted);
    assert_eq!(protected_strike.combat.player_health, 2);
    assert!(!protected_strike.combat.reposition_ready);
}

#[test]
fn local_combat_can_cast_one_wind_spark_per_encounter() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-spell");
    for (index, (dx, dy)) in [(1, 0), (1, 0), (0, -1), (0, -1)].into_iter().enumerate() {
        repo.movement(
            &session.account_token,
            tarrowyn_protocol::MovementIntent {
                request_id: format!("spell-move-{index}"),
                dx,
                dy,
            },
        )
        .unwrap();
    }
    repo.local_combat(
        &session.account_token,
        LocalCombatRequest {
            request_id: "spell-prepare".to_owned(),
            action: LocalCombatAction::Prepare,
            weapon: WeaponKind::IronSword,
        },
    )
    .unwrap();
    let cast = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "spell-cast".to_owned(),
                action: LocalCombatAction::CastSpell,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(cast.accepted);
    assert_eq!(cast.combat.enemy_health, 1);
    assert!(!cast.combat.spell_ready);
    let wind = repo
        .skills(&session.account_token)
        .unwrap()
        .data
        .skills
        .into_iter()
        .find(|skill| skill.skill_id == "wind-magic")
        .unwrap();
    assert_eq!(wind.mastery, 1);

    let repeated = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "spell-repeated".to_owned(),
                action: LocalCombatAction::CastSpell,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(!repeated.accepted);
    assert!(repeated.reason.unwrap().contains("spent"));
}
