use super::*;
use crate::network::OnlineClient;
use tarrowyn_protocol::{ExpeditionAction, ExpeditionRole};

mod expedition;

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
        starting_seeds: 6,
        starting_skill: 1,
    }
}

fn player_projection(position: tarrowyn_protocol::Position) -> tarrowyn_protocol::PlayerProjection {
    tarrowyn_protocol::PlayerProjection {
        account_id: "account-1".to_owned(),
        character_id: "character-1".to_owned(),
        display_name: "Traveller".to_owned(),
        position,
        gold: 12,
        field_tool_condition: 3,
        field_weather: tarrowyn_protocol::FieldWeather::Clear,
        field_pest_pressure: 0,
        animal_condition: 10,
        animal_max_condition: 10,
        skill: 1,
        reputation: 0,
        adventurer_rank: tarrowyn_protocol::AdventurerRank::Unproven,
        adventurer_credentials: Vec::new(),
        inventory: tarrowyn_protocol::Inventory::default(),
        weapon: WeaponKind::IronSword,
        knocked_out: false,
        injuries: 0,
        recovery_cost: 0,
    }
}

#[test]
fn frontier_rejection_without_a_reason_still_leaves_a_visible_notice() {
    let mut notices = Vec::new();

    super::command_notice(false, None, "unused success", &mut notices);

    assert!(matches!(
        notices.first(),
        Some(NetworkNotice::Warning(message))
            if message == "The frontier action was not accepted."
    ));
}

#[test]
fn contract_rejection_without_a_reason_still_leaves_a_visible_notice() {
    let contract = AdventurerContract {
        contract_id: "brambleback-watch".to_owned(),
        title: "Brambleback watch".to_owned(),
        description: "A repeatable watch.".to_owned(),
        target: tarrowyn_protocol::MonsterKind::Brambleback,
        progress: 0,
        required_progress: 3,
        reward_gold: 8,
        status: ContractStatus::Available,
        completion_count: 0,
        available_at_tick: 0,
    };
    let response = FrontierCommandResponse::Contract(ContractResponse {
        request_id: "contract-rejected".to_owned(),
        accepted: false,
        contract,
        player: player_projection(tarrowyn_protocol::Position { x: 8, y: 6 }),
        reason: None,
    });
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    let mut notices = Vec::new();

    client
        .frontier
        .apply_command(response, &mut client.projection, &mut notices, true);

    assert!(matches!(
        notices.first(),
        Some(NetworkNotice::Warning(message))
            if message == "The frontier contract was not accepted."
    ));
}

#[test]
fn accepted_contract_response_restores_authoritative_player_position() {
    let contract = AdventurerContract {
        contract_id: "brambleback-watch".to_owned(),
        title: "Brambleback watch".to_owned(),
        description: "A repeatable watch.".to_owned(),
        target: tarrowyn_protocol::MonsterKind::Brambleback,
        progress: 1,
        required_progress: 3,
        reward_gold: 8,
        status: ContractStatus::Accepted,
        completion_count: 0,
        available_at_tick: 0,
    };
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.projection.forget_authoritative_player_position();
    let mut notices = Vec::new();

    client.frontier.apply_command(
        FrontierCommandResponse::Contract(ContractResponse {
            request_id: "contract-position".to_owned(),
            accepted: true,
            contract,
            player: player_projection(tarrowyn_protocol::Position { x: 11, y: 4 }),
            reason: None,
        }),
        &mut client.projection,
        &mut notices,
        true,
    );

    assert_eq!(
        client.projection.authoritative_player_position(),
        Some(macroquad_toolkit::grid::TilePos::new(11, 4))
    );
    assert_eq!(notices.len(), 1);
}

#[test]
fn expedition_rejection_without_a_reason_still_leaves_a_visible_notice() {
    let mut notices = Vec::new();

    super::expedition_notice(
        &ExpeditionResponse {
            request_id: "expedition-rejected".to_owned(),
            accepted: false,
            expedition: None,
            reason: None,
        },
        &mut notices,
    );

    assert!(matches!(
        notices.first(),
        Some(NetworkNotice::Warning(message))
            if message == "The pioneer action was not accepted."
    ));
}

