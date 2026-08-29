use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{GuestSessionRequest, MovementIntent};

#[test]
fn operational_metrics_require_a_configured_support_operator() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("metrics-operator".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let player = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("metrics-player".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;

    let error = repository
        .ops_metrics(&player.account_token)
        .expect_err("ordinary players must not read operational metrics");
    assert_eq!(error.status, 403);
    assert_eq!(error.error.code, "support_operator_required");
    assert!(repository.ops_metrics(&operator.account_token).is_ok());

    repository
        .movement(
            &player.account_token,
            MovementIntent {
                request_id: "metrics-accepted".to_owned(),
                dx: 0,
                dy: 1,
            },
        )
        .unwrap();
    repository
        .movement(
            &player.account_token,
            MovementIntent {
                request_id: "metrics-rejected".to_owned(),
                dx: 2,
                dy: 0,
            },
        )
        .unwrap();
    let metrics = repository
        .ops_metrics(&operator.account_token)
        .unwrap()
        .data;
    assert!(metrics.completed_commands >= 1);
    assert!(metrics.rejected_commands >= 1);
    assert!(metrics.average_price_index_percent > 0);
    assert!(metrics.scarce_goods_count > 0);
    assert!(metrics.npc_fallback_households > 0);
    assert_eq!(metrics.open_market_fallback_orders, 0);
    assert_eq!(metrics.abandoned_claims, 0);
    assert!(metrics.declining_settlements > 0);
    assert!(metrics.newcomer_access);
}
