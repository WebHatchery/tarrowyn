use super::super::super::WorldRepository;
use tarrowyn_protocol::{ClaimLifecycleAction, ClaimLifecycleRequest};
use tarrowyn_protocol::{
    GuestSessionRequest, LocalCombatAction, LocalCombatRequest, MaterialStock, ProfessionKind,
    ServiceOrder, ServiceOrderStatus, SkillLesson, WeaponKind,
};

pub(super) fn seeded_phase4_claim(repository: &WorldRepository) {
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

pub(super) fn seeded_phase4_order(repository: &WorldRepository) {
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

pub(super) fn seeded_phase4_lesson(repository: &WorldRepository) {
    let teacher = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-lesson-teacher".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let learner = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-lesson-learner".to_owned()),
            reset: false,
        })
        .expect("learner session")
        .data;
    let mut state = repository.state.lock().expect("repository lock");
    let started_tick = state.tick;
    state.phase4.lessons.push(SkillLesson {
        lesson_id: "phase4-lesson-record".to_owned(),
        teacher_account_id: teacher.account_id,
        teacher_name: teacher.display_name,
        learner_account_id: learner.account_id,
        learner_name: learner.display_name,
        skill_id: "sword-fighting".to_owned(),
        skill_name: "Sword Fighting".to_owned(),
        started_tick,
        expires_tick: started_tick.saturating_add(20),
    });
}

pub(super) fn seeded_phase4_combat(repository: &WorldRepository) -> String {
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("phase4-combat-state".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let zone_position = repository
        .state
        .lock()
        .expect("repository lock")
        .phase3
        .zone
        .position;
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .identities
            .get_mut(&session.client_key)
            .expect("guest identity")
            .position = zone_position;
    }
    repository
        .local_combat(
            &session.account_token,
            LocalCombatRequest {
                request_id: "phase4-combat-state-seed".to_owned(),
                action: LocalCombatAction::Prepare,
                weapon: WeaponKind::IronSword,
            },
        )
        .expect("combat preparation");
    session.client_key
}
