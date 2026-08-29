use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    GuestSessionRequest, ProfessionAction, ProfessionKind, ProfessionRequest, ServiceOrderStatus,
};

fn seeded_order(repository: &WorldRepository) -> String {
    let player = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-order-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .profession_order(
            &player.account_token,
            ProfessionRequest {
                request_id: "phase4-order-integrity-create".to_owned(),
                action: ProfessionAction::CreateOrder,
                order_id: None,
                profession: Some(ProfessionKind::Carpenter),
                capability_id: None,
                service: None,
                timing_score: None,
            },
        )
        .expect("service order");
    player.account_id
}

#[test]
fn malformed_phase4_service_order_text_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    seeded_order(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .orders
            .last_mut()
            .expect("service order")
            .benefit = "benefit\nwith-control".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn missing_phase4_provider_name_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let provider_account = seeded_order(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let order = state.phase4.orders.last_mut().expect("service order");
        order.provider_account_id = Some(provider_account);
        order.provider_name = None;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn incomplete_phase4_completed_order_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let provider_account = seeded_order(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let order = state.phase4.orders.last_mut().expect("service order");
        order.status = ServiceOrderStatus::Completed;
        order.provider_account_id = Some(provider_account);
        order.provider_name = Some("Provider".to_owned());
        order.completed_tick = None;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
