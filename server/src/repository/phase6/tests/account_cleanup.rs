use super::*;
use tarrowyn_protocol::{
    AccountDeletionRequest, AuthLinkRequest, GuestSessionRequest, MarketOrderAction,
    MarketOrderRequest, ProfessionAction, ProfessionKind, ProfessionRequest, SkillAction,
    SkillRequest,
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
        .practice_skill(
            &linked.session.account_token,
            SkillRequest {
                request_id: "replay-cleanup-skill".to_owned(),
                action: SkillAction::Practice,
                lesson_id: None,
                skill_id: Some("fishing".to_owned()),
                target_account_id: None,
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
        .phase4
        .request_results
        .keys()
        .any(|key| key.starts_with(&format!("skill-practice:{account_id}:"))));
    assert!(!state
        .phase5
        .request_results
        .keys()
        .any(|key| key.starts_with(&format!("{identity_key}:"))));
}

#[test]
fn account_link_preserves_phase4_and_skill_replay_idempotency() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-replay-link".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let order_request = ProfessionRequest {
        request_id: "phase4-replay-order".to_owned(),
        action: ProfessionAction::CreateOrder,
        order_id: None,
        profession: Some(ProfessionKind::Carpenter),
        capability_id: None,
        service: None,
        timing_score: None,
    };
    let original_order = repository
        .profession_order(&guest.account_token, order_request.clone())
        .unwrap()
        .data
        .order
        .expect("the guest should create a service order");
    let skill_request = SkillRequest {
        request_id: "phase4-replay-skill".to_owned(),
        action: SkillAction::Practice,
        lesson_id: None,
        skill_id: Some("fishing".to_owned()),
        target_account_id: None,
    };
    repository
        .practice_skill(&guest.account_token, skill_request.clone())
        .unwrap();

    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "phase4-replay-link-request".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "phase4-replay-subject".to_owned(),
                display_name: Some("Linked replay resident".to_owned()),
            },
        )
        .unwrap()
        .data;
    let replayed_order = repository
        .profession_order(&linked.session.account_token, order_request)
        .unwrap()
        .data
        .order
        .expect("the pre-link order request should replay after linking");
    assert_eq!(replayed_order.order_id, original_order.order_id);
    assert_eq!(replayed_order.requester_account_id, linked.account_id);
    assert_eq!(replayed_order.requester_name, "Linked replay resident");
    repository
        .practice_skill(&linked.session.account_token, skill_request)
        .unwrap();

    let state = repository.state.lock().unwrap();
    let identity_key = state
        .phase6
        .accounts
        .get(&linked.account_id)
        .expect("the linked account should be present")
        .identity_key
        .clone();
    assert_eq!(
        state
            .identities
            .get(&identity_key)
            .and_then(|identity| identity.skills.practice.get("fishing"))
            .copied(),
        Some(1)
    );
}
