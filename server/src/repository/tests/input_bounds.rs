use super::{guest, repo};
use tarrowyn_protocol::{TradeAction, TradeRequest};

#[test]
fn trade_selectors_reject_unbounded_or_controlled_ids() {
    let repository = repo();
    let session = guest(&repository, "trade-input");

    let cases = [
        (
            Some("account\nwith-control".to_owned()),
            None,
            "invalid_recipient_account_id",
        ),
        (None, Some("x".repeat(161)), "invalid_trade_id"),
    ];
    for (recipient_account_id, trade_id, expected_code) in cases {
        let error = repository
            .trade(
                &session.account_token,
                TradeRequest {
                    request_id: format!("trade-input-{expected_code}"),
                    action: TradeAction::Review,
                    trade_id,
                    recipient_account_id,
                    offer: None,
                    request: None,
                },
            )
            .expect_err("invalid trade selector should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, expected_code);
    }
}
