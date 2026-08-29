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
fn account_deletion_requires_two_taps_for_a_linked_account() {
    let mut client = Phase5Client::new();
    client.account = Some(account_response(false));

    client.queue_cycle("delete-account");
    assert!(client.deletion_armed);
    assert!(client.commands.is_empty());

    client.queue_cycle("delete-account");
    assert!(!client.deletion_armed);
    assert!(matches!(
        client.commands.front(),
        Some(Phase5Command::Delete(request)) if request.account_id == "account-1"
    ));
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
fn guest_account_cannot_arm_deletion() {
    let mut client = Phase5Client::new();
    client.account = Some(account_response(true));

    client.queue_cycle("delete-account");
    assert!(!client.deletion_armed);
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
