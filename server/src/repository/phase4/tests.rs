use super::super::{ServerConfig, WorldRepository};
mod claim_retention;
mod combat_actions;
mod farming;
mod governance_history_retention;
mod governance_retention;
mod household_history;
mod infrastructure_history;
mod input_validation;
mod knowledge;
mod lesson_retention;
mod numeric_boundaries;
mod professions;
mod service_order_retention;

use tarrowyn_protocol::{
    ClaimLifecycleAction, ClaimLifecycleRequest, GovernanceAction, GovernanceRequest,
    GuestSessionRequest, KnowledgeAction, KnowledgeRequest, LocalCombatAction, LocalCombatRequest,
    ProfessionAction, ProfessionKind, ProfessionRequest, WeaponKind,
};

fn guest(repo: &WorldRepository, key: &str) -> tarrowyn_protocol::GuestSessionResponse {
    repo.guest_session(GuestSessionRequest {
        client_key: Some(key.to_owned()),
        reset: false,
    })
    .expect("guest session")
    .data
}

fn governance_request(action: GovernanceAction, request_id: &str) -> GovernanceRequest {
    GovernanceRequest {
        request_id: request_id.to_owned(),
        action,
        office_id: None,
        proposal_id: None,
        public_action: None,
        target: None,
        cost: None,
        tax_rate_percent: None,
    }
}

#[test]
fn governance_public_work_is_authorised_costed_and_auditable() {
    let repo = WorldRepository::new(ServerConfig::default());
    let session = guest(&repo, "phase4-steward");
    let mut claim_office = governance_request(GovernanceAction::ClaimOffice, "office");
    claim_office.office_id = Some("steward".to_owned());
    assert!(
        repo.governance(&session.account_token, claim_office)
            .unwrap()
            .data
            .accepted
    );

    let mut propose = governance_request(GovernanceAction::Propose, "proposal");
    propose.public_action = Some(tarrowyn_protocol::PublicAction::RepairRoad);
    let proposed = repo
        .governance(&session.account_token, propose)
        .unwrap()
        .data;
    let proposal_id = proposed.governance.proposals[0].proposal_id.clone();

    let mut approve = governance_request(GovernanceAction::Approve, "approve");
    approve.proposal_id = Some(proposal_id.clone());
    assert!(
        repo.governance(&session.account_token, approve)
            .unwrap()
            .data
            .accepted
    );
    let mut complete = governance_request(GovernanceAction::Complete, "complete");
    complete.proposal_id = Some(proposal_id);
    let completed = repo
        .governance(&session.account_token, complete)
        .unwrap()
        .data;
    assert!(completed.accepted);
    assert_eq!(completed.governance.public_treasury, 40);
    assert!(completed
        .governance
        .decisions
        .iter()
        .any(|decision| decision.cost == 8));
    let road = repo
        .infrastructure(&session.account_token)
        .unwrap()
        .data
        .records;
    assert_eq!(
        road.iter()
            .find(|record| record.infrastructure_id == "north-road")
            .unwrap()
            .condition,
        100
    );
    assert!(repo
        .chronicle(&session.account_token, 0)
        .unwrap()
        .data
        .entries
        .iter()
        .any(|entry| entry.kind == "public action completed"));
}

