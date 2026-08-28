use crate::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    AdventurerRank, ContractAction, ContractRequest, GuestSessionRequest, Position,
};

#[test]
fn completing_a_watch_earns_a_trailhand_credential() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let session = repository.guest_session(GuestSessionRequest {
        client_key: Some("adventurer-rank".to_owned()),
        reset: false,
    });
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
