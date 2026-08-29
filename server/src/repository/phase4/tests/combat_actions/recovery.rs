use super::super::super::super::{ServerConfig, WorldRepository};
use super::super::guest;
use tarrowyn_protocol::{LocalCombatAction, LocalCombatRequest};

#[test]
fn local_combat_has_readable_recovery_and_safe_storage_rules() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        combat_action_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-combat");
    for (index, (dx, dy)) in [(1, 0), (1, 0), (0, -1), (0, -1)].into_iter().enumerate() {
        repo.movement(
            &session.account_token,
            tarrowyn_protocol::MovementIntent {
                request_id: format!("move-{index}"),
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
                request_id: "prepare".to_owned(),
                action: LocalCombatAction::Prepare,
                weapon: tarrowyn_protocol::WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(prepared.accepted);
    assert!(prepared.prompt.contains("TECHNIQUE"));
    let guarded = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "guard".to_owned(),
                action: LocalCombatAction::Guard,
                weapon: tarrowyn_protocol::WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(guarded.accepted);
    assert_eq!(guarded.combat.turn, 1);
    assert_eq!(guarded.combat.player_health, 2);
    let strike = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "strike".to_owned(),
                action: LocalCombatAction::Strike,
                weapon: tarrowyn_protocol::WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(strike.accepted);
    let second = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "strike-two".to_owned(),
                action: LocalCombatAction::Strike,
                weapon: tarrowyn_protocol::WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert_eq!(
        second.combat.status,
        tarrowyn_protocol::LocalCombatStatus::Victorious
    );
    assert!(second.combat.stored_property_safe);
}
