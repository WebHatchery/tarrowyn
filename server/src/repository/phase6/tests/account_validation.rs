use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{AuthLinkRequest, GuestSessionRequest};

const PROVIDER: &str = "webhatchery-identity-oidc";

#[test]
fn account_link_rejects_unbounded_or_controlled_display_names() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("account-name-validation".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    for (subject, display_name) in [
        ("long-name", "x".repeat(81)),
        ("control-name", "safe\nname".to_owned()),
    ] {
        let error = repository
            .auth_link(
                &session.account_token,
                AuthLinkRequest {
                    request_id: format!("validate-{subject}"),
                    provider: PROVIDER.to_owned(),
                    subject: subject.to_owned(),
                    display_name: Some(display_name),
                },
            )
            .expect_err("invalid display name should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, "invalid_display_name");
    }
}

#[test]
fn account_link_rejects_unbounded_or_controlled_provider_subjects() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("account-subject-validation".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    for (subject, request_id) in [
        ("x".repeat(161), "long-subject".to_owned()),
        ("safe\nsubject".to_owned(), "control-subject".to_owned()),
    ] {
        let error = repository
            .auth_link(
                &session.account_token,
                AuthLinkRequest {
                    request_id,
                    provider: PROVIDER.to_owned(),
                    subject,
                    display_name: None,
                },
            )
            .expect_err("invalid provider subject should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, "invalid_subject");
    }
}

#[test]
fn account_link_replays_a_lost_response_through_the_rotated_guest_token() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("account-link-replay".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let request = AuthLinkRequest {
        request_id: "link-replay".to_owned(),
        provider: PROVIDER.to_owned(),
        subject: "link-replay-subject".to_owned(),
        display_name: Some("Replay resident".to_owned()),
    };

    let linked = repository
        .auth_link(&session.account_token, request.clone())
        .expect("initial account link")
        .data;
    assert!(repository.account(&session.account_token).is_err());

    let replay = repository
        .auth_link(&session.account_token, request)
        .expect("the rotated guest token should replay its link result")
        .data;
    assert_eq!(replay, linked);

    let error = repository
        .auth_link(
            &session.account_token,
            AuthLinkRequest {
                request_id: "different-request".to_owned(),
                provider: PROVIDER.to_owned(),
                subject: "link-replay-subject".to_owned(),
                display_name: None,
            },
        )
        .expect_err("the rotated token must not authorize a new link request");
    assert_eq!(error.status, 401);
}

#[test]
fn guest_session_rejects_unbounded_or_controlled_client_keys() {
    let repository = WorldRepository::new(ServerConfig::default());

    for (index, client_key) in ["x".repeat(129), "stable\tkey".to_owned()]
        .into_iter()
        .enumerate()
    {
        let error = repository
            .guest_session(GuestSessionRequest {
                client_key: Some(client_key),
                reset: false,
            })
            .expect_err("invalid client key should be rejected");
        assert_eq!(error.status, 400, "fixture {index}");
        assert_eq!(error.error.code, "invalid_client_key", "fixture {index}");
    }
}
