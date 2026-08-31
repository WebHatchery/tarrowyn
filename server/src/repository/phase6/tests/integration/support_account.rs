use super::*;

#[test]
fn support_account_view_is_operator_only_and_secret_free() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("support-view-operator".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let target = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("support-view-target".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    repository
        .chat(
            &target.account_token,
            ChatRequest {
                request_id: "support-view-chat".to_owned(),
                channel: "settlement".to_owned(),
                text: "A public history note.".to_owned(),
            },
        )
        .unwrap();
    repository
        .claim_lifecycle(
            &target.account_token,
            ClaimLifecycleRequest {
                request_id: "support-view-claim".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap();
    repository
        .trade(
            &target.account_token,
            TradeRequest {
                request_id: "support-view-trade".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(operator.account_id.clone()),
                offer: Some(TradeBundle {
                    seeds: 1,
                    ..TradeBundle::default()
                }),
                request: Some(TradeBundle::default()),
            },
        )
        .unwrap();

    let view = repository
        .support_account(&operator.account_token, &target.account_id)
        .unwrap()
        .data;
    assert_eq!(view.account.account_id, target.account_id);
    assert_eq!(view.account.character_id, target.character_id);
    assert!(view.account.guest_fixture);
    assert_eq!(view.claims.len(), 1);
    assert_eq!(view.trades.len(), 1);
    assert!(!view.chronicle.is_empty());
    assert!(view.event_cursor > 0);

    let forbidden = repository
        .support_account(&target.account_token, &target.account_id)
        .expect_err("ordinary players must not read support account views");
    assert_eq!(forbidden.status, 403);
    assert_eq!(forbidden.error.code, "support_operator_required");
    let missing = repository
        .support_account(&operator.account_token, "missing-account")
        .expect_err("unknown accounts should not produce an empty support view");
    assert_eq!(missing.status, 404);
}
