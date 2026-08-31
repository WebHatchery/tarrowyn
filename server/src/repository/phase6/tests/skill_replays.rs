use super::super::super::ServerConfig;
use super::super::super::WorldRepository;
use tarrowyn_protocol::{
    AccountDeletionRequest, AuthLinkRequest, GuestSessionRequest, SkillAction, SkillRequest,
};

#[test]
fn account_deletion_removes_skill_replays_kept_by_another_identity() {
    let repository = WorldRepository::new(ServerConfig::default());
    let teacher_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("skill-replay-teacher".to_owned()),
            reset: false,
        })
        .expect("teacher guest session")
        .data;
    let teacher = repository
        .auth_link(
            &teacher_guest.account_token,
            AuthLinkRequest {
                request_id: "skill-replay-teacher-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "skill-replay-teacher-subject".to_owned(),
                display_name: Some("Departing teacher".to_owned()),
            },
        )
        .expect("teacher link")
        .data;
    let learner = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("skill-replay-learner".to_owned()),
            reset: false,
        })
        .expect("learner guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        for _ in 0..16 {
            super::super::super::skills::record_practice(
                &mut state,
                &teacher_guest.client_key,
                "sword-fighting",
            );
        }
        super::super::super::skills::record_practice(
            &mut state,
            &teacher_guest.client_key,
            "teaching",
        );
    }
    let lesson = repository
        .begin_skill_lesson(
            &teacher.session.account_token,
            SkillRequest {
                request_id: "skill-replay-begin".to_owned(),
                action: SkillAction::BeginLesson,
                lesson_id: None,
                skill_id: Some("sword-fighting".to_owned()),
                target_account_id: Some(learner.account_id.clone()),
            },
        )
        .expect("begin lesson")
        .data
        .lesson
        .expect("open lesson");
    let practice_request = SkillRequest {
        request_id: "skill-replay-practice".to_owned(),
        action: SkillAction::Practice,
        lesson_id: None,
        skill_id: Some("fishing".to_owned()),
        target_account_id: None,
    };
    let practiced = repository
        .practice_skill(&learner.account_token, practice_request.clone())
        .expect("learner practice")
        .data;
    assert!(practiced.skills.lessons.iter().any(|record| {
        record.lesson_id == lesson.lesson_id
            && record.teacher_account_id == teacher.account_id
            && record.teacher_name == "Departing teacher"
    }));

    repository
        .account_delete(
            &teacher.session.account_token,
            AccountDeletionRequest {
                request_id: "skill-replay-teacher-delete".to_owned(),
                account_id: teacher.account_id,
            },
        )
        .expect("schedule teacher deletion");
    repository.tick();

    let replay = repository
        .practice_skill(&learner.account_token, practice_request)
        .expect("skill replay")
        .data;
    assert!(replay.skills.lessons.is_empty());
    assert!(replay.lesson.is_none());
    assert!(repository.ops_health().data.ready);
}
