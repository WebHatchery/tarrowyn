use crate::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{GuestSessionRequest, WorldEvent};

#[test]
fn authentication_evicts_a_session_with_a_missing_character() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("orphaned-session".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("state lock");
        state.identities.remove(&session.client_key);
    }

    let error = repository.world(&session.account_token).unwrap_err();
    assert_eq!(error.status, 401);
    assert_eq!(error.error.code, "unauthorized");

    let state = repository.state.lock().expect("state lock");
    assert!(!state.sessions.contains_key(&session.account_token));
}

#[test]
fn guest_session_expires_at_the_configured_tick_boundary() {
    let repository = WorldRepository::new(ServerConfig {
        session_ttl_seconds: 1,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("exact-session-expiry".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    {
        let mut state = repository.state.lock().expect("state lock");
        state.tick = repository.config.session_ttl_ticks();
    }

    let error = repository.world(&session.account_token).unwrap_err();
    assert_eq!(error.status, 401);
    assert_eq!(error.error.code, "unauthorized");
}

#[test]
fn guest_resume_records_departure_before_issuing_a_new_session() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        session_ttl_seconds: 1,
        ..ServerConfig::default()
    });
    let first = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("expired-guest-resume".to_owned()),
            reset: false,
        })
        .expect("first guest session")
        .data;

    {
        let mut state = repository.state.lock().expect("state lock");
        state.tick = repository.config.session_ttl_ticks();
    }

    let resumed = repository
        .guest_session(GuestSessionRequest {
            client_key: Some(first.client_key.clone()),
            reset: false,
        })
        .expect("guest resume")
        .data;
    assert_eq!(resumed.account_id, first.account_id);
    assert_ne!(resumed.account_token, first.account_token);

    let state = repository.state.lock().expect("state lock");
    assert!(!state.sessions.contains_key(&first.account_token));
    assert!(state.events.iter().any(|record| matches!(
        &record.event,
        WorldEvent::Presence(presence)
            if !presence.online && presence.account_id == first.account_id
    )));
}

#[test]
fn expired_account_read_persists_presence_before_rejecting_access() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-session-expiry-read-{}.json",
        std::process::id()
    ));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        backup_path: None,
        session_ttl_seconds: 1,
        ..ServerConfig::default()
    };
    let repository = WorldRepository::new(config.clone());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("persisted-expiry-read".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    {
        let mut state = repository.state.lock().expect("state lock");
        state.tick = repository.config.session_ttl_ticks();
    }

    let error = repository.account(&session.account_token).unwrap_err();
    assert_eq!(error.status, 401);
    assert_eq!(error.error.code, "unauthorized");

    let offline_cursor = repository
        .state
        .lock()
        .expect("state lock")
        .events
        .iter()
        .find_map(|record| match &record.event {
            WorldEvent::Presence(presence)
                if !presence.online && presence.account_id == session.account_id =>
            {
                Some(record.cursor)
            }
            _ => None,
        })
        .expect("expired session should emit an offline event");
    drop(repository);

    let restored = WorldRepository::new(config);
    let state = restored.state.lock().expect("state lock");
    assert_eq!(state.cursor, offline_cursor);
    assert!(state.events.iter().any(|record| {
        matches!(
            &record.event,
            WorldEvent::Presence(presence)
                if !presence.online && presence.account_id == session.account_id
        )
    }));
    drop(state);
    let _ = std::fs::remove_file(path);
}

#[test]
fn expired_revoke_attempt_persists_presence_before_rejecting_access() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        session_ttl_seconds: 1,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("expired-revoke-presence".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    {
        let mut state = repository.state.lock().expect("state lock");
        state.tick = repository.config.session_ttl_ticks();
    }

    let error = repository
        .auth_revoke(
            &session.account_token,
            tarrowyn_protocol::AuthRevokeRequest {
                request_id: "expired-revoke".to_owned(),
                revoke_all: false,
            },
        )
        .unwrap_err();
    assert_eq!(error.status, 401);
    assert!(repository
        .state
        .lock()
        .expect("state lock")
        .events
        .iter()
        .any(|record| matches!(
            &record.event,
            WorldEvent::Presence(presence)
                if !presence.online && presence.account_id == session.account_id
        )));
}

