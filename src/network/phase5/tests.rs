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
