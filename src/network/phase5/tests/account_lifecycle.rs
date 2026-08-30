use super::*;
use tarrowyn_protocol::AuthLinkResponse;

#[test]
fn linked_production_session_replaces_the_guest_projection() {
    let mut client = Phase5Client::new();
    client.linked_account = Some(AuthLinkResponse {
        request_id: "link".to_owned(),
        provider: "webhatchery-identity-oidc".to_owned(),
        account_id: "account-1".to_owned(),
        character_id: "dev-character-1".to_owned(),
        display_name: "Linked traveller".to_owned(),
        session: tarrowyn_protocol::AuthSession {
            account_token: "prod-session-1".to_owned(),
            refresh_token: "prod-refresh-1".to_owned(),
            expires_in_seconds: 900,
            expires_at_tick: 3600,
        },
        linked_guest: true,
    });

    let account = client.take_linked_account(Some("guest-key")).unwrap();
    assert_eq!(account.client_key, "guest-key");
    assert_eq!(account.account_id, "account-1");
    assert_eq!(account.display_name, "Linked traveller");
    assert_eq!(account.account_token, "prod-session-1");
    assert!(client.take_linked_account(Some("guest-key")).is_none());
}

#[test]
fn linking_discards_an_in_flight_guest_account_projection() {
    let mut client = Phase5Client::new();
    client.pending_region = Some(Pending::failed("guest region still in flight"));
    client.pending_settlements = Some(Pending::failed("guest settlements still in flight"));
    client.pending_households = Some(Pending::failed("guest households still in flight"));
    client.pending_market = Some(Pending::failed("guest market still in flight"));
    client.pending_events = Some(Pending::failed("guest events still in flight"));
    client.pending_law = Some(Pending::failed("guest law still in flight"));
    client.pending_account = Some(Pending::failed("guest account still in flight"));
    client.market = Some(tarrowyn_protocol::MarketSnapshot {
        orders: Vec::new(),
        stock_notes: Vec::new(),
        prices: Vec::new(),
        cursor: 8,
    });
    client.refresh_timer = 4.0;
    let response = Phase5CommandResponse::Link(AuthLinkResponse {
        request_id: "link-race".to_owned(),
        provider: "webhatchery-identity-oidc".to_owned(),
        account_id: "account-1".to_owned(),
        character_id: "character-1".to_owned(),
        display_name: "Linked traveller".to_owned(),
        session: tarrowyn_protocol::AuthSession {
            account_token: "prod-session-1".to_owned(),
            refresh_token: "prod-refresh-1".to_owned(),
            expires_in_seconds: 900,
            expires_at_tick: 3600,
        },
        linked_guest: true,
    });
    let mut api = HttpClient::new("https://example.test");
    let mut notices = Vec::new();

    client.apply_command(response, None, &mut api, &mut notices);

    assert!(client.pending_region.is_none());
    assert!(client.pending_settlements.is_none());
    assert!(client.pending_households.is_none());
    assert!(client.pending_market.is_none());
    assert!(client.pending_events.is_none());
    assert!(client.pending_law.is_none());
    assert!(client.pending_account.is_none());
    assert!(client.account.is_none());
    assert!(client.market.is_none());
    assert_eq!(client.refresh_timer, 0.0);
    assert_eq!(client.refresh_token.as_deref(), Some("prod-refresh-1"));
}

#[test]
fn logout_signal_is_consumed_once() {
    let mut client = Phase5Client::new();
    client.logged_out = true;
    client.refresh_token = Some("refresh-secret".to_owned());
    client.refreshed_session = Some(tarrowyn_protocol::AuthSession {
        account_token: "access".to_owned(),
        refresh_token: "refresh".to_owned(),
        expires_in_seconds: 10,
        expires_at_tick: 10,
    });
    client.clear();
    assert!(client.refresh_token.is_none());
    assert!(client.refreshed_session.is_none());
    client.logged_out = true;
    assert!(client.take_logged_out());
    assert!(!client.take_logged_out());
}

#[test]
fn refresh_failure_discards_authenticated_projections() {
    let mut client = Phase5Client::new();
    client.pending_refresh = Some(Pending::failed("expired session"));
    client.account = Some(account_response(false));
    client.refresh_token = Some("refresh-secret".to_owned());
    client.region = Some(tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: Vec::new(),
        visible_settlements: Vec::new(),
        player_location_id: "hearth".to_owned(),
        travel: None,
        interest_radius: 12,
        cursor: 1,
    });
    client.market = Some(tarrowyn_protocol::MarketSnapshot {
        orders: Vec::new(),
        stock_notes: Vec::new(),
        prices: Vec::new(),
        cursor: 1,
    });

    let mut api = HttpClient::new("https://example.test");
    api.set_bearer_token(Some("expired-access"));
    let mut notices = Vec::new();
    client.poll_refresh(0.0, &mut api, &mut notices);

    assert!(client.pending_refresh.is_none());
    assert!(client.account.is_none());
    assert!(client.refresh_token.is_none());
    assert!(client.region.is_none());
    assert!(client.market.is_none());
    assert!(client.logged_out);
    assert_eq!(notices.len(), 1);
}

