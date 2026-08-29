use crate::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    AdventurerRank, ContractAction, ContractRequest, Expedition, ExpeditionAction,
    ExpeditionMember, ExpeditionRequest, ExpeditionRole, ExpeditionStatus, GuestSessionRequest,
    Position,
};

#[test]
fn completing_a_watch_earns_a_trailhand_credential() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("adventurer-rank".to_owned()),
            reset: false,
        })
        .expect("guest session");
    repository
        .contract(
            &session.data.account_token,
            ContractRequest {
                request_id: "rank-accept".to_owned(),
                action: ContractAction::Accept,
                contract_id: "brambleback-watch".to_owned(),
            },
        )
        .unwrap();
    {
        let mut state = repository.state.lock().expect("state lock");
        state
            .identities
            .get_mut(&session.data.client_key)
            .expect("identity exists")
            .position = Position { x: 10, y: 4 };
    }
    for index in 0..3 {
        repository
            .contract(
                &session.data.account_token,
                ContractRequest {
                    request_id: format!("rank-progress-{index}"),
                    action: ContractAction::Progress,
                    contract_id: "brambleback-watch".to_owned(),
                },
            )
            .unwrap();
    }
    let report = repository
        .contract(
            &session.data.account_token,
            ContractRequest {
                request_id: "rank-report".to_owned(),
                action: ContractAction::Report,
                contract_id: "brambleback-watch".to_owned(),
            },
        )
        .unwrap()
        .data;
    assert_eq!(report.player.adventurer_rank, AdventurerRank::Trailhand);
    assert_eq!(
        report.player.adventurer_credentials,
        vec!["Brambleback watch report".to_owned()]
    );
}

#[test]
fn expedition_completion_records_a_credential_beyond_the_current_expedition() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("adventurer-expedition-history".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("state lock");
        state.phase3.expedition = Some(Expedition {
            expedition_id: "history-expedition".to_owned(),
            outpost_name: "Lantern Rest".to_owned(),
            leader_account_id: session.account_id.clone(),
            members: vec![ExpeditionMember {
                account_id: session.account_id.clone(),
                display_name: session.display_name.clone(),
                role: ExpeditionRole::Scout,
            }],
            food: 0,
            tools: 0,
            materials: 0,
            safety: 0,
            status: ExpeditionStatus::Launched,
            outcome: None,
            outpost_position: Position { x: 14, y: 8 },
        });
    }
    let response = repository
        .expedition(
            &session.account_token,
            ExpeditionRequest {
                request_id: "history-expedition-resolve".to_owned(),
                action: ExpeditionAction::Resolve,
                expedition_id: Some("history-expedition".to_owned()),
                role: None,
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("expedition resolve")
        .data;
    assert!(response.accepted);

    let mut state = repository.state.lock().expect("state lock");
    state.phase3.expedition = None;

    let (_, credentials) = super::profile(&state, &session.client_key);
    assert_eq!(credentials, vec!["Lantern Rest expedition".to_owned()]);
}
