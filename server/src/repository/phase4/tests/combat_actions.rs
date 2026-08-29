use super::super::super::{ServerConfig, WorldRepository};
use super::guest;
use tarrowyn_protocol::{
    LocalCombatAction, LocalCombatRequest, RecoveryChoice, RecoveryRequest, SkillStatus,
    TravelAction, TravelRequest, WeaponKind,
};

#[test]
fn local_combat_accepts_one_opening_weapon_technique() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        combat_action_cooldown_ticks: 0,
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
    repo.movement(
        &session.account_token,
        tarrowyn_protocol::MovementIntent {
            request_id: "technique-away".to_owned(),
            dx: -1,
            dy: 0,
        },
    )
    .unwrap();
    let out_of_range = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "technique-out-of-range".to_owned(),
                action: LocalCombatAction::Strike,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(!out_of_range.accepted);
    assert_eq!(out_of_range.combat.turn, 0);
    assert!(out_of_range
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("Whisperwood")));
    repo.movement(
        &session.account_token,
        tarrowyn_protocol::MovementIntent {
            request_id: "technique-back".to_owned(),
            dx: 1,
            dy: 0,
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
fn local_combat_rejects_same_tick_actions_until_the_server_window_opens() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        combat_action_cooldown_ticks: 1,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-combat-timing");
    for (index, (dx, dy)) in [(1, 0), (1, 0), (0, -1), (0, -1)].into_iter().enumerate() {
        repo.movement(
            &session.account_token,
            tarrowyn_protocol::MovementIntent {
                request_id: format!("timing-move-{index}"),
                dx,
                dy,
            },
        )
        .unwrap();
    }
    let prepared = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "timing-prepare".to_owned(),
                action: LocalCombatAction::Prepare,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(prepared.accepted);
    assert_eq!(prepared.combat.action_available_at_tick, 1);

    let too_soon = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "timing-too-soon".to_owned(),
                action: LocalCombatAction::Strike,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(!too_soon.accepted);
    assert!(too_soon.reason.unwrap().contains("server beat 1"));
    assert_eq!(too_soon.combat.turn, 0);

    repo.tick();
    let after_window = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "timing-after-window".to_owned(),
                action: LocalCombatAction::Strike,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(after_window.accepted);
    assert_eq!(after_window.combat.turn, 1);
    assert_eq!(after_window.combat.enemy_health, 1);
}

#[test]
fn local_combat_can_spend_a_bandage_after_a_bounded_injury() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        combat_action_cooldown_ticks: 0,
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
        combat_action_cooldown_ticks: 0,
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
fn knocked_out_local_player_cannot_reenter_before_recovery() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        combat_action_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-combat-recovery-boundary");
    for (index, (dx, dy)) in [(1, 0), (1, 0), (0, -1), (0, -1)].into_iter().enumerate() {
        repo.movement(
            &session.account_token,
            tarrowyn_protocol::MovementIntent {
                request_id: format!("recovery-boundary-move-{index}"),
                dx,
                dy,
            },
        )
        .unwrap();
    }
    repo.local_combat(
        &session.account_token,
        LocalCombatRequest {
            request_id: "recovery-boundary-prepare".to_owned(),
            action: LocalCombatAction::Prepare,
            weapon: WeaponKind::Shield,
        },
    )
    .unwrap();
    repo.local_combat(
        &session.account_token,
        LocalCombatRequest {
            request_id: "recovery-boundary-injury".to_owned(),
            action: LocalCombatAction::Strike,
            weapon: WeaponKind::Shield,
        },
    )
    .unwrap();
    let knockout = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "recovery-boundary-knockout".to_owned(),
                action: LocalCombatAction::Strike,
                weapon: WeaponKind::Shield,
            },
        )
        .unwrap()
        .data;
    assert_eq!(
        knockout.combat.status,
        tarrowyn_protocol::LocalCombatStatus::KnockedOut
    );
    assert!(knockout.player.knocked_out);

    let bypass = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "recovery-boundary-bypass".to_owned(),
                action: LocalCombatAction::Prepare,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(!bypass.accepted);
    assert_eq!(
        bypass.combat.status,
        tarrowyn_protocol::LocalCombatStatus::KnockedOut
    );
    assert!(bypass
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("recovery")));

    let travel_bypass = repo
        .travel(
            &session.account_token,
            TravelRequest {
                request_id: "recovery-boundary-travel".to_owned(),
                action: TravelAction::Start,
                route_id: Some("north-pack-road".to_owned()),
                travel_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(!travel_bypass.accepted);
    assert!(travel_bypass
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("recovery")));

    let recovery = repo
        .recovery(
            &session.account_token,
            RecoveryRequest {
                request_id: "recovery-boundary-recover".to_owned(),
                choice: RecoveryChoice::AskRescuer,
            },
        )
        .unwrap()
        .data;
    assert!(recovery.accepted);
    assert!(!recovery.player.knocked_out);
    for (index, (dx, dy)) in [(1, 0), (1, 0), (1, 0), (1, 0), (0, -1), (0, -1), (0, -1)]
        .into_iter()
        .enumerate()
    {
        repo.movement(
            &session.account_token,
            tarrowyn_protocol::MovementIntent {
                request_id: format!("recovery-boundary-return-{index}"),
                dx,
                dy,
            },
        )
        .unwrap();
    }
    let prepared = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "recovery-boundary-after".to_owned(),
                action: LocalCombatAction::Prepare,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(prepared.accepted);
    assert_eq!(
        prepared.combat.status,
        tarrowyn_protocol::LocalCombatStatus::Engaged
    );
}

#[test]
fn local_combat_can_cast_one_wind_spark_per_encounter() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        combat_action_cooldown_ticks: 0,
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

#[test]
fn local_combat_records_spear_and_axe_experience() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        combat_action_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-weapon-families");
    for (index, (dx, dy)) in [(1, 0), (1, 0), (0, -1), (0, -1)].into_iter().enumerate() {
        repo.movement(
            &session.account_token,
            tarrowyn_protocol::MovementIntent {
                request_id: format!("weapon-move-{index}"),
                dx,
                dy,
            },
        )
        .unwrap();
    }
    for (weapon, skill_id) in [
        (WeaponKind::Spear, "spear-fighting"),
        (WeaponKind::Axe, "axe-fighting"),
    ] {
        repo.local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: format!("prepare-{}", weapon.label()),
                action: LocalCombatAction::Prepare,
                weapon,
            },
        )
        .unwrap();
        let victory = repo
            .local_combat(
                &session.account_token,
                LocalCombatRequest {
                    request_id: format!("strike-one-{}", weapon.label()),
                    action: LocalCombatAction::Strike,
                    weapon,
                },
            )
            .unwrap();
        assert!(victory.data.accepted);
        let victory = repo
            .local_combat(
                &session.account_token,
                LocalCombatRequest {
                    request_id: format!("strike-two-{}", weapon.label()),
                    action: LocalCombatAction::Strike,
                    weapon,
                },
            )
            .unwrap();
        assert_eq!(
            victory.data.combat.status,
            tarrowyn_protocol::LocalCombatStatus::Victorious
        );
        let skill = repo
            .skills(&session.account_token)
            .unwrap()
            .data
            .skills
            .into_iter()
            .find(|skill| skill.skill_id == skill_id)
            .unwrap();
        assert_eq!(skill.mastery, 1);
        assert_eq!(skill.status, SkillStatus::Practising);
    }
}
