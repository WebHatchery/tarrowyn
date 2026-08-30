use super::super::super::{ServerConfig, WorldRepository};
use super::{governance_request, guest};
use tarrowyn_protocol::{GovernanceAction, KnowledgeAction, KnowledgeRequest};

#[test]
fn governance_inspection_is_fresh_and_side_effect_free() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-governance-read-view");

    let before = repository
        .governance(
            &session.account_token,
            governance_request(GovernanceAction::Inspect, "same-beat-inspect"),
        )
        .unwrap()
        .data;
    let (completed_before, rejected_before, audit_count_before) = {
        let state = repository.state.lock().expect("repository lock");
        (
            state.phase6.completed_commands,
            state.phase6.rejected_commands,
            state.phase6.audits.len(),
        )
    };

    let mut claim_office = governance_request(GovernanceAction::ClaimOffice, "read-view-office");
    claim_office.office_id = Some("steward".to_owned());
    assert!(
        repository
            .governance(&session.account_token, claim_office)
            .unwrap()
            .data
            .accepted
    );

    let after = repository
        .governance(
            &session.account_token,
            governance_request(GovernanceAction::Inspect, "same-beat-inspect"),
        )
        .unwrap()
        .data;
    assert!(before.governance.offices[0].holder_account_id.is_none());
    assert_eq!(
        after.governance.offices[0].holder_account_id.as_deref(),
        Some(session.account_id.as_str())
    );
    let state = repository.state.lock().expect("repository lock");
    assert_eq!(state.phase6.completed_commands, completed_before + 1);
    assert_eq!(state.phase6.rejected_commands, rejected_before);
    assert_eq!(state.phase6.audits.len(), audit_count_before + 1);
}

#[test]
fn knowledge_inspection_is_fresh_and_does_not_fill_replay_storage() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-knowledge-read-view");
    let request = KnowledgeRequest {
        request_id: "same-beat-knowledge-inspect".to_owned(),
        action: KnowledgeAction::Inspect,
        knowledge_id: None,
        target_account_id: None,
    };

    let _ = repository
        .knowledge(&session.account_token, request.clone())
        .unwrap();
    {
        let state = repository.state.lock().expect("repository lock");
        assert!(state.phase4.request_results.is_empty());
        assert_eq!(state.phase6.completed_commands, 0);
    }

    let discovered = repository
        .knowledge(
            &session.account_token,
            KnowledgeRequest {
                request_id: "read-view-discover".to_owned(),
                action: KnowledgeAction::Discover,
                knowledge_id: Some("moonberry-tending".to_owned()),
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(discovered.accepted);

    let after = repository
        .knowledge(&session.account_token, request)
        .unwrap()
        .data;
    assert_eq!(after.knowledge.items[0].title, "Moonberry trellis method");
    let state = repository.state.lock().expect("repository lock");
    assert_eq!(state.phase6.completed_commands, 1);
    assert_eq!(
        state
            .phase4
            .request_results
            .values()
            .filter(|response| matches!(response, super::super::Phase4Response::Knowledge(_)))
            .count(),
        1
    );
}

#[test]
fn profession_inspection_projects_defaults_without_materializing_player_state() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-profession-read-view");

    let response = repository.professions(&session.account_token).unwrap().data;
    assert_eq!(response.profiles.len(), 1);
    assert_eq!(
        response.profiles[0].profession,
        tarrowyn_protocol::ProfessionKind::Farmer
    );
    assert_eq!(response.materials.wood, 3);

    let state = repository.state.lock().expect("repository lock");
    assert!(!state.phase4.profiles.contains_key(&session.client_key));
    assert!(!state.phase4.materials.contains_key(&session.client_key));
    assert!(!state.phase4.credentials.contains_key(&session.client_key));
}

#[test]
fn combat_status_projects_ready_state_without_inserting_a_record() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-combat-read-view");

    let response = repository
        .combat_status(&session.account_token)
        .unwrap()
        .data;
    assert_eq!(response.status, tarrowyn_protocol::LocalCombatStatus::Ready);
    assert_eq!(response.enemy_health, 3);

    let state = repository.state.lock().expect("repository lock");
    assert!(!state.phase4.combat.contains_key(&session.client_key));
}
