use super::*;

fn config() -> crate::data::GameConfig {
    crate::data::GameConfig {
        game_name: "years_of_tarrowyn".to_owned(),
        display_name: "The Years of Tarrowyn".to_owned(),
        save_slot: "phase_0".to_owned(),
        version: "0.1.0".to_owned(),
        world_width: 3,
        world_height: 2,
        day_length_seconds: 180.0,
        starting_gold: 12,
        starting_skill: 1,
    }
}

#[test]
fn finished_expedition_cycle_announces_a_new_party() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;
    client.projection.expedition = Some(tarrowyn_protocol::Expedition {
        expedition_id: "pioneer-1".to_owned(),
        outpost_name: "Lantern Rest".to_owned(),
        leader_account_id: "account-1".to_owned(),
        members: Vec::new(),
        food: 12,
        tools: 6,
        materials: 16,
        safety: 6,
        status: tarrowyn_protocol::ExpeditionStatus::Succeeded,
        outcome: Some("The outpost stands.".to_owned()),
        outpost_position: tarrowyn_protocol::Position { x: 14, y: 8 },
    });

    client.queue_expedition_cycle();
    let Some(FrontierCommand::Expedition(request)) = client.frontier.commands.pop_front() else {
        panic!("a finished expedition should queue a new announcement");
    };
    assert_eq!(request.action, ExpeditionAction::Announce);
    assert_eq!(request.role, Some(ExpeditionRole::Scout));
}

#[test]
fn contract_cycle_waits_through_the_tavern_cooldown() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;
    client.frontier.contracts = vec![AdventurerContract {
        contract_id: "brambleback-watch".to_owned(),
        title: "Brambleback watch".to_owned(),
        description: "A repeatable watch.".to_owned(),
        target: tarrowyn_protocol::MonsterKind::Brambleback,
        progress: 3,
        required_progress: 3,
        reward_gold: 8,
        status: ContractStatus::Cooldown,
        completion_count: 1,
        available_at_tick: 10,
    }];

    client.queue_contract_cycle();
    assert!(client.frontier.commands.is_empty());
    assert!(client.status_message.contains("cooling down"));
}

#[test]
fn frontier_error_summary_keeps_an_api_rejection_code() {
    assert_eq!(
        super::short_error(
            "HTTP API error in 'POST /v1/contracts/brambleback-watch' [rate_limited]: Try again later."
        ),
        "HTTP API error in 'POST /v1/contracts/brambleback-watch' [rate_limited]: Try again later."
    );
}

#[test]
fn frontier_action_reports_a_full_command_queue() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;
    for index in 0..super::super::queue::MAX_PENDING_COMMANDS {
        client
            .frontier
            .commands
            .push_back(FrontierCommand::Contract(ContractRequest {
                request_id: format!("queued-{index}"),
                action: ContractAction::Accept,
                contract_id: "brambleback-watch".to_owned(),
            }));
    }

    client.queue_claim_cycle();

    assert!(client
        .status_message
        .contains("frontier action is not ready"));
    assert_eq!(
        client.frontier.commands.len(),
        super::super::queue::MAX_PENDING_COMMANDS
    );
}

#[test]
fn recovery_buttons_queue_each_authoritative_choice() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;

    for choice in [
        RecoveryChoice::SelfRecover,
        RecoveryChoice::AskRescuer,
        RecoveryChoice::PayHealer,
    ] {
        client.queue_recovery(choice);
        let Some(FrontierCommand::Recovery(request)) = client.frontier.commands.pop_front() else {
            panic!("a recovery choice should queue a recovery request");
        };
        assert_eq!(request.choice, choice);
    }
}

#[test]
fn abandoned_homestead_cycle_requests_a_new_lease() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;
    client.account = Some(tarrowyn_protocol::GuestSessionResponse {
        client_key: "client-1".to_owned(),
        account_id: "account-1".to_owned(),
        character_id: "character-1".to_owned(),
        display_name: "The traveller".to_owned(),
        account_token: "token".to_owned(),
        expires_in_seconds: 900,
    });
    client.projection.claim = Some(tarrowyn_protocol::LandClaim {
        claim_id: "homestead-1".to_owned(),
        owner_account_id: "account-1".to_owned(),
        owner_name: "The traveller".to_owned(),
        position: tarrowyn_protocol::Position { x: 10, y: 8 },
        lease_days: 3,
        last_active_tick: 1,
        reclaim_after_ticks: 20,
        status: tarrowyn_protocol::ClaimStatus::Abandoned,
    });

    client.queue_claim_cycle();
    let Some(FrontierCommand::Claim(request)) = client.frontier.commands.pop_front() else {
        panic!("an abandoned homestead should queue a new lease request");
    };
    assert_eq!(request.action, ClaimAction::Request);
}