#[test]
fn homestead_success_message_explains_lease_state() {
    let claim = tarrowyn_protocol::LandClaim {
        claim_id: "homestead-1".to_owned(),
        owner_account_id: "account-1".to_owned(),
        owner_name: "The traveller".to_owned(),
        position: tarrowyn_protocol::Position { x: 10, y: 8 },
        lease_days: 3,
        last_active_tick: 4,
        reclaim_after_ticks: 20,
        status: tarrowyn_protocol::ClaimStatus::Active,
    };
    assert_eq!(
        super::homestead_success_message(Some(&claim)),
        "Homestead lease active at plot (10, 8); 3-day access is recognised."
    );

    let mut abandoned = claim;
    abandoned.status = tarrowyn_protocol::ClaimStatus::Abandoned;
    assert_eq!(
        super::homestead_success_message(Some(&abandoned)),
        "Homestead lease abandoned at plot (10, 8); reclamation opens after 20 inactive beats."
    );
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
fn contract_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;
    let request = ContractRequest {
        request_id: "contract-queued".to_owned(),
        action: ContractAction::Accept,
        contract_id: "brambleback-watch".to_owned(),
    };
    client
        .frontier
        .commands
        .push_back(FrontierCommand::Contract(request.clone()));

    assert!(client.contract_pending());
    client.queue_contract_cycle();
    assert_eq!(client.frontier.commands.len(), 1);
    assert!(client
        .status_message
        .contains("frontier action is not ready"));

    client.frontier.commands.clear();
    client.frontier.in_flight_command = Some(FrontierCommand::Contract(request));
    assert!(client.contract_pending());
    client.queue_contract_cycle();
    assert!(client.frontier.commands.is_empty());
}

#[test]
fn expedition_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;
    let request = ExpeditionRequest {
        request_id: "expedition-queued".to_owned(),
        action: ExpeditionAction::Announce,
        expedition_id: Some("pioneer-1".to_owned()),
        role: Some(ExpeditionRole::Scout),
        food: 0,
        tools: 0,
        materials: 0,
        safety: 0,
        outpost_name: Some("Lantern Rest".to_owned()),
    };
    client
        .frontier
        .commands
        .push_back(FrontierCommand::Expedition(request.clone()));

    assert!(client.expedition_pending());
    client.queue_expedition_cycle();
    assert_eq!(client.frontier.commands.len(), 1);
    assert!(client
        .status_message
        .contains("frontier action is not ready"));

    client.frontier.commands.clear();
    client.frontier.in_flight_command = Some(FrontierCommand::Expedition(request));
    assert!(client.expedition_pending());
    client.queue_expedition_cycle();
    assert!(client.frontier.commands.is_empty());
}

#[test]
fn frontier_combat_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;
    let request = CombatRequest {
        request_id: "frontier-combat-queued".to_owned(),
        action: CombatAction::Retreat,
        weapon: WeaponKind::IronSword,
    };
    client
        .frontier
        .commands
        .push_back(FrontierCommand::Combat(request.clone()));

    assert!(client.frontier_combat_pending());
    client.queue_combat(CombatAction::Retreat, WeaponKind::IronSword);
    assert_eq!(client.frontier.commands.len(), 1);
    assert!(client
        .status_message
        .contains("frontier action is not ready"));

    client.frontier.commands.clear();
    client.frontier.in_flight_command = Some(FrontierCommand::Combat(request));
    assert!(client.frontier_combat_pending());
    client.queue_combat(CombatAction::Retreat, WeaponKind::IronSword);
    assert!(client.frontier.commands.is_empty());
}

#[test]
fn chronicle_search_queues_the_latest_query_for_the_frontier_reader() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;

    client.search_chronicle("Storm Magic");

    assert_eq!(
        client.frontier.chronicle_search_request.as_ref(),
        Some(&("Storm Magic".to_owned(), 0))
    );
    assert!(client.chronicle_search_pending());
    assert!(client.status_message.contains("durable chronicle"));
}

#[test]
fn contract_success_message_explains_active_progress() {
    let contract = AdventurerContract {
        contract_id: "brambleback-watch".to_owned(),
        title: "Brambleback watch".to_owned(),
        description: "A repeatable watch.".to_owned(),
        target: tarrowyn_protocol::MonsterKind::Brambleback,
        progress: 2,
        required_progress: 3,
        reward_gold: 8,
        status: ContractStatus::Accepted,
        completion_count: 0,
        available_at_tick: 0,
    };

    assert_eq!(
        super::contract_success_message(&contract),
        "Brambleback watch accepted • progress 2/3."
    );
}

#[test]
fn contract_success_message_explains_report_cooldown() {
    let contract = AdventurerContract {
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
    };

    assert_eq!(
        super::contract_success_message(&contract),
        "Brambleback watch reported • reward paid; available after beat 10."
    );
}

