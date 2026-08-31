use super::*;

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
    {
        let mut state = repository.state.lock().unwrap();
        state
            .phase3
            .expedition_credentials
            .push(guest.account_id.clone());
    }

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
    assert!(state
        .phase3
        .expedition_credentials
        .iter()
        .any(|id| id == &linked.account_id));
    assert!(!state
        .phase3
        .expedition_credentials
        .iter()
        .any(|id| id == &guest.account_id));
}

#[test]
fn account_link_preserves_support_replay_idempotency() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned(), "account-1".to_owned()],
        ..ServerConfig::default()
    });
    let guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("support-replay-link".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let repair_request = SupportRepairRequest {
        request_id: "support-replay-link-request".to_owned(),
        action: SupportRepairAction::NormalizeInventory,
        account_id: None,
        target_id: None,
        note: "The support repair remains idempotent through identity linking.".to_owned(),
    };
    let original = repository
        .support_repair(&guest.account_token, repair_request.clone())
        .unwrap()
        .data;
    assert!(original.accepted);

    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "support-replay-link-auth".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "support-replay-link-subject".to_owned(),
                display_name: Some("Linked support operator".to_owned()),
            },
        )
        .unwrap()
        .data;
    let replayed = repository
        .support_repair(&linked.session.account_token, repair_request)
        .unwrap()
        .data;

    assert_eq!(replayed.audit_id, original.audit_id);
}
