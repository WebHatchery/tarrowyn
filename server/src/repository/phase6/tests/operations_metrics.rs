use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{AuthLinkRequest, GuestSessionRequest, MovementIntent};

#[test]
fn mysql_pool_metric_matches_the_selected_backend_safely() {
    let mut config = ServerConfig::default();
    assert_eq!(super::super::operations::mysql_pool_max_metric(&config), 0);

    config.db_driver = " MySQL ".to_owned();
    assert_eq!(super::super::operations::mysql_pool_max_metric(&config), 4);

    config.mysql_pool_max_connections = usize::MAX;
    assert_eq!(
        super::super::operations::mysql_pool_max_metric(&config),
        u32::MAX
    );
}

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
    assert_eq!(metrics.mysql_pool_max_connections, 0);
    assert_eq!(metrics.abandoned_claims, 0);
    assert!(metrics.declining_settlements > 0);
    assert!(metrics.newcomer_access);
}

#[test]
fn operational_metrics_exclude_sessions_expired_since_the_last_world_tick() {
    let repository = WorldRepository::new(ServerConfig {
        session_ttl_seconds: 1,
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("metrics-expiry-operator".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let player = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("metrics-expiry-player".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;

    {
        let mut state = repository.state.lock().expect("repository lock");
        state.tick = repository.config.session_ttl_ticks();
        state
            .sessions
            .get_mut(&operator.account_token)
            .expect("operator session")
            .last_seen_tick = state.tick;
        assert!(state.sessions.contains_key(&player.account_token));
    }

    let metrics = repository
        .ops_metrics(&operator.account_token)
        .expect("operator metrics")
        .data;

    assert_eq!(metrics.connected_sessions, 1);
    let state = repository.state.lock().expect("repository lock");
    assert!(!state.sessions.contains_key(&player.account_token));
}

#[test]
fn operational_health_cleans_expired_sessions_before_checking_readiness() {
    let config = ServerConfig {
        production_session_ttl_seconds: 1,
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    };
    let repository = WorldRepository::new(config);
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("health-expired-session".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "health-expired-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "health-expired-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("linked session")
        .data;

    {
        let mut state = repository.state.lock().expect("repository lock");
        state.tick = linked.session.expires_at_tick;
        assert!(state.sessions.contains_key(&linked.session.account_token));
    }

    let health = repository.ops_health().data;
    assert!(health.ready);
    assert!(health.integrity_ok);
    let state = repository.state.lock().expect("repository lock");
    assert!(!state.sessions.contains_key(&linked.session.account_token));
}