#[test]
fn frontier_refresh_error_keeps_an_api_rejection_code() {
    assert_eq!(
        super::refresh_error_notice(
            "tavern contracts",
            "HTTP API error in 'GET /v1/contracts' [rate_limited]: Try again later."
        ),
        "The tavern contracts could not be refreshed; reconnect or tap the visible control to retry. HTTP API error in 'GET /v1/contracts' [rate_limited]: Try again later."
    );
}

#[test]
fn transient_frontier_action_requeues_the_same_request() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    let request = ClaimRequest {
        request_id: "frontier-retry".to_owned(),
        action: ClaimAction::Request,
    };
    client.frontier.pending_command = Some(macroquad_toolkit::net::Pending::failed(
        "HTTP request 'POST /v1/claims' timed out after 6.0 seconds",
    ));
    client.frontier.in_flight_command = Some(FrontierCommand::Claim(request));

    let mut notices = Vec::new();
    client
        .frontier
        .update(&mut client.projection, 0.0, true, &mut notices);

    assert!(matches!(
        client.frontier.commands.front(),
        Some(FrontierCommand::Claim(request)) if request.request_id == "frontier-retry"
    ));
    assert_eq!(client.frontier.command_retry_count, 1);
    assert_eq!(client.frontier.command_retry_timer, 1.0);
    assert_eq!(notices.len(), 1);
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
        assert!(client.recovery_pending());
        let Some(FrontierCommand::Recovery(request)) = client.frontier.commands.pop_front() else {
            panic!("a recovery choice should queue a recovery request");
        };
        assert_eq!(request.choice, choice);
        assert!(!client.recovery_pending());
    }
}

#[test]
fn recovery_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;
    client.queue_recovery(RecoveryChoice::SelfRecover);
    assert!(client.recovery_pending());

    client.queue_recovery(RecoveryChoice::AskRescuer);
    assert!(client
        .status_message
        .contains("frontier action is not ready"));
    assert_eq!(client.frontier.commands.len(), 1);

    let Some(FrontierCommand::Recovery(request)) = client.frontier.commands.pop_front() else {
        panic!("the first recovery choice should remain queued");
    };
    client.frontier.in_flight_command = Some(FrontierCommand::Recovery(request));
    client.queue_recovery(RecoveryChoice::PayHealer);
    assert!(client
        .status_message
        .contains("frontier action is not ready"));
}

#[test]
fn frontier_claim_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = crate::network::ConnectionState::Online;
    let request = ClaimRequest {
        request_id: "claim-queued".to_owned(),
        action: ClaimAction::Request,
    };
    client
        .frontier
        .commands
        .push_back(FrontierCommand::Claim(request.clone()));

    assert!(client.frontier.claim_command_pending());
    client.queue_claim_cycle();
    assert!(client
        .status_message
        .contains("frontier action is not ready"));
    assert_eq!(client.frontier.commands.len(), 1);

    client.frontier.commands.clear();
    client.frontier.in_flight_command = Some(FrontierCommand::Claim(request));
    assert!(client.frontier.claim_command_pending());
    client.queue_claim(ClaimAction::Renew);
    assert!(client
        .status_message
        .contains("frontier action is not ready"));
}

#[test]
fn recovery_response_moves_the_map_to_the_hearth() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.projection.player_position = macroquad_toolkit::grid::TilePos::new(14, 8);
    let mut notices = Vec::new();

    super::apply_recovery(
        RecoveryResponse {
            request_id: "recover-position".to_owned(),
            accepted: true,
            choice: RecoveryChoice::SelfRecover,
            player: tarrowyn_protocol::PlayerProjection {
                account_id: "account-1".to_owned(),
                character_id: "character-1".to_owned(),
                display_name: "Traveller".to_owned(),
                position: tarrowyn_protocol::Position { x: 8, y: 5 },
                gold: 8,
                field_tool_condition: 3,
                field_weather: tarrowyn_protocol::FieldWeather::Clear,
                field_pest_pressure: 0,
                animal_condition: 10,
                animal_max_condition: 10,
                skill: 1,
                reputation: 0,
                adventurer_rank: tarrowyn_protocol::AdventurerRank::Unproven,
                adventurer_credentials: Vec::new(),
                inventory: tarrowyn_protocol::Inventory::default(),
                weapon: WeaponKind::IronSword,
                knocked_out: false,
                injuries: 0,
                recovery_cost: 0,
            },
            consequence: "Recovered at the Hearth.".to_owned(),
            reason: None,
        },
        &mut client.projection,
        &mut notices,
        true,
    );

    assert_eq!(
        client.projection.player_position,
        macroquad_toolkit::grid::TilePos::new(8, 5)
    );
    assert_eq!(notices.len(), 1);
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
