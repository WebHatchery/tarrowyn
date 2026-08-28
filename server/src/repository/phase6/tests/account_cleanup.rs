use super::*;
use tarrowyn_protocol::{
    AccountDeletionRequest, AuthLinkRequest, GuestSessionRequest, MarketOrderAction,
    MarketOrderRequest, ProfessionAction, ProfessionRequest,
};

#[test]
fn account_deletion_removes_phase4_and_phase5_replay_payloads() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("replay-cleanup-client".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "replay-cleanup-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "replay-cleanup-subject".to_owned(),
                display_name: Some("Replay cleanup resident".to_owned()),
            },
        )
        .unwrap()
        .data;
    let account_id = linked.account_id.clone();
    let identity_key = {
        let state = repository.state.lock().unwrap();
        state
            .phase6
            .accounts
            .get(&account_id)
            .expect("the linked account should be present")
            .identity_key
            .clone()
    };

    repository
        .profession_order(
            &linked.session.account_token,
            ProfessionRequest {
                request_id: "replay-cleanup-phase4".to_owned(),
                action: ProfessionAction::Inspect,
                order_id: None,
                profession: None,
                capability_id: None,
                service: None,
                timing_score: None,
            },
        )
        .unwrap();
    repository
        .market_order(
            &linked.session.account_token,
            MarketOrderRequest {
                request_id: "replay-cleanup-phase5".to_owned(),
                action: MarketOrderAction::Cancel,
                order_id: Some("missing-replay-cleanup-order".to_owned()),
                destination_location_id: None,
                commodity: None,
                quantity: None,
            },
        )
        .unwrap();

    let deletion = repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "replay-cleanup-delete".to_owned(),
                account_id: account_id.clone(),
            },
        )
        .unwrap()
        .data;
    assert!(deletion.accepted);
    repository.tick();

    let state = repository.state.lock().unwrap();
    assert!(!state
        .phase4
        .request_results
        .keys()
        .any(|key| key.starts_with(&format!("phase4:{account_id}:"))));
    assert!(!state
        .phase5
        .request_results
        .keys()
        .any(|key| key.starts_with(&format!("{identity_key}:"))));
}
