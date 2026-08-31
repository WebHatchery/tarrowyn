use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    AccountDeletionRequest, AuthLinkRequest, GuestSessionRequest, TradeAction, TradeBundle,
    TradeRequest,
};

#[test]
fn account_deletion_invalidates_trade_replays_kept_by_another_identity() {
    let repository = WorldRepository::new(ServerConfig::default());
    let creator_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("trade-replay-creator".to_owned()),
            reset: false,
        })
        .expect("creator guest session")
        .data;
    let creator = repository
        .auth_link(
            &creator_guest.account_token,
            AuthLinkRequest {
                request_id: "trade-replay-creator-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "trade-replay-creator-subject".to_owned(),
                display_name: Some("Departing trader".to_owned()),
            },
        )
        .expect("creator link")
        .data;
    let recipient = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("trade-replay-recipient".to_owned()),
            reset: false,
        })
        .expect("recipient guest session")
        .data;
    let created = repository
        .trade(
            &creator.session.account_token,
            TradeRequest {
                request_id: "trade-replay-create".to_owned(),
                action: TradeAction::Create,
                trade_id: None,
                recipient_account_id: Some(recipient.account_id.clone()),
                offer: Some(TradeBundle {
                    seeds: 1,
                    ..TradeBundle::default()
                }),
                request: Some(TradeBundle {
                    gold: 1,
                    ..TradeBundle::default()
                }),
            },
        )
        .expect("trade creation")
        .data;
    let trade_id = created.trade.expect("created trade").trade_id;
    let review_request = TradeRequest {
        request_id: "trade-replay-review".to_owned(),
        action: TradeAction::Review,
        trade_id: Some(trade_id),
        recipient_account_id: None,
        offer: None,
        request: None,
    };
    let reviewed = repository
        .trade(&recipient.account_token, review_request.clone())
        .expect("trade review")
        .data;
    assert_eq!(
        reviewed
            .trade
            .as_ref()
            .expect("reviewed trade")
            .creator_account_id,
        creator.account_id
    );

    repository
        .account_delete(
            &creator.session.account_token,
            AccountDeletionRequest {
                request_id: "trade-replay-creator-delete".to_owned(),
                account_id: creator.account_id,
            },
        )
        .expect("schedule creator deletion");
    repository.tick();

    let replay = repository
        .trade(&recipient.account_token, review_request)
        .expect("trade replay")
        .data;
    assert!(!replay.accepted);
    assert!(replay.trade.is_none());
    assert_eq!(
        replay.reason.as_deref(),
        Some("That trade is no longer available after an account departure.")
    );
    assert!(repository.ops_health().data.ready);
}
