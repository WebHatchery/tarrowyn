use super::*;

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
fn severe_weather_interactions_discover_and_power_storm_magic() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        combat_action_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-storm-magic");
    {
        let mut state = repo.state.lock().expect("repository lock");
        let zone_position = state.phase3.zone.position;
        let identity = state
            .identities
            .get_mut(&session.client_key)
            .expect("identity exists");
        identity.position = zone_position;
        for skill_id in ["wind-magic", "water-magic", "electricity-magic"] {
            identity.skills.practice.insert(skill_id.to_owned(), 16);
        }
        state.clock.day = 5;
    }

    for interaction in 0..25 {
        let prepared = repo
            .local_combat(
                &session.account_token,
                LocalCombatRequest {
                    request_id: format!("storm-prepare-{interaction}"),
                    action: LocalCombatAction::Prepare,
                    weapon: WeaponKind::IronSword,
                },
            )
            .unwrap()
            .data;
        assert!(prepared.accepted);
        let cast = repo
            .local_combat(
                &session.account_token,
                LocalCombatRequest {
                    request_id: format!("storm-interaction-{interaction}"),
                    action: LocalCombatAction::CastSpell,
                    weapon: WeaponKind::IronSword,
                },
            )
            .unwrap()
            .data;
        assert!(cast.accepted);
        assert_eq!(cast.combat.enemy_health, 1);
        repo.local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: format!("storm-retreat-{interaction}"),
                action: LocalCombatAction::Retreat,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap();
    }

    {
        let state = repo.state.lock().expect("repository lock");
        let identity = state
            .identities
            .get(&session.client_key)
            .expect("identity exists");
        assert!(identity
            .skills
            .known
            .iter()
            .any(|skill_id| skill_id == "storm-magic"));
        assert_eq!(
            identity.skills.qualifying_events.get("storm_interactions"),
            Some(&25)
        );
    }

    repo.local_combat(
        &session.account_token,
        LocalCombatRequest {
            request_id: "storm-technique-prepare".to_owned(),
            action: LocalCombatAction::Prepare,
            weapon: WeaponKind::IronSword,
        },
    )
    .unwrap();
    let storm = repo
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "storm-technique-cast".to_owned(),
                action: LocalCombatAction::CastSpell,
                weapon: WeaponKind::IronSword,
            },
        )
        .unwrap()
        .data;
    assert!(storm.accepted);
    assert_eq!(storm.combat.enemy_health, 0);
    assert_eq!(
        storm.combat.status,
        tarrowyn_protocol::LocalCombatStatus::Victorious
    );
}
