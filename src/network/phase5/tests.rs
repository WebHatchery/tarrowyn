use super::*;

#[test]
fn refresh_is_scheduled_before_a_production_session_expires() {
    assert_eq!(refresh_delay(0), 1.0);
    assert_eq!(refresh_delay(20), 15.0);
}

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

fn account_response(guest_fixture: bool) -> tarrowyn_protocol::AccountResponse {
    let account_id = if guest_fixture {
        "guest-1"
    } else {
        "account-1"
    };
    let character_id = if guest_fixture {
        "guest-character-1"
    } else {
        "character-1"
    };
    let display_name = if guest_fixture {
        "Guest"
    } else {
        "Linked traveller"
    };
    tarrowyn_protocol::AccountResponse {
        account_id: account_id.to_owned(),
        provider: if guest_fixture {
            "development-guest"
        } else {
            "webhatchery-identity-oidc"
        }
        .to_owned(),
        character_id: character_id.to_owned(),
        display_name: display_name.to_owned(),
        guest_fixture,
        privacy_policy_version: "2026-01".to_owned(),
        retention_note: "retained until deletion".to_owned(),
        session_expires_at_tick: 100,
        character: tarrowyn_protocol::PlayerProjection {
            account_id: account_id.to_owned(),
            character_id: character_id.to_owned(),
            display_name: display_name.to_owned(),
            position: tarrowyn_protocol::Position { x: 8, y: 6 },
            gold: 10,
            field_tool_condition: 20,
            field_weather: tarrowyn_protocol::FieldWeather::Clear,
            field_pest_pressure: 0,
            animal_condition: 10,
            animal_max_condition: 10,
            skill: 1,
            reputation: 0,
            adventurer_rank: tarrowyn_protocol::AdventurerRank::Unproven,
            adventurer_credentials: Vec::new(),
            inventory: tarrowyn_protocol::Inventory::default(),
            weapon: tarrowyn_protocol::WeaponKind::IronSword,
            knocked_out: false,
            injuries: 0,
            recovery_cost: 0,
        },
    }
}
