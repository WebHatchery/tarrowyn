use super::super::super::{ServerConfig, WorldRepository};
use super::guest;
use tarrowyn_protocol::{
    MaterialStock, ProfessionAction, ProfessionKind, ProfessionRequest, ServiceOrder,
    ServiceOrderStatus,
};

fn service_order(index: usize, status: ServiceOrderStatus) -> ServiceOrder {
    ServiceOrder {
        order_id: format!("service-order-{index}"),
        requester_account_id: "other-account".to_owned(),
        requester_name: "Resident".to_owned(),
        provider_account_id: None,
        provider_name: None,
        service: "Repair the farmer's field tool".to_owned(),
        required_profession: ProfessionKind::Carpenter,
        materials: MaterialStock {
            wood: 1,
            iron: 1,
            tools: 1,
            ..MaterialStock::default()
        },
        tools_required: 1,
        reward_gold: 8,
        benefit: "The requesting field tool returns to working condition.".to_owned(),
        status,
        quality: if status == ServiceOrderStatus::Completed {
            100
        } else {
            0
        },
        created_tick: index as u64,
        completed_tick: (status == ServiceOrderStatus::Completed).then_some(index as u64),
    }
}

fn create_request(request_id: &str) -> ProfessionRequest {
    ProfessionRequest {
        request_id: request_id.to_owned(),
        action: ProfessionAction::CreateOrder,
        order_id: None,
        profession: Some(ProfessionKind::Carpenter),
        capability_id: None,
        service: None,
        timing_score: None,
    }
}

#[test]
fn service_order_history_evicts_settled_records_and_preserves_live_escrow() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-service-retention");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.next_order_id = 64;
        state.phase4.orders = (0..64)
            .map(|index| service_order(index, ServiceOrderStatus::Completed))
            .collect();
    }

    let created = repository
        .profession_order(
            &session.account_token,
            create_request("service-after-history"),
        )
        .expect("settled history should make room")
        .data;
    assert!(created.accepted);
    {
        let state = repository.state.lock().expect("repository lock");
        assert_eq!(state.phase4.orders.len(), 64);
        assert!(!state
            .phase4
            .orders
            .iter()
            .any(|order| order.order_id == "service-order-0"));
        assert!(state
            .phase4
            .orders
            .iter()
            .any(|order| order.order_id == "service-order-64"));
    }

    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.orders = (0..64)
            .map(|index| service_order(index, ServiceOrderStatus::Open))
            .collect();
    }
    let blocked = repository
        .profession_order(&session.account_token, create_request("service-while-full"))
        .expect("full service board should return a readable response")
        .data;
    assert!(!blocked.accepted);
    assert!(blocked
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("board is full")));
    assert_eq!(
        blocked.professions.materials,
        MaterialStock {
            wood: 2,
            iron: 1,
            cloth: 1,
            bandages: 1,
            tools: 0,
        }
    );
    assert_eq!(blocked.professions.orders.len(), 64);
}

#[test]
fn service_order_id_stays_at_the_numeric_ceiling() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "service-order-id-ceiling");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.next_order_id = u64::MAX;
    }

    let response = repository
        .profession_order(
            &session.account_token,
            create_request("service-order-id-ceiling-request"),
        )
        .expect("service order response")
        .data;

    assert!(response.accepted);
    assert_eq!(
        response.order.expect("created service order").order_id,
        format!("service-order-{}", u64::MAX)
    );
    let state = repository.state.lock().expect("repository lock");
    assert_eq!(state.phase4.next_order_id, u64::MAX);
}
