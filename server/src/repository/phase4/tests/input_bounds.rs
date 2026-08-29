use super::*;
use tarrowyn_protocol::{
    ClaimLifecycleAction, ClaimLifecycleRequest, GovernanceAction, KnowledgeAction,
    KnowledgeRequest, ProfessionAction, ProfessionKind, ProfessionRequest, SkillAction,
    SkillRequest,
};

#[test]
fn claim_selectors_reject_unbounded_or_controlled_ids() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-claim-input");

    for claim_id in ["x".repeat(161), "claim\nwith-control".to_owned()] {
        let error = repository
            .claim_lifecycle(
                &session.account_token,
                ClaimLifecycleRequest {
                    request_id: format!("claim-input-{}", claim_id.len()),
                    action: ClaimLifecycleAction::Inspect,
                    claim_id: Some(claim_id),
                    target_account_id: None,
                },
            )
            .expect_err("invalid claim selector should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, "invalid_claim_id");
    }
}

#[test]
fn knowledge_selectors_reject_unbounded_or_controlled_ids() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-knowledge-input");

    for knowledge_id in ["x".repeat(161), "knowledge\nwith-control".to_owned()] {
        let error = repository
            .knowledge(
                &session.account_token,
                KnowledgeRequest {
                    request_id: format!("knowledge-input-{}", knowledge_id.len()),
                    action: KnowledgeAction::Inspect,
                    knowledge_id: Some(knowledge_id),
                    target_account_id: None,
                },
            )
            .expect_err("invalid knowledge selector should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, "invalid_knowledge_id");
    }
}

#[test]
fn profession_selectors_reject_unbounded_or_controlled_ids() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-profession-input");

    let cases = [
        (Some("x".repeat(161)), None, "invalid_order_id"),
        (
            None,
            Some("capability\nwith-control".to_owned()),
            "invalid_capability_id",
        ),
    ];
    for (order_id, capability_id, expected_code) in cases {
        let error = repository
            .profession_order(
                &session.account_token,
                ProfessionRequest {
                    request_id: format!("profession-input-{expected_code}"),
                    action: ProfessionAction::Inspect,
                    order_id,
                    profession: Some(ProfessionKind::Carpenter),
                    capability_id,
                    service: None,
                    timing_score: None,
                },
            )
            .expect_err("invalid profession selector should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, expected_code);
    }
}

#[test]
fn skill_selectors_reject_unbounded_or_controlled_ids() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-skill-input");

    let cases = [
        (
            SkillAction::Practice,
            Some("x".repeat(161)),
            None,
            None,
            "invalid_skill_id",
        ),
        (
            SkillAction::BeginLesson,
            Some("fishing".to_owned()),
            None,
            Some("account\nwith-control".to_owned()),
            "invalid_target_account_id",
        ),
        (
            SkillAction::CompleteLesson,
            None,
            Some("lesson\nwith-control".to_owned()),
            None,
            "invalid_lesson_id",
        ),
    ];
    for (action, skill_id, lesson_id, target_account_id, expected_code) in cases {
        let error = repository
            .skill_action(
                &session.account_token,
                SkillRequest {
                    request_id: format!("skill-input-{expected_code}"),
                    action,
                    lesson_id,
                    skill_id,
                    target_account_id,
                },
            )
            .expect_err("invalid skill selector should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, expected_code);
    }
}

#[test]
fn governance_selectors_reject_unbounded_or_controlled_ids() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-governance-input");

    let cases = [
        (
            Some("office\nwith-control".to_owned()),
            None,
            "invalid_office_id",
        ),
        (None, Some("x".repeat(161)), "invalid_proposal_id"),
    ];
    for (office_id, proposal_id, expected_code) in cases {
        let mut request = governance_request(GovernanceAction::Inspect, expected_code);
        request.office_id = office_id;
        request.proposal_id = proposal_id;
        let error = repository
            .governance(&session.account_token, request)
            .expect_err("invalid governance selector should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, expected_code);
    }
}
