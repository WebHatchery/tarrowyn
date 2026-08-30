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
    let farmer = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("adventurer-expedition-farmer".to_owned()),
            reset: false,
        })
        .expect("farmer session")
        .data;
    let builder = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("adventurer-expedition-builder".to_owned()),
            reset: false,
        })
        .expect("builder session")
        .data;
    {
        let mut state = repository.state.lock().expect("state lock");
        state.phase3.expedition = Some(Expedition {
            expedition_id: "history-expedition".to_owned(),
            outpost_name: "Lantern Rest".to_owned(),
            leader_account_id: session.account_id.clone(),
            members: vec![
                ExpeditionMember {
                    account_id: session.account_id.clone(),
                    display_name: session.display_name.clone(),
                    role: ExpeditionRole::Scout,
                },
                ExpeditionMember {
                    account_id: farmer.account_id,
                    display_name: farmer.display_name,
                    role: ExpeditionRole::Farmer,
                },
                ExpeditionMember {
                    account_id: builder.account_id,
                    display_name: builder.display_name,
                    role: ExpeditionRole::Builder,
                },
            ],
            food: 6,
            tools: 3,
            materials: 8,
            safety: 3,
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

#[test]
fn expedition_supply_totals_remain_bounded_across_actions() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("adventurer-supply-boundary".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .expedition(
            &session.account_token,
            ExpeditionRequest {
                request_id: "supply-boundary-announce".to_owned(),
                action: ExpeditionAction::Announce,
                expedition_id: None,
                role: Some(ExpeditionRole::Scout),
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("announce expedition");
    for request_id in ["supply-boundary-first", "supply-boundary-second"] {
        let response = repository
            .expedition(
                &session.account_token,
                ExpeditionRequest {
                    request_id: request_id.to_owned(),
                    action: ExpeditionAction::Supply,
                    expedition_id: Some("pioneer-1".to_owned()),
                    role: None,
                    food: u32::MAX,
                    tools: u32::MAX,
                    materials: u32::MAX,
                    safety: u32::MAX,
                    outpost_name: None,
                },
            )
            .expect("supply expedition")
            .data;
        assert!(response.accepted);
        let expedition = response.expedition.expect("expedition projection");
        assert_eq!(expedition.food, 99);
        assert_eq!(expedition.tools, 99);
        assert_eq!(expedition.materials, 99);
        assert_eq!(expedition.safety, 99);
    }
}

#[test]
fn legacy_succeeded_expedition_is_backfilled_before_rotation() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("legacy-expedition-history".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("state lock");
        state.phase3.expedition = Some(Expedition {
            expedition_id: "legacy-expedition".to_owned(),
            outpost_name: "Lantern Rest".to_owned(),
            leader_account_id: session.account_id.clone(),
            members: vec![ExpeditionMember {
                account_id: session.account_id.clone(),
                display_name: session.display_name,
                role: ExpeditionRole::Scout,
            }],
            food: 0,
            tools: 0,
            materials: 0,
            safety: 0,
            status: ExpeditionStatus::Succeeded,
            outcome: Some("The old outpost stands.".to_owned()),
            outpost_position: Position { x: 14, y: 8 },
        });
    }
    let replacement = repository
        .expedition(
            &session.account_token,
            ExpeditionRequest {
                request_id: "legacy-expedition-replacement".to_owned(),
                action: ExpeditionAction::Announce,
                expedition_id: None,
                role: Some(ExpeditionRole::Scout),
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: Some("Second Lantern Rest".to_owned()),
            },
        )
        .expect("expedition replacement")
        .data;
    assert!(replacement.accepted);

    let mut state = repository.state.lock().expect("state lock");
    state.phase3.expedition = None;

    let (_, credentials) = super::profile(&state, &session.client_key);
    assert_eq!(credentials, vec!["Lantern Rest expedition".to_owned()]);
}
