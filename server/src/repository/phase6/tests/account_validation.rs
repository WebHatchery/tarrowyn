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
