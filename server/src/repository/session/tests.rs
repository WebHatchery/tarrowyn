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
fn expired_read_persists_presence_before_rejecting_access() {
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

    let error = repository.world(&session.account_token).unwrap_err();
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
