use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{ClaimLifecycleAction, ClaimLifecycleRequest};
use tarrowyn_protocol::{GovernanceAction, GovernanceRequest, GuestSessionRequest, PublicAction};
use tarrowyn_protocol::{MaterialStock, ProfessionKind, ServiceOrder, ServiceOrderStatus};

fn seeded_phase4_claim(repository: &WorldRepository) {
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-claim-state".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "phase4-claim-state-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request");
}

fn seeded_phase4_order(repository: &WorldRepository) {
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-order-state".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let mut state = repository.state.lock().expect("repository lock");
    let created_tick = state.tick;
    state.phase4.orders.push(ServiceOrder {
        order_id: "phase4-order-state-record".to_owned(),
        requester_account_id: session.account_id,
        requester_name: "Resident".to_owned(),
        provider_account_id: None,
        provider_name: None,
        service: "field-tool repair".to_owned(),
        required_profession: ProfessionKind::Carpenter,
        materials: MaterialStock {
            wood: 1,
            iron: 1,
            cloth: 0,
            bandages: 0,
            tools: 0,
        },
        tools_required: 0,
        reward_gold: 1,
        benefit: "A repaired field tool".to_owned(),
        status: ServiceOrderStatus::Open,
        quality: 0,
        created_tick,
        completed_tick: None,
    });
}

#[test]
fn invalid_phase4_sequence_metadata_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.next_claim_id = 0;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_phase4_governance_cursor_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.governance.cursor = state.cursor.saturating_add(1);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_phase4_proposal_timestamp_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-governance-time".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .governance(
            &session.account_token,
            GovernanceRequest {
                request_id: "phase4-proposal-time".to_owned(),
                action: GovernanceAction::Propose,
                office_id: None,
                proposal_id: None,
                public_action: Some(PublicAction::RepairRoad),
                target: None,
                cost: None,
                tax_rate_percent: None,
            },
        )
        .expect("proposal");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let future_tick = state.tick.saturating_add(1);
        state
            .phase4
            .governance
            .proposals
            .last_mut()
            .expect("proposal")
            .created_tick = future_tick;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_phase4_claim_activity_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    seeded_phase4_claim(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let future_tick = state.tick.saturating_add(1);
        state
            .phase4
            .claims
            .last_mut()
            .expect("claim")
            .last_active_tick = future_tick;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn requested_phase4_claim_cannot_have_building_access() {
    let repository = WorldRepository::new(ServerConfig::default());
    seeded_phase4_claim(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .claims
            .last_mut()
            .expect("claim")
            .building_access = true;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn out_of_bounds_phase4_infrastructure_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .infrastructure
            .first_mut()
            .expect("infrastructure")
            .position
            .x = state.phase5.locations.len() as i32 + 100;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn mismatched_phase4_infrastructure_status_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        let record = state
            .phase4
            .infrastructure
            .first_mut()
            .expect("infrastructure");
        record.condition = 0;
        record.status = tarrowyn_protocol::InfrastructureStatus::Operational;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_phase4_infrastructure_maintenance_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        let future_tick = state.tick.saturating_add(1);
        state
            .phase4
            .infrastructure
            .first_mut()
            .expect("infrastructure")
            .last_maintained_tick = future_tick;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn malformed_phase4_household_member_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .households
            .first_mut()
            .expect("household")
            .members
            .first_mut()
            .expect("household member")
            .role = "role\nwith-control".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_phase4_household_decision_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        let future_tick = state.tick.saturating_add(1);
        state
            .phase4
            .households
            .first_mut()
            .expect("household")
            .last_decision_tick = future_tick;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_phase4_service_order_timestamp_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    seeded_phase4_order(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let future_tick = state.tick.saturating_add(1);
        state
            .phase4
            .orders
            .last_mut()
            .expect("service order")
            .created_tick = future_tick;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn open_phase4_service_order_cannot_have_completion_tick() {
    let repository = WorldRepository::new(ServerConfig::default());
    seeded_phase4_order(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let tick = state.tick;
        state
            .phase4
            .orders
            .last_mut()
            .expect("service order")
            .completed_tick = Some(tick);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn malformed_phase4_knowledge_text_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .knowledge
            .first_mut()
            .expect("knowledge item")
            .effect = "effect\rwith-control".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn duplicate_phase4_knowledge_discoverer_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-knowledge-state".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.knowledge[0].discovered_by =
            vec![session.account_id.clone(), session.account_id];
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn duplicate_phase4_profession_profile_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-profile-state".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .professions(&session.account_token)
        .expect("profession view");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let profile = state
            .phase4
            .profiles
            .get(&session.client_key)
            .and_then(|profiles| profiles.first())
            .cloned()
            .expect("profile");
        state
            .phase4
            .profiles
            .get_mut(&session.client_key)
            .expect("profiles")
            .push(profile);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn malformed_phase4_capability_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-capability-state".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .professions(&session.account_token)
        .expect("profession view");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .profiles
            .get_mut(&session.client_key)
            .expect("profiles")
            .first_mut()
            .expect("profile")
            .capabilities
            .first_mut()
            .expect("capability")
            .level = 0;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
