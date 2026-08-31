use super::*;
use tarrowyn_protocol::{AccountDeletionRequest, AuthLinkRequest, ChronicleEntry};

fn history_entry(name: &str) -> ChronicleEntry {
    ChronicleEntry {
        event_id: "privacy-history-entry".to_owned(),
        kind: "social".to_owned(),
        title: format!("{name} keeps the regional ledger"),
        text: format!("The Hearth remembers {name} beside the road."),
        created_tick: 1,
        cursor: 1,
    }
}

#[test]
fn account_lifecycle_updates_copied_settlement_history_names() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("settlement-history-privacy".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    {
        let mut state = repository.state.lock().unwrap();
        let entry = history_entry(&guest.display_name);
        state.phase3.chronicle.push_back(entry.clone());
        state.phase5.settlements[0].chronicle.push(entry);
    }

    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "settlement-history-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "settlement-history-subject".to_owned(),
                display_name: Some("Chronicle resident".to_owned()),
            },
        )
        .unwrap()
        .data;
    {
        let state = repository.state.lock().unwrap();
        let entry = &state.phase5.settlements[0].chronicle[0];
        assert!(entry.title.contains("Chronicle resident"));
        assert!(!entry.title.contains(&guest.display_name));
    }

    repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "settlement-history-delete".to_owned(),
                account_id: linked.account_id,
            },
        )
        .unwrap();
    repository.tick();

    let state = repository.state.lock().unwrap();
    let entry = &state.phase5.settlements[0].chronicle[0];
    assert!(entry.title.contains("Former resident"));
    assert!(!entry.title.contains("Chronicle resident"));
    assert!(entry.text.contains("Former resident"));
}

#[test]
fn account_link_keeps_expanded_history_fields_bounded() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("settlement-history-bound".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    {
        let mut state = repository.state.lock().unwrap();
        state.phase3.chronicle.push_back(ChronicleEntry {
            event_id: "bounded-history-entry".to_owned(),
            kind: "social".to_owned(),
            title: format!("{}{}", guest.display_name, "x".repeat(505)),
            text: format!("{}{}", guest.display_name, "y".repeat(505)),
            created_tick: 1,
            cursor: 1,
        });
    }

    repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "settlement-history-bound-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "settlement-history-bound-subject".to_owned(),
                display_name: Some("A much longer linked resident name".to_owned()),
            },
        )
        .unwrap();

    let state = repository.state.lock().unwrap();
    let entry = &state.phase3.chronicle[0];
    assert!(entry.title.contains("A much longer linked resident name"));
    assert!(entry.text.contains("A much longer linked resident name"));
    assert!(entry.title.chars().count() <= 512);
    assert!(entry.text.chars().count() <= 512);
}
