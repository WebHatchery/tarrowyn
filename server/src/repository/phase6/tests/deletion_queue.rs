use super::super::super::{ServerConfig, WorldRepository};
use super::super::deletion::PendingAccountDeletion;
use tarrowyn_protocol::{AccountDeletionRequest, AuthLinkRequest, GuestSessionRequest};

#[test]
fn deletion_queue_preserves_pending_work_and_coalesces_retries() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("deletion-queue-client".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "deletion-queue-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "deletion-queue-subject".to_owned(),
                display_name: None,
            },
        )
        .unwrap()
        .data;
    let first_request = AccountDeletionRequest {
        request_id: "deletion-queue-first".to_owned(),
        account_id: linked.account_id.clone(),
    };
    let scheduled = repository
        .account_delete(&linked.session.account_token, first_request)
        .unwrap()
        .data;
    assert!(scheduled.accepted);

    let retry = repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "deletion-queue-retry".to_owned(),
                account_id: linked.account_id.clone(),
            },
        )
        .unwrap()
        .data;
    assert!(retry.accepted);
    assert_eq!(retry.request_id, "deletion-queue-retry");
    assert_eq!(retry.status, "scheduled");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase6.deletion_requests.clear();
        for index in 0..super::super::MAX_PENDING_DELETIONS {
            state.phase6.deletion_requests.insert(
                format!("delete:other-account:{index}"),
                PendingAccountDeletion {
                    request_id: format!("other-{index}"),
                    account_id: format!("other-account-{index}"),
                    identity_key: format!("other-key-{index}"),
                    character_id: format!("other-character-{index}"),
                    replay_key: format!("other-fingerprint:{index}"),
                },
            );
        }
    }
    let blocked = repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "deletion-queue-blocked".to_owned(),
                account_id: linked.account_id,
            },
        )
        .unwrap()
        .data;
    assert!(!blocked.accepted);
    assert_eq!(blocked.status, "blocked");
    assert!(blocked.reason.unwrap().contains("queue is full"));
    let state = repository.state.lock().expect("repository lock");
    assert_eq!(
        state.phase6.deletion_requests.len(),
        super::super::MAX_PENDING_DELETIONS
    );
    assert_eq!(state.phase6.rejected_commands, 1);
}

#[test]
fn completed_deletion_replays_after_the_identity_is_removed() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("deletion-terminal-replay".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "deletion-terminal-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "deletion-terminal-subject".to_owned(),
                display_name: None,
            },
        )
        .unwrap()
        .data;
    let request = AccountDeletionRequest {
        request_id: "deletion-terminal-request".to_owned(),
        account_id: linked.account_id.clone(),
    };
    let scheduled = repository
        .account_delete(&linked.session.account_token, request.clone())
        .unwrap()
        .data;
    let coalesced = repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "deletion-terminal-coalesced".to_owned(),
                account_id: linked.account_id.clone(),
            },
        )
        .expect("the pending deletion should coalesce")
        .data;
    repository.tick();

    let replay = repository
        .account_delete(&linked.session.account_token, request)
        .expect("the terminal deletion result should replay")
        .data;

    assert_eq!(replay, scheduled);
    let coalesced_replay = repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "deletion-terminal-coalesced".to_owned(),
                account_id: linked.account_id,
            },
        )
        .expect("the coalesced terminal deletion result should replay")
        .data;
    assert_eq!(coalesced_replay, coalesced);
    assert!(repository.state.lock().unwrap().identities.is_empty());
}

#[test]
fn coalesced_deletion_replay_survives_a_restart_before_terminal_cleanup() {
    let state_path = std::env::temp_dir().join(format!(
        "tarrowyn-deletion-coalesced-replay-{}.json",
        std::process::id()
    ));
    let config = ServerConfig {
        persistence_path: Some(state_path.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    };
    let repository = WorldRepository::new(config.clone());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("deletion-coalesced-restart".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "deletion-coalesced-restart-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "deletion-coalesced-restart-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("linked session")
        .data;
    let coalesced_request = AccountDeletionRequest {
        request_id: "deletion-coalesced-restart-retry".to_owned(),
        account_id: linked.account_id.clone(),
    };
    repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "deletion-coalesced-restart-first".to_owned(),
                account_id: linked.account_id.clone(),
            },
        )
        .expect("initial deletion request");
    let coalesced = repository
        .account_delete(&linked.session.account_token, coalesced_request.clone())
        .expect("coalesced deletion request")
        .data;
    drop(repository);

    let restored = WorldRepository::new(config);
    restored.tick();
    let replay = restored
        .account_delete(&linked.session.account_token, coalesced_request)
        .expect("coalesced terminal result should survive restart")
        .data;

    assert_eq!(replay, coalesced);
    assert!(restored.state.lock().unwrap().identities.is_empty());
    let _ = std::fs::remove_file(state_path);
}