#[test]
fn transient_refresh_failure_retries_the_same_request() {
    let mut client = Phase5Client::new();
    let request = tarrowyn_protocol::AuthRefreshRequest {
        request_id: "refresh-retry".to_owned(),
        refresh_token: "refresh-secret".to_owned(),
    };
    client.pending_refresh = Some(Pending::failed(
        "HTTP request 'POST /v1/auth/refresh' timed out after 6.0 seconds",
    ));
    client.in_flight_refresh = Some(request.clone());
    client.refresh_token = Some(request.refresh_token.clone());

    let mut api = HttpClient::new("https://example.test");
    let mut notices = Vec::new();
    client.poll_refresh(0.0, &mut api, &mut notices);

    assert_eq!(client.in_flight_refresh, Some(request.clone()));
    assert_eq!(client.refresh_retry_count, 1);
    assert_eq!(client.refresh_retry_timer, 1.0);
    assert!(!client.logged_out);
    assert_eq!(notices.len(), 1);

    client.refresh_retry_timer = 0.0;
    client.dispatch(&mut api, false);
    assert!(client.pending_refresh.is_some());
    assert_eq!(client.in_flight_refresh, Some(request));
}

#[test]
fn regional_reads_wait_during_the_refresh_retry_window() {
    let mut client = Phase5Client::new();
    client.in_flight_refresh = Some(tarrowyn_protocol::AuthRefreshRequest {
        request_id: "refresh-window".to_owned(),
        refresh_token: "refresh-secret".to_owned(),
    });
    client.refresh_retry_timer = 1.0;
    client.refresh_timer = 0.0;

    let mut api = HttpClient::new("https://example.test");
    client.dispatch(&mut api, false);

    assert!(client.pending_refresh.is_none());
    assert!(client.pending_region.is_none());
    assert!(client.pending_settlements.is_none());
    assert!(client.pending_households.is_none());
    assert!(client.pending_market.is_none());
    assert!(client.pending_events.is_none());
    assert!(client.pending_law.is_none());
    assert!(client.pending_account.is_none());
}

#[test]
fn refresh_waits_for_commands_and_blocks_new_dispatch_until_rotation_finishes() {
    let mut client = Phase5Client::new();
    client.refresh_token = Some("refresh-secret".to_owned());
    client.auth_refresh_timer = 0.0;
    client.queue_report("queued-report".to_owned(), None, None);
    client.pending_command = Some(Pending::failed("command still in flight"));

    let mut api = HttpClient::new("https://example.test");
    client.dispatch_refresh(&mut api, false);
    assert!(client.pending_refresh.is_none());

    client.pending_command = None;
    client.dispatch(&mut api, false);
    assert!(client.pending_refresh.is_some());
    assert!(client.pending_command.is_none());
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Report(_))
    ));
}

#[test]
fn another_subsystem_mutation_blocks_refresh_and_regional_dispatch() {
    let mut client = Phase5Client::new();
    client.refresh_token = Some("refresh-secret".to_owned());
    client.auth_refresh_timer = 0.0;
    client.queue_report("queued-report".to_owned(), None, None);

    let mut api = HttpClient::new("https://example.test");
    let data = crate::data::GameData::load().expect("embedded game data should load");
    let mut projection = WorldProjection::new(&data.config);
    let mut notices = Vec::new();
    client.update(0.0, &mut api, &mut projection, true, true, &mut notices);

    assert!(client.pending_refresh.is_none());
    assert!(client.pending_command.is_none());
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Report(_))
    ));
}

#[test]
fn revoke_response_discards_authenticated_state() {
    let mut client = Phase5Client::new();
    client.account = Some(account_response(false));
    client.refresh_token = Some("refresh-secret".to_owned());
    client.region = Some(tarrowyn_protocol::RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: Vec::new(),
        routes: Vec::new(),
        visible_settlements: Vec::new(),
        player_location_id: "hearth".to_owned(),
        travel: None,
        interest_radius: 12,
        cursor: 1,
    });
    client.commands.push_back(Phase5Command::Report(
        tarrowyn_protocol::ModerationReportRequest {
            request_id: "queued".to_owned(),
            target_account_id: None,
            message_id: None,
            category: "player_report".to_owned(),
            note: "queued".to_owned(),
        },
    ));

    let response = serde_json::from_value::<Phase5CommandResponse>(serde_json::json!({
        "request_id": "revoke-1",
        "revoked_sessions": 1
    }))
    .expect("revoke response should decode");
    let mut api = HttpClient::new("https://example.test");
    let mut notices = Vec::new();
    client.apply_command(response, None, &mut api, &mut notices);

    assert!(client.account.is_none());
    assert!(client.refresh_token.is_none());
    assert!(client.region.is_none());
    assert!(client.commands.is_empty());
    assert!(client.logged_out);
    assert_eq!(notices.len(), 1);
}

