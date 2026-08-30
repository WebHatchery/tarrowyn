use super::super::super::super::{ServerConfig, WorldRepository};
use super::super::guest;
use tarrowyn_protocol::{LocalCombatAction, LocalCombatRequest, SkillStatus, WeaponKind};

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
