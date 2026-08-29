use super::*;

#[test]
fn phase_three_contract_combat_recovery_and_chronicle_are_authoritative() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "frontier-player");
    let accept = repo
        .contract(
            &session.account_token,
            ContractRequest {
                request_id: "contract-accept".to_owned(),
                action: ContractAction::Accept,
                contract_id: "brambleback-watch".to_owned(),
            },
        )
        .unwrap()
        .data;
    assert!(accept.accepted);
    for (index, (dx, dy)) in [(1, 0), (1, 0), (1, 0), (1, 0), (0, -1), (0, -1)]
        .into_iter()
        .enumerate()
    {
        assert!(
            repo.movement(
                &session.account_token,
                MovementIntent {
                    request_id: format!("frontier-step-{index}"),
                    dx,
                    dy,
                },
            )
            .unwrap()
            .data
            .accepted
        );
    }
    for index in 0..3 {
        assert!(
            repo.contract(
                &session.account_token,
                ContractRequest {
                    request_id: format!("contract-progress-{index}"),
                    action: ContractAction::Progress,
                    contract_id: "brambleback-watch".to_owned(),
                },
            )
            .unwrap()
            .data
            .accepted
        );
    }
    let report = repo
        .contract(
            &session.account_token,
            ContractRequest {
                request_id: "contract-report".to_owned(),
                action: ContractAction::Report,
                contract_id: "brambleback-watch".to_owned(),
            },
        )
        .unwrap()
        .data;
    assert!(report.accepted);

    let seeds_before = repo
        .inventory(&session.account_token)
        .unwrap()
        .data
        .inventory
        .seeds;
    let knockout = repo
        .combat(
            &session.account_token,
            CombatRequest {
                request_id: "club-strike".to_owned(),
                action: CombatAction::Strike,
                weapon: WeaponKind::ImprovisedClub,
            },
        )
        .unwrap()
        .data;
    assert_eq!(
        knockout.outcome,
        Some(tarrowyn_protocol::CombatOutcome::KnockedOut)
    );
    assert!(knockout.player.knocked_out);
    assert_eq!(
        repo.inventory(&session.account_token)
            .unwrap()
            .data
            .inventory
            .seeds,
        seeds_before - 1
    );
    assert_eq!(
        repo.combat(
            &session.account_token,
            CombatRequest {
                request_id: "club-strike".to_owned(),
                action: CombatAction::Strike,
                weapon: WeaponKind::ImprovisedClub,
            },
        )
        .unwrap()
        .data,
        knockout
    );
    let recovery = repo
        .recovery(
            &session.account_token,
            tarrowyn_protocol::RecoveryRequest {
                request_id: "rescued".to_owned(),
                choice: tarrowyn_protocol::RecoveryChoice::AskRescuer,
            },
        )
        .unwrap()
        .data;
    assert!(recovery.accepted);
    assert!(!recovery.player.knocked_out);
    let chronicle = repo.chronicle(&session.account_token, 0).unwrap().data;
    assert!(chronicle
        .entries
        .iter()
        .any(|entry| entry.kind == "knockout"));
    assert!(repo
        .events(&session.account_token, 0)
        .unwrap()
        .data
        .events
        .iter()
        .any(|event| matches!(event.event, WorldEvent::Chronicle(_))));
}

#[test]
fn phase_three_household_and_claim_lifecycles_emit_recovery_events() {
    let repo = WorldRepository::new(ServerConfig {
        claim_reclaim_ticks: 2,
        session_ttl_seconds: 100,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "frontier-lifecycle");
    let claim = repo
        .claim(
            &session.account_token,
            ClaimRequest {
                request_id: "lifecycle-claim".to_owned(),
                action: ClaimAction::Request,
            },
        )
        .unwrap()
        .data;
    assert!(claim.accepted);

    for _ in 0..17 {
        repo.tick();
    }

    let opportunities = repo.opportunities(&session.account_token).unwrap().data;
    assert_eq!(
        opportunities.opportunities[0].status,
        HouseholdStatus::Departed
    );
    let inspected = repo
        .claim(
            &session.account_token,
            ClaimRequest {
                request_id: "lifecycle-inspect".to_owned(),
                action: ClaimAction::Inspect,
            },
        )
        .unwrap()
        .data;
    assert_eq!(inspected.claim.unwrap().status, ClaimStatus::Reclaimed);

    let chronicle = repo.chronicle(&session.account_token, 0).unwrap().data;
    let kinds: Vec<&str> = chronicle
        .entries
        .iter()
        .map(|entry| entry.kind.as_str())
        .collect();
    assert!(kinds.contains(&"arrival candidate"));
    assert!(kinds.contains(&"household arrival"));
    assert!(kinds.contains(&"household departure"));
    assert!(kinds.contains(&"claim reclaimed"));
}

#[test]
fn phase_three_claim_and_expedition_survive_as_durable_world_state() {
    let repo = WorldRepository::new(ServerConfig {
        movement_cooldown_ticks: 0,
        claim_reclaim_ticks: 2,
        ..ServerConfig::default()
    });
    let one = guest(&repo, "pioneer-one");
    let two = guest(&repo, "pioneer-two");
    let three = guest(&repo, "pioneer-three");
    assert!(
        repo.claim(
            &one.account_token,
            ClaimRequest {
                request_id: "claim".to_owned(),
                action: ClaimAction::Request,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    let announce = repo
        .expedition(
            &one.account_token,
            ExpeditionRequest {
                request_id: "announce".to_owned(),
                action: ExpeditionAction::Announce,
                expedition_id: None,
                role: Some(ExpeditionRole::Scout),
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: Some("Test Rest".to_owned()),
            },
        )
        .unwrap()
        .data;
    assert!(announce.accepted);
    for (session, role, id) in [
        (&two, ExpeditionRole::Farmer, "join-farmer"),
        (&three, ExpeditionRole::Builder, "join-builder"),
    ] {
        assert!(
            repo.expedition(
                &session.account_token,
                ExpeditionRequest {
                    request_id: id.to_owned(),
                    action: ExpeditionAction::Join,
                    expedition_id: Some("pioneer-1".to_owned()),
                    role: Some(role),
                    food: 0,
                    tools: 0,
                    materials: 0,
                    safety: 0,
                    outpost_name: None,
                },
            )
            .unwrap()
            .data
            .accepted
        );
    }
    assert!(
        repo.expedition(
            &one.account_token,
            ExpeditionRequest {
                request_id: "supply".to_owned(),
                action: ExpeditionAction::Supply,
                expedition_id: Some("pioneer-1".to_owned()),
                role: None,
                food: 6,
                tools: 3,
                materials: 8,
                safety: 3,
                outpost_name: None,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    assert!(
        repo.expedition(
            &one.account_token,
            ExpeditionRequest {
                request_id: "launch".to_owned(),
                action: ExpeditionAction::Launch,
                expedition_id: Some("pioneer-1".to_owned()),
                role: None,
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    let resolved = repo
        .expedition(
            &one.account_token,
            ExpeditionRequest {
                request_id: "resolve".to_owned(),
                action: ExpeditionAction::Resolve,
                expedition_id: Some("pioneer-1".to_owned()),
                role: None,
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .unwrap()
        .data;
    assert!(resolved.accepted);
    assert_eq!(
        resolved.expedition.unwrap().status,
        tarrowyn_protocol::ExpeditionStatus::Succeeded
    );
    assert!(repo
        .world(&two.account_token)
        .unwrap()
        .data
        .outpost
        .is_some());
}