#[test]
fn transient_command_failure_requeues_the_same_request() {
    let mut client = Phase5Client::new();
    let request = tarrowyn_protocol::AuthLinkRequest {
        request_id: "link-retry".to_owned(),
        provider: "webhatchery-identity-oidc".to_owned(),
        subject: "retry-subject".to_owned(),
        display_name: None,
    };
    client.pending_command = Some(Pending::failed(
        "HTTP request 'POST /v1/auth/link' timed out after 6.0 seconds",
    ));
    client.in_flight_command = Some(Phase5Command::Link(request));

    let mut api = HttpClient::new("https://example.test");
    let data = crate::data::GameData::load().expect("embedded game data should load");
    let mut projection = WorldProjection::new(&data.config);
    let mut notices = Vec::new();
    client.update(0.0, &mut api, &mut projection, true, false, &mut notices);

    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Link(request)) if request.request_id == "link-retry"
    ));
    assert_eq!(client.command_retry_count, 1);
    assert_eq!(client.command_retry_timer, 1.0);
    assert_eq!(notices.len(), 1);
}

#[test]
fn account_deletion_requires_two_taps_for_a_linked_account() {
    let mut client = Phase5Client::new();
    client.account = Some(account_response(false));

    assert!(client.account_deletion_available());

    client.queue_cycle("delete-account");
    assert!(client.deletion_armed);
    assert!(client.commands.is_empty());

    client.queue_cycle("delete-account");
    assert!(!client.deletion_armed);
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Delete(request)) if request.account_id == "account-1"
    ));
    assert!(client.identity_command_pending());
    assert!(!client.queue_cycle("logout"));
    assert_eq!(client.commands.len(), 1);
}

#[test]
fn report_queue_keeps_selected_account_and_chat_evidence() {
    let mut client = Phase5Client::new();

    assert!(client.queue_report(
        "report-1".to_owned(),
        Some("account-2".to_owned()),
        Some(17),
    ));
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Report(request))
            if request.target_account_id.as_deref() == Some("account-2")
                && request.message_id == Some(17)
    ));
}

#[test]
fn report_controls_wait_for_one_queued_or_in_flight_command() {
    let mut client = Phase5Client::new();
    let request = tarrowyn_protocol::ModerationReportRequest {
        request_id: "report-queued".to_owned(),
        target_account_id: Some("account-2".to_owned()),
        message_id: Some(17),
        category: "player_report".to_owned(),
        note: "queued".to_owned(),
    };
    client
        .commands
        .push_back(Phase5Command::Report(request.clone()));

    assert!(client.report_command_pending());
    assert!(!client.queue_cycle("report"));

    client.commands.clear();
    client.in_flight_command = Some(Phase5Command::Report(request));
    assert!(client.report_command_pending());
    assert!(!client.queue_report("report-in-flight".to_owned(), None, None));
}

#[test]
fn guest_account_cannot_arm_deletion() {
    let mut client = Phase5Client::new();
    client.account = Some(account_response(true));

    assert!(!client.account_deletion_available());
    client.queue_cycle("delete-account");
    assert!(!client.deletion_armed);
    assert!(client.commands.is_empty());
}

#[test]
fn linked_account_control_does_not_queue_a_second_link() {
    let mut client = Phase5Client::new();
    client.account = Some(account_response(false));

    assert!(!client.account_link_available());
    assert!(!client.queue_cycle("account"));
    assert!(client.commands.is_empty());

    client.account = Some(account_response(true));
    assert!(client.account_link_available());
    assert!(client.queue_cycle("account"));
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Link(request)) if request.provider == "webhatchery-identity-oidc"
    ));
    assert!(!client.account_link_available());
    assert!(!client.queue_cycle("account"));
    assert_eq!(client.commands.len(), 1);
}

#[test]
fn account_link_stays_closed_while_the_linked_account_projection_loads() {
    let mut client = Phase5Client::new();
    client.refresh_token = Some("production-refresh".to_owned());

    assert!(!client.account_link_available());
    assert!(!client.queue_cycle("account"));
    assert!(client.commands.is_empty());
}

#[test]
fn account_deletion_response_selects_its_own_command_variant() {
    let response = serde_json::from_value::<Phase5CommandResponse>(serde_json::json!({
        "request_id": "delete-1",
        "account_id": "account-1",
        "character_id": "character-1",
        "accepted": true,
        "status": "scheduled",
        "reason": null
    }))
    .expect("account deletion response should decode");
    assert!(matches!(response, Phase5CommandResponse::Delete(_)));
}

#[test]
fn rejected_account_deletion_without_a_reason_still_explains_the_outcome() {
    let response = Phase5CommandResponse::Delete(tarrowyn_protocol::AccountDeletionResponse {
        request_id: "delete-rejected".to_owned(),
        account_id: "account-1".to_owned(),
        character_id: "character-1".to_owned(),
        accepted: false,
        status: "blocked".to_owned(),
        reason: None,
    });
    let mut client = Phase5Client::new();
    let mut api = HttpClient::new("https://example.test");
    let mut notices = Vec::new();

    client.apply_command(response, None, &mut api, &mut notices);

    assert!(matches!(
        notices.first(),
        Some(NetworkNotice::Warning(message))
            if message == "The account deletion request was not accepted."
    ));
}