#[test]
fn settlement_tax_is_bounded_daily_and_recorded_in_the_public_ledger() {
    let repo = WorldRepository::new(ServerConfig {
        starting_gold: 100,
        day_length_seconds: 1.0,
        world_seconds_per_tick: 1.0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-taxpayer");
    let mut claim_office = governance_request(GovernanceAction::ClaimOffice, "tax-office");
    claim_office.office_id = Some("steward".to_owned());
    assert!(
        repo.governance(&session.account_token, claim_office)
            .unwrap()
            .data
            .accepted
    );

    let mut set_tax = governance_request(GovernanceAction::SetTaxRate, "tax-rate");
    set_tax.tax_rate_percent = Some(7);
    let changed = repo
        .governance(&session.account_token, set_tax)
        .unwrap()
        .data;
    assert!(changed.accepted);
    assert_eq!(changed.governance.taxation.unwrap().rate_percent, 7);
    assert!(changed.governance.tax_ledger.is_empty());

    let mut excessive = governance_request(GovernanceAction::SetTaxRate, "tax-too-high");
    excessive.tax_rate_percent = Some(11);
    let rejected = repo
        .governance(&session.account_token, excessive)
        .unwrap()
        .data;
    assert!(!rejected.accepted);
    assert!(rejected.reason.unwrap().contains("0% and 10%"));

    repo.tick();
    let player = repo.inventory(&session.account_token).unwrap().data;
    assert_eq!(player.gold, 93);
    let ledger = repo
        .governance(
            &session.account_token,
            governance_request(GovernanceAction::Inspect, "tax-inspect"),
        )
        .unwrap()
        .data
        .governance;
    assert_eq!(ledger.public_treasury, 55);
    assert_eq!(ledger.tax_ledger.len(), 1);
    assert_eq!(ledger.tax_ledger[0].amount, 7);
    assert_eq!(ledger.tax_ledger[0].day, 2);

    repo.tick();
    let second_day = repo.inventory(&session.account_token).unwrap().data;
    assert_eq!(second_day.gold, 87);
}

#[test]
fn land_rights_complete_the_lifecycle_without_touching_character_state() {
    let repo = WorldRepository::new(ServerConfig {
        claim_reclaim_grace_ticks: 1,
        ..ServerConfig::default()
    });
    let one = guest(&repo, "phase4-land-one");
    let two = guest(&repo, "phase4-land-two");
    let requested = repo
        .claim_lifecycle(
            &one.account_token,
            ClaimLifecycleRequest {
                request_id: "request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    let claim_id = requested.claim.unwrap().claim_id;
    let approved = repo
        .claim_lifecycle(
            &one.account_token,
            ClaimLifecycleRequest {
                request_id: "approve".to_owned(),
                action: ClaimLifecycleAction::Approve,
                claim_id: Some(claim_id.clone()),
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(approved.claim.unwrap().building_access);
    let renewed = repo
        .claim_lifecycle(
            &one.account_token,
            ClaimLifecycleRequest {
                request_id: "renew".to_owned(),
                action: ClaimLifecycleAction::Renew,
                claim_id: Some(claim_id.clone()),
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    assert_eq!(
        renewed.claim.unwrap().status,
        tarrowyn_protocol::ClaimLifecycleStatus::Renewed
    );
    assert!(
        repo.claim_lifecycle(
            &one.account_token,
            ClaimLifecycleRequest {
                request_id: "transfer".to_owned(),
                action: ClaimLifecycleAction::Transfer,
                claim_id: Some(claim_id.clone()),
                target_account_id: Some(two.account_id.clone()),
            }
        )
        .unwrap()
        .data
        .accepted
    );
    assert!(
        repo.claim_lifecycle(
            &two.account_token,
            ClaimLifecycleRequest {
                request_id: "abandon".to_owned(),
                action: ClaimLifecycleAction::Abandon,
                claim_id: Some(claim_id.clone()),
                target_account_id: None,
            }
        )
        .unwrap()
        .data
        .accepted
    );
    let reclaimed = repo
        .claim_lifecycle(
            &two.account_token,
            ClaimLifecycleRequest {
                request_id: "reclaim".to_owned(),
                action: ClaimLifecycleAction::Reclaim,
                claim_id: Some(claim_id),
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    assert_eq!(
        reclaimed.claim.unwrap().status,
        tarrowyn_protocol::ClaimLifecycleStatus::Reclaimed
    );
    assert_eq!(repo.inventory(&one.account_token).unwrap().data.gold, 12);
}

#[test]
fn claim_actions_without_ids_select_the_relevant_actor_lease() {
    let repo = WorldRepository::new(ServerConfig::default());
    let first = guest(&repo, "phase4-claim-fallback-first");
    let second = guest(&repo, "phase4-claim-fallback-second");
    let first_claim = repo
        .claim_lifecycle(
            &first.account_token,
            ClaimLifecycleRequest {
                request_id: "fallback-first-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data
        .claim
        .expect("the first player should receive a lease");
    assert!(
        repo.claim_lifecycle(
            &first.account_token,
            ClaimLifecycleRequest {
                request_id: "fallback-first-approve".to_owned(),
                action: ClaimLifecycleAction::Approve,
                claim_id: Some(first_claim.claim_id),
                target_account_id: None,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    let second_claim = repo
        .claim_lifecycle(
            &second.account_token,
            ClaimLifecycleRequest {
                request_id: "fallback-second-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data
        .claim
        .expect("the second player should receive a lease");
    assert!(
        repo.claim_lifecycle(
            &second.account_token,
            ClaimLifecycleRequest {
                request_id: "fallback-second-approve".to_owned(),
                action: ClaimLifecycleAction::Approve,
                claim_id: Some(second_claim.claim_id),
                target_account_id: None,
            },
        )
        .unwrap()
        .data
        .accepted
    );
    let renewed = repo
        .claim_lifecycle(
            &second.account_token,
            ClaimLifecycleRequest {
                request_id: "fallback-second-renew".to_owned(),
                action: ClaimLifecycleAction::Renew,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    assert_eq!(
        renewed.claim.unwrap().owner_account_id.as_deref(),
        Some(second.account_id.as_str())
    );
}

#[test]
fn lease_expiry_uses_real_time_instead_of_the_accelerated_world_clock() {
    let repo = WorldRepository::new(ServerConfig {
        day_length_seconds: 1.0,
        world_seconds_per_tick: 10_000.0,
        ..ServerConfig::default()
    });
    let session = guest(&repo, "phase4-real-lease");
    let requested = repo
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "real-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    let claim_id = requested.claim.unwrap().claim_id;
    let approved = repo
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "real-approve".to_owned(),
                action: ClaimLifecycleAction::Approve,
                claim_id: Some(claim_id.clone()),
                target_account_id: None,
            },
        )
        .unwrap()
        .data
        .claim
        .unwrap();
    let ninety_days = 90 * 24 * 60 * 60;
    assert_eq!(approved.lease_days, 90);
    assert_eq!(
        approved.expires_at_unix_seconds - approved.started_at_unix_seconds,
        ninety_days
    );

    for _ in 0..5 {
        repo.tick();
    }
    let claims = repo.claims(&session.account_token).unwrap().data;
    assert_eq!(claims.lease_duration_days, 90);
    let claim = claims
        .claims
        .into_iter()
        .find(|claim| claim.claim_id == claim_id)
        .unwrap();
    assert_eq!(
        claim.status,
        tarrowyn_protocol::ClaimLifecycleStatus::Active
    );
}

#[test]
fn professions_knowledge_and_households_make_the_settlement_interdependent() {
    let repo = WorldRepository::new(ServerConfig::default());
    let requester = guest(&repo, "phase4-requester");
    let provider = guest(&repo, "phase4-provider");
    let order = repo
        .profession_order(
            &requester.account_token,
            ProfessionRequest {
                request_id: "create-order".to_owned(),
                action: ProfessionAction::CreateOrder,
                order_id: None,
                profession: Some(ProfessionKind::Carpenter),
                capability_id: None,
                service: Some("Repair the farmer's field tool".to_owned()),
                timing_score: None,
            },
        )
        .unwrap()
        .data
        .order
        .unwrap();
    assert!(
        repo.profession_order(
            &provider.account_token,
            ProfessionRequest {
                request_id: "learn-carpentry".to_owned(),
                action: ProfessionAction::LearnCapability,
                order_id: None,
                profession: Some(ProfessionKind::Carpenter),
                capability_id: None,
                service: None,
                timing_score: None,
            }
        )
        .unwrap()
        .data
        .accepted
    );
    assert!(
        repo.profession_order(
            &provider.account_token,
            ProfessionRequest {
                request_id: "accept-order".to_owned(),
                action: ProfessionAction::AcceptOrder,
                order_id: Some(order.order_id.clone()),
                profession: None,
                capability_id: None,
                service: None,
                timing_score: None,
            }
        )
        .unwrap()
        .data
        .accepted
    );
    let invalid_timing = repo
        .profession_order(
            &provider.account_token,
            ProfessionRequest {
                request_id: "invalid-timing".to_owned(),
                action: ProfessionAction::CompleteOrder,
                order_id: Some(order.order_id.clone()),
                profession: None,
                capability_id: None,
                service: None,
                timing_score: Some(101),
            },
        )
        .unwrap()
        .data;
    assert!(!invalid_timing.accepted);
    assert!(invalid_timing
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("0 to 100")));
    let completed = repo
        .profession_order(
            &provider.account_token,
            ProfessionRequest {
                request_id: "complete-order".to_owned(),
                action: ProfessionAction::CompleteOrder,
                order_id: Some(order.order_id),
                profession: None,
                capability_id: None,
                service: None,
                timing_score: Some(100),
            },
        )
        .unwrap()
        .data;
    assert_eq!(
        completed.order.unwrap().quality,
        100,
        "a centered timing result reaches the server's quality ceiling"
    );

    assert!(
        repo.knowledge(
            &requester.account_token,
            KnowledgeRequest {
                request_id: "discover".to_owned(),
                action: KnowledgeAction::Discover,
                knowledge_id: None,
                target_account_id: None,
            }
        )
        .unwrap()
        .data
        .accepted
    );
    assert!(
        !repo
            .knowledge(
                &requester.account_token,
                KnowledgeRequest {
                    request_id: "teach-self".to_owned(),
                    action: KnowledgeAction::Teach,
                    knowledge_id: Some("moonberry-tending".to_owned()),
                    target_account_id: Some(requester.account_id.clone()),
                },
            )
            .unwrap()
            .data
            .accepted
    );
    assert!(
        repo.knowledge(
            &requester.account_token,
            KnowledgeRequest {
                request_id: "teach".to_owned(),
                action: KnowledgeAction::Teach,
                knowledge_id: Some("moonberry-tending".to_owned()),
                target_account_id: Some(provider.account_id.clone()),
            }
        )
        .unwrap()
        .data
        .accepted
    );
    let teaching = repo
        .skills(&requester.account_token)
        .unwrap()
        .data
        .skills
        .into_iter()
        .find(|skill| skill.skill_id == "teaching")
        .expect("teaching root should be present");
    assert_eq!(teaching.mastery, 1);
    assert_eq!(teaching.status, tarrowyn_protocol::SkillStatus::Practising);
    assert!(
        repo.knowledge(
            &provider.account_token,
            KnowledgeRequest {
                request_id: "apply".to_owned(),
                action: KnowledgeAction::Apply,
                knowledge_id: Some("moonberry-tending".to_owned()),
                target_account_id: None,
            }
        )
        .unwrap()
        .data
        .accepted
    );
    repo.tick();
    let household = repo
        .households(&requester.account_token)
        .unwrap()
        .data
        .households
        .remove(0);
    assert_eq!(household.members.len(), 2);
    assert!(household.work.contains("miller") || household.work.contains("healer"));
}

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
