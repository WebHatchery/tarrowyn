use super::*;
use crate::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{GuestSessionRequest, SkillAction, SkillRequest, SkillStatus};

fn guest(repository: &WorldRepository, client_key: &str) -> String {
    repository
        .guest_session(GuestSessionRequest {
            client_key: Some(client_key.to_owned()),
            reset: false,
        })
        .data
        .account_token
}

#[test]
fn catalogue_exposes_direct_roots_and_hides_advanced_recipe_details() {
    validate_catalog().expect("skill content should validate");
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let token = guest(&repository, "skills-catalogue");
    let response = repository
        .skills(&token)
        .expect("skills should be readable");
    assert!(response.data.skills.len() >= 30);
    let crop = response
        .data
        .skills
        .iter()
        .find(|skill| skill.skill_id == "crop-tending")
        .expect("crop tending root should be present");
    assert_eq!(crop.status, SkillStatus::Available);
    assert_eq!(crop.mastery, 0);
    let weapon = response
        .data
        .skills
        .iter()
        .find(|skill| skill.skill_id == "weapon-fighting")
        .expect("weapon fighting discovery should be present");
    assert_eq!(weapon.status, SkillStatus::Available);
    assert!(!weapon.entry_hint.contains("100"));
    assert!(!weapon.entry_hint.contains("sword-fighting"));
}

#[test]
fn a_root_can_begin_through_an_idempotent_first_practice() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let session = repository.guest_session(GuestSessionRequest {
        client_key: Some("skills-first-practice".to_owned()),
        reset: false,
    });
    let request = SkillRequest {
        request_id: "practice-fishing".to_owned(),
        action: SkillAction::Practice,
        lesson_id: None,
        skill_id: Some("fishing".to_owned()),
        target_account_id: None,
    };
    let first = repository
        .practice_skill(&session.data.account_token, request.clone())
        .unwrap()
        .data;
    assert!(first.accepted);
    assert!(first.message.contains("Fishing"));
    let retry = repository
        .practice_skill(&session.data.account_token, request)
        .unwrap()
        .data;
    assert_eq!(retry, first);
    let fishing = repository
        .skills(&session.data.account_token)
        .unwrap()
        .data
        .skills
        .into_iter()
        .find(|skill| skill.skill_id == "fishing")
        .unwrap();
    assert_eq!(fishing.mastery, 1);
    assert_eq!(fishing.status, SkillStatus::Practising);
}

#[test]
fn practice_is_persistent_and_complete_discoveries_are_authoritative() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let session = repository.guest_session(GuestSessionRequest {
        client_key: Some("skills-discovery".to_owned()),
        reset: false,
    });
    {
        let mut state = repository.state.lock().expect("state lock");
        for skill in ["sword-fighting", "spear-fighting", "axe-fighting"] {
            for _ in 0..16 {
                record_practice(&mut state, &session.data.client_key, skill);
            }
        }
        for family in ["sword", "spear", "axe"] {
            for _ in 0..20 {
                record_qualifying_event(
                    &mut state,
                    &session.data.client_key,
                    &format!("weapon_defeats:{family}"),
                );
            }
        }
        for _ in 0..40 {
            record_qualifying_event(&mut state, &session.data.client_key, "weapon_defeats:sword");
        }
    }
    let response = repository.skills(&session.data.account_token).unwrap();
    let sword = response
        .data
        .skills
        .iter()
        .find(|skill| skill.skill_id == "sword-fighting")
        .unwrap();
    assert_eq!(sword.status, SkillStatus::Mastered);
    let weapon = response
        .data
        .skills
        .iter()
        .find(|skill| skill.skill_id == "weapon-fighting")
        .unwrap();
    assert_eq!(weapon.status, SkillStatus::Discovered);
    assert!(response.data.cursor > 0);
}

#[test]
fn a_nearby_master_can_teach_a_root_once() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let teacher = repository.guest_session(GuestSessionRequest {
        client_key: Some("school-teacher".to_owned()),
        reset: false,
    });
    let learner = repository.guest_session(GuestSessionRequest {
        client_key: Some("school-learner".to_owned()),
        reset: false,
    });
    {
        let mut state = repository.state.lock().expect("state lock");
        for _ in 0..16 {
            record_practice(&mut state, &teacher.data.client_key, "sword-fighting");
        }
        record_practice(&mut state, &teacher.data.client_key, "teaching");
    }
    let lesson = repository
        .begin_skill_lesson(
            &teacher.data.account_token,
            SkillRequest {
                request_id: "school-lesson".to_owned(),
                action: SkillAction::BeginLesson,
                lesson_id: None,
                skill_id: Some("sword-fighting".to_owned()),
                target_account_id: Some(learner.data.account_id.clone()),
            },
        )
        .unwrap()
        .data;
    assert!(lesson.accepted);
    let lesson_id = lesson
        .lesson
        .as_ref()
        .expect("lesson should be open")
        .lesson_id
        .clone();
    assert_eq!(
        repository
            .skills(&learner.data.account_token)
            .unwrap()
            .data
            .lessons
            .len(),
        1
    );
    let learner_join = repository
        .complete_skill_lesson(
            &learner.data.account_token,
            SkillRequest {
                request_id: "school-lesson-join".to_owned(),
                action: SkillAction::CompleteLesson,
                lesson_id: Some(lesson_id),
                skill_id: Some("sword-fighting".to_owned()),
                target_account_id: Some(teacher.data.account_id.clone()),
            },
        )
        .unwrap()
        .data;
    assert!(learner_join.accepted);
    let learner_sword = repository
        .skills(&learner.data.account_token)
        .unwrap()
        .data
        .skills
        .into_iter()
        .find(|skill| skill.skill_id == "sword-fighting")
        .unwrap();
    assert_eq!(learner_sword.mastery, 1);
    assert!(
        !repository
            .begin_skill_lesson(
                &teacher.data.account_token,
                SkillRequest {
                    request_id: "school-lesson-again".to_owned(),
                    action: SkillAction::BeginLesson,
                    lesson_id: None,
                    skill_id: Some("sword-fighting".to_owned()),
                    target_account_id: Some(learner.data.account_id.clone()),
                },
            )
            .unwrap()
            .data
            .accepted
    );
}
