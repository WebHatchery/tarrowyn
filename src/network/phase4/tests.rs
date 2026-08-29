use super::*;
use tarrowyn_protocol::{
    LocalCombatAction, LocalCombatRequest, LocalCombatState, ProfessionAction, SkillAction,
    SkillLesson, SkillStatus, SkillView, SkillsResponse, WeaponKind,
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
fn phase_four_reset_discards_cached_ledgers() {
    let mut client = Phase4Client::new();
    client.skills = Some(SkillsResponse {
        skills: vec![SkillView {
            skill_id: "fishing".to_owned(),
            name: "Fishing".to_owned(),
            family: tarrowyn_protocol::SkillFamily::Gathering,
            depth: 1,
            mastery: 2,
            status: SkillStatus::Mastered,
            description: "Read water.".to_owned(),
            entry_hint: "Make a first catch.".to_owned(),
        }],
        lessons: Vec::new(),
        cursor: 3,
    });
    client.combat = Some(LocalCombatState {
        encounter_id: "encounter".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 2,
        player_health: 2,
        turn: 1,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: WeaponKind::IronSword,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: false,
    });

    client.clear();

    assert!(client.skills.is_none());
    assert!(client.combat.is_none());
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
fn crafting_completion_stays_available_when_the_command_queue_is_full() {
    let mut client = Phase4Client::new();
    client.begin_crafting("service-order-full");
    for index in 0..super::super::queue::MAX_PENDING_COMMANDS {
        client
            .commands
            .push_back(Phase4Command::Combat(LocalCombatRequest {
                request_id: format!("queued-{index}"),
                action: LocalCombatAction::Prepare,
                weapon: WeaponKind::IronSword,
            }));
    }

    assert!(!client.submit_crafting("craft-full".to_owned()));
    assert!(client.crafting_view().is_some());
    assert_eq!(
        client.commands.len(),
        super::super::queue::MAX_PENDING_COMMANDS
    );
}

#[test]
fn registry_button_chooses_the_current_account_claim() {
    let mut client = Phase4Client::new();
    client.own_account_id = Some("account-1".to_owned());
    client.claims = Some(ClaimsResponse {
        claims: vec![
            claim_for_test(
                "own-lease",
                Some("account-1"),
                tarrowyn_protocol::ClaimLifecycleStatus::Active,
            ),
            claim_for_test(
                "newer-other-lease",
                Some("account-2"),
                tarrowyn_protocol::ClaimLifecycleStatus::Active,
            ),
        ],
        available_plots: Vec::new(),
        lease_duration_days: 90,
        cursor: 2,
    });

    client.queue_cycle("registry", "registry-1".to_owned());
    let Some(Phase4Command::Claim(request)) = client.commands.pop_front() else {
        panic!("the registry should queue a claim lifecycle request");
    };
    assert_eq!(request.action, ClaimLifecycleAction::Renew);
    assert_eq!(request.claim_id.as_deref(), Some("own-lease"));

    client.commands.clear();
    client.own_account_id = Some("account-3".to_owned());
    client.queue_cycle("registry", "registry-2".to_owned());
    let Some(Phase4Command::Claim(request)) = client.commands.pop_front() else {
        panic!("a new account should queue a fresh claim request");
    };
    assert_eq!(request.action, ClaimLifecycleAction::Request);
    assert!(request.claim_id.is_none());

    client.commands.clear();
    client.own_account_id = Some("account-1".to_owned());
    client.governance = Some(tarrowyn_protocol::GovernanceState {
        settlement_id: "hearth".to_owned(),
        offices: vec![tarrowyn_protocol::OfficeRecord {
            office_id: "steward".to_owned(),
            kind: tarrowyn_protocol::OfficeKind::Steward,
            title: "Settlement Steward".to_owned(),
            authority: "Approve leases".to_owned(),
            holder_account_id: Some("account-1".to_owned()),
            holder_name: Some("The traveller".to_owned()),
            last_active_tick: 1,
            vacant: false,
            vacancy_reason: None,
        }],
        proposals: Vec::new(),
        decisions: Vec::new(),
        public_treasury: 10,
        administration_quality: 50,
        service_funding_until_tick: 0,
        taxation: None,
        tax_ledger: Vec::new(),
        cursor: 2,
    });
    client.claims.as_mut().expect("claim projection").claims[1].status =
        tarrowyn_protocol::ClaimLifecycleStatus::Requested;
    client.queue_cycle("registry", "registry-3".to_owned());
    let Some(Phase4Command::Claim(request)) = client.commands.pop_front() else {
        panic!("a steward should queue approval for a pending resident lease");
    };
    assert_eq!(request.action, ClaimLifecycleAction::Approve);
    assert_eq!(request.claim_id.as_deref(), Some("newer-other-lease"));
}

#[test]
fn order_button_waits_for_the_account_service_request() {
    let mut client = Phase4Client::new();
    client.own_account_id = Some("account-1".to_owned());
    client.professions = Some(ProfessionsResponse {
        profiles: Vec::new(),
        orders: vec![tarrowyn_protocol::ServiceOrder {
            order_id: "service-order-1".to_owned(),
            requester_account_id: "account-1".to_owned(),
            requester_name: "The traveller".to_owned(),
            provider_account_id: None,
            provider_name: None,
            service: "Repair a field tool".to_owned(),
            required_profession: ProfessionKind::Carpenter,
            materials: tarrowyn_protocol::MaterialStock::default(),
            tools_required: 1,
            reward_gold: 4,
            benefit: "A sound field tool".to_owned(),
            status: tarrowyn_protocol::ServiceOrderStatus::Open,
            quality: 0,
            created_tick: 1,
            completed_tick: None,
        }],
        materials: tarrowyn_protocol::MaterialStock::default(),
        credentials: Vec::new(),
        cursor: 1,
    });

    client.queue_cycle("order", "order-1".to_owned());
    assert!(client.commands.is_empty());
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
        lessons: Vec::new(),
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

#[test]
fn guard_button_queues_an_explicit_local_defense() {
    let mut client = Phase4Client::new();
    client.combat = Some(LocalCombatState {
        encounter_id: "whisperwood-local-1".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 3,
        player_health: 2,
        turn: 1,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: WeaponKind::Spear,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: false,
    });
    client.queue_cycle("guard", "guard-1".to_owned());
    let Some(Phase4Command::Combat(request)) = client.commands.pop_front() else {
        panic!("guard should queue a local combat request");
    };
    assert_eq!(request.action, LocalCombatAction::Guard);
    assert_eq!(request.weapon, WeaponKind::Spear);
}

#[test]
fn technique_button_queues_an_explicit_opening() {
    let mut client = Phase4Client::new();
    client.combat = Some(LocalCombatState {
        encounter_id: "whisperwood-local-1".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 3,
        player_health: 2,
        turn: 0,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: WeaponKind::IronSword,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: false,
    });
    client.queue_cycle("technique", "technique-1".to_owned());
    let Some(Phase4Command::Combat(request)) = client.commands.pop_front() else {
        panic!("technique should queue a local combat request");
    };
    assert_eq!(request.action, LocalCombatAction::Technique);
    assert_eq!(request.weapon, WeaponKind::IronSword);
}

#[test]
fn bandage_button_queues_an_explicit_item_use() {
    let mut client = Phase4Client::new();
    client.combat = Some(LocalCombatState {
        encounter_id: "whisperwood-local-1".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 2,
        player_health: 1,
        turn: 1,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: WeaponKind::Shield,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: false,
    });
    client.queue_cycle("item", "item-1".to_owned());
    let Some(Phase4Command::Combat(request)) = client.commands.pop_front() else {
        panic!("bandage should queue a local combat request");
    };
    assert_eq!(request.action, LocalCombatAction::UseItem);
    assert_eq!(request.weapon, WeaponKind::Shield);
}

#[test]
fn reposition_button_queues_an_explicit_movement_action() {
    let mut client = Phase4Client::new();
    client.combat = Some(LocalCombatState {
        encounter_id: "whisperwood-local-1".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 3,
        player_health: 2,
        turn: 0,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: WeaponKind::IronSword,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: false,
    });
    client.queue_cycle("reposition", "reposition-1".to_owned());
    let Some(Phase4Command::Combat(request)) = client.commands.pop_front() else {
        panic!("reposition should queue a local combat request");
    };
    assert_eq!(request.action, LocalCombatAction::Reposition);
    assert_eq!(request.weapon, WeaponKind::IronSword);
}

#[test]
fn spell_button_queues_an_explicit_cast() {
    let mut client = Phase4Client::new();
    client.combat = Some(LocalCombatState {
        encounter_id: "whisperwood-local-1".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 3,
        player_health: 2,
        turn: 0,
        status: tarrowyn_protocol::LocalCombatStatus::Engaged,
        weapon: WeaponKind::IronSword,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "A seed may be risked.".to_owned(),
        recovery_cost: 4,
        action_available_at_tick: 0,
        reposition_ready: false,
        spell_ready: true,
    });
    client.queue_cycle("spell", "spell-1".to_owned());
    let Some(Phase4Command::Combat(request)) = client.commands.pop_front() else {
        panic!("spell should queue a local combat request");
    };
    assert_eq!(request.action, LocalCombatAction::CastSpell);
    assert_eq!(request.weapon, WeaponKind::IronSword);
}

#[test]
fn discovered_storm_magic_changes_the_visible_spell_capability() {
    let mut client = Phase4Client::new();
    client.skills = Some(SkillsResponse {
        skills: vec![SkillView {
            skill_id: "storm-magic".to_owned(),
            name: "Storm Magic".to_owned(),
            family: tarrowyn_protocol::SkillFamily::Magic,
            depth: 2,
            mastery: 0,
            status: SkillStatus::Discovered,
            description: "A deliberate storm working.".to_owned(),
            entry_hint: "The three currents answer one another.".to_owned(),
        }],
        lessons: Vec::new(),
        cursor: 0,
    });

    assert!(client.storm_magic_unlocked());
}

#[test]
fn school_button_joins_an_open_lesson_for_the_learner() {
    let mut client = Phase4Client::new();
    client.own_account_id = Some("learner-1".to_owned());
    client.skills = Some(SkillsResponse {
        skills: Vec::new(),
        lessons: vec![SkillLesson {
            lesson_id: "school-lesson-1".to_owned(),
            teacher_account_id: "teacher-1".to_owned(),
            teacher_name: "Teacher".to_owned(),
            learner_account_id: "learner-1".to_owned(),
            learner_name: "Learner".to_owned(),
            skill_id: "sword-fighting".to_owned(),
            skill_name: "Sword Fighting".to_owned(),
            started_tick: 4,
            expires_tick: 24,
        }],
        cursor: 4,
    });
    assert!(client.queue_school("school-join".to_owned(), "teacher-1".to_owned()));
    let Some(Phase4Command::Skill(request)) = client.commands.pop_front() else {
        panic!("the learner should queue the open lesson");
    };
    assert_eq!(request.action, SkillAction::CompleteLesson);
    assert_eq!(request.lesson_id.as_deref(), Some("school-lesson-1"));
    assert_eq!(request.target_account_id.as_deref(), Some("teacher-1"));
}

#[test]
fn school_button_reports_a_full_command_queue() {
    let mut client = Phase4Client::new();
    client.own_account_id = Some("learner-1".to_owned());
    client.skills = Some(SkillsResponse {
        skills: Vec::new(),
        lessons: vec![SkillLesson {
            lesson_id: "school-lesson-full".to_owned(),
            teacher_account_id: "teacher-1".to_owned(),
            teacher_name: "Teacher".to_owned(),
            learner_account_id: "learner-1".to_owned(),
            learner_name: "Learner".to_owned(),
            skill_id: "sword-fighting".to_owned(),
            skill_name: "Sword Fighting".to_owned(),
            started_tick: 4,
            expires_tick: 24,
        }],
        cursor: 4,
    });
    for index in 0..super::super::queue::MAX_PENDING_COMMANDS {
        client
            .commands
            .push_back(Phase4Command::Combat(LocalCombatRequest {
                request_id: format!("queued-{index}"),
                action: LocalCombatAction::Prepare,
                weapon: WeaponKind::IronSword,
            }));
    }

    assert!(!client.queue_school("school-full".to_owned(), "teacher-1".to_owned()));
    assert_eq!(
        client.commands.len(),
        super::super::queue::MAX_PENDING_COMMANDS
    );
}

#[test]
fn phase_four_cycle_reports_when_no_command_is_ready() {
    let mut client = Phase4Client::new();

    assert!(!client.queue_cycle("practice", "practice-loading".to_owned()));
    assert!(client.commands.is_empty());
}

fn claim_for_test(
    claim_id: &str,
    owner_account_id: Option<&str>,
    status: tarrowyn_protocol::ClaimLifecycleStatus,
) -> tarrowyn_protocol::ClaimRecord {
    tarrowyn_protocol::ClaimRecord {
        claim_id: claim_id.to_owned(),
        plot_id: format!("plot-{claim_id}"),
        owner_account_id: owner_account_id.map(str::to_owned),
        owner_name: owner_account_id.map(|_| "Resident".to_owned()),
        position: tarrowyn_protocol::Position { x: 1, y: 1 },
        lease_days: 90,
        started_tick: 1,
        expires_tick: 100,
        started_at_unix_seconds: 1,
        expires_at_unix_seconds: 100,
        last_active_tick: 1,
        status,
        approved_by: None,
        building_access: true,
        protected_goods_policy: "Safe".to_owned(),
        inspection_note: "Recorded for the test.".to_owned(),
    }
}
