use super::*;

#[test]
fn support_account_view_keeps_the_latest_chronicle_window_bounded() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("support-chronicle-operator".to_owned()),
            reset: false,
        })
        .expect("operator session")
        .data;
    let target = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("support-chronicle-target".to_owned()),
            reset: false,
        })
        .expect("target session")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        for index in 0..(super::super::super::phase3::MAX_CHRONICLE + 160) {
            super::super::super::phase3::record(
                &mut state,
                "support history",
                &format!("Support history {index:03}"),
                "The operator view keeps the latest regional records.",
            );
        }
    }

    let view = repository
        .support_account(&operator.account_token, &target.account_id)
        .expect("support account view")
        .data;
    assert_eq!(view.chronicle.len(), 128);
    assert_eq!(
        view.chronicle.last().map(|entry| entry.title.as_str()),
        Some("Support history 223")
    );
}