#[test]
fn revoking_the_last_session_records_an_offline_presence() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("revoke-last-session-presence".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    repository
        .auth_revoke(
            &session.account_token,
            tarrowyn_protocol::AuthRevokeRequest {
                request_id: "revoke-last-session-presence-request".to_owned(),
                revoke_all: false,
            },
        )
        .expect("revoke session");

    let state = repository.state.lock().expect("state lock");
    assert!(state.events.iter().any(|record| matches!(
        &record.event,
        WorldEvent::Presence(presence)
            if !presence.online && presence.account_id == session.account_id
    )));
}

#[test]
fn revoking_one_of_two_sessions_keeps_presence_online_until_the_last_leaves() {
    let repository = WorldRepository::new(ServerConfig::default());
    let first = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("two-session-presence".to_owned()),
            reset: false,
        })
        .expect("first guest session")
        .data;
    let second_token = "dev-session-secondary".to_owned();
    {
        let mut state = repository.state.lock().expect("state lock");
        let last_seen_tick = state.tick;
        state.sessions.insert(
            second_token.clone(),
            super::super::models::Session {
                client_key: first.client_key.clone(),
                identity_key: first.client_key.clone(),
                last_seen_tick,
                last_movement_tick: None,
                last_chat_tick: None,
            },
        );
    }
    let cursor_before_revoke = repository
        .world(&second_token)
        .unwrap()
        .meta
        .cursor
        .expect("world response cursor");

    repository
        .auth_revoke(
            &first.account_token,
            tarrowyn_protocol::AuthRevokeRequest {
                request_id: "revoke-one-session-presence".to_owned(),
                revoke_all: false,
            },
        )
        .expect("revoke first session");

    let events = repository
        .events(&second_token, cursor_before_revoke)
        .expect("remaining session should read the event stream")
        .data
        .events;
    assert!(!events.iter().any(|record| matches!(
        &record.event,
        WorldEvent::Presence(presence)
            if !presence.online && presence.account_id == first.account_id
    )));
    assert_eq!(
        repository.world(&second_token).unwrap().data.players.len(),
        1
    );

    repository
        .auth_revoke(
            &second_token,
            tarrowyn_protocol::AuthRevokeRequest {
                request_id: "revoke-last-session-presence".to_owned(),
                revoke_all: false,
            },
        )
        .expect("revoke final session");
    let state = repository.state.lock().expect("state lock");
    assert_eq!(
        state
            .events
            .iter()
            .filter(|record| matches!(
                &record.event,
                WorldEvent::Presence(presence)
                    if !presence.online && presence.account_id == first.account_id
            ))
            .count(),
        1
    );
}

#[test]
fn expiring_multiple_sessions_records_one_departure_and_removes_duplicate_presence() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        session_ttl_seconds: 1,
        ..ServerConfig::default()
    });
    let first = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("expiry-duplicate-presence".to_owned()),
            reset: false,
        })
        .expect("first guest session")
        .data;
    let second_token = "dev-session-expiry-secondary".to_owned();
    {
        let mut state = repository.state.lock().expect("state lock");
        let last_seen_tick = state.tick;
        state.sessions.insert(
            second_token.clone(),
            super::super::models::Session {
                client_key: first.client_key.clone(),
                identity_key: first.client_key.clone(),
                last_seen_tick,
                last_movement_tick: None,
                last_chat_tick: None,
            },
        );
    }
    {
        let mut state = repository.state.lock().expect("state lock");
        state.tick = repository.config.session_ttl_ticks();
    }

    assert!(repository.world(&first.account_token).is_err());
    let state = repository.state.lock().expect("state lock");
    assert!(!state.sessions.contains_key(&first.account_token));
    assert!(!state.sessions.contains_key(&second_token));
    assert_eq!(
        state
            .events
            .iter()
            .filter(|record| matches!(
                &record.event,
                WorldEvent::Presence(presence)
                    if !presence.online && presence.account_id == first.account_id
            ))
            .count(),
        1
    );
}
