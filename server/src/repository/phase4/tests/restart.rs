use super::*;

#[test]
fn phase_four_records_survive_restart_and_missing_phase_four_data_migrates() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-phase4-restart-{}.json",
        std::process::id()
    ));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        movement_cooldown_ticks: 0,
        combat_action_cooldown_ticks: 1,
        ..ServerConfig::default()
    };
    let first = WorldRepository::new(config.clone());
    let session = guest(&first, "phase4-restart");
    for (index, (dx, dy)) in [(1, 0), (1, 0), (0, -1), (0, -1)].into_iter().enumerate() {
        first
            .movement(
                &session.account_token,
                tarrowyn_protocol::MovementIntent {
                    request_id: format!("restart-move-{index}"),
                    dx,
                    dy,
                },
            )
            .unwrap();
    }
    let prepared = first
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "restart-combat-prepare".to_owned(),
                action: LocalCombatAction::Prepare,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert_eq!(prepared.combat.action_available_at_tick, 1);
    let mut office = governance_request(GovernanceAction::ClaimOffice, "restart-office");
    office.office_id = Some("steward".to_owned());
    assert!(
        first
            .governance(&session.account_token, office)
            .unwrap()
            .data
            .accepted
    );
    drop(first);

    let resumed = WorldRepository::new(config.clone());
    let resumed_session = guest(&resumed, "phase4-restart");
    assert_eq!(resumed_session.character_id, session.character_id);
    assert_eq!(
        resumed
            .combat_status(&resumed_session.account_token)
            .unwrap()
            .data
            .action_available_at_tick,
        1
    );
    let governance = resumed
        .governance(
            &resumed_session.account_token,
            governance_request(GovernanceAction::Inspect, "restart-inspect"),
        )
        .unwrap()
        .data;
    assert_eq!(
        governance.governance.offices[0]
            .holder_account_id
            .as_deref(),
        Some(session.account_id.as_str())
    );

    let bytes = std::fs::read(&path).unwrap();
    let mut document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    document.as_object_mut().unwrap().remove("phase4");
    std::fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    let migrated = WorldRepository::new(config);
    let migrated_session = guest(&migrated, "phase4-restart");
    let defaults = migrated
        .governance(
            &migrated_session.account_token,
            governance_request(GovernanceAction::Inspect, "migration-inspect"),
        )
        .unwrap()
        .data;
    assert!(defaults
        .governance
        .offices
        .iter()
        .any(|office| office.vacant));
    let _ = std::fs::remove_file(path);
}
