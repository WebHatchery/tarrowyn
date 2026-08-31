use super::*;
use crate::repository::models::SkillLedger;
use crate::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{GuestSessionRequest, SkillAction, SkillRequest, SkillStatus};

fn guest(repository: &WorldRepository, client_key: &str) -> String {
    repository
        .guest_session(GuestSessionRequest {
            client_key: Some(client_key.to_owned()),
            reset: false,
        })
        .expect("guest session")
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
fn skill_manifest_rejects_empty_player_guidance() {
    let mut manifest: SkillManifest = serde_json::from_str(SKILLS_JSON).unwrap();
    manifest.skills[0].entry_hint.clear();

    let error = validate_manifest(&manifest).unwrap_err();
    assert!(error.contains("entry hint"));
}

#[test]
fn skill_manifest_rejects_control_characters_in_player_guidance() {
    let mut manifest: SkillManifest = serde_json::from_str(SKILLS_JSON).unwrap();
    manifest.skills[0].description = "unsafe\ndescription".to_owned();

    let error = validate_manifest(&manifest).unwrap_err();
    assert!(error.contains("entry hint"));
}

#[test]
fn skill_manifest_rejects_duplicate_prerequisites() {
    let mut manifest: SkillManifest = serde_json::from_str(SKILLS_JSON).unwrap();
    let duplicate = manifest
        .skills
        .last()
        .unwrap()
        .prerequisites
        .first()
        .unwrap()
        .clone();
    manifest
        .skills
        .last_mut()
        .unwrap()
        .prerequisites
        .push(duplicate);

    let error = validate_manifest(&manifest).unwrap_err();
    assert!(error.contains("duplicate prerequisites"));
}

#[test]
fn skill_manifest_rejects_zero_advanced_discovery_thresholds() {
    let mut manifest: SkillManifest = serde_json::from_str(SKILLS_JSON).unwrap();
    let last = manifest.skills.len() - 1;
    manifest.skills[last].qualifying_count = Some(0);

    let error = validate_manifest(&manifest).unwrap_err();
    assert!(error.contains("discovery requirements"));
}

#[test]
fn skill_manifest_rejects_prerequisite_cycles() {
    let mut manifest: SkillManifest = serde_json::from_str(SKILLS_JSON).unwrap();
    let first = manifest.skills.len() - 2;
    let second = manifest.skills.len() - 1;
    let first_id = manifest.skills[first].id.clone();
    let second_id = manifest.skills[second].id.clone();
    manifest.skills[first].prerequisites.push(second_id);
    manifest.skills[second].prerequisites.push(first_id);

    let error = validate_manifest(&manifest).unwrap_err();
    assert!(error.contains("prerequisite cycle"));
}

#[test]
fn weapon_discovery_counts_saturate_when_each_family_reaches_the_ceiling() {
    let definition = catalog()
        .skills
        .iter()
        .find(|skill| skill.id == "weapon-fighting")
        .expect("weapon fighting definition should be present");
    let mut ledger = SkillLedger::default();
    for family in ["sword", "spear", "axe"] {
        ledger
            .qualifying_events
            .insert(format!("weapon_defeats:{family}"), u32::MAX);
    }

    assert!(qualifying_requirements_met(&ledger, definition));
}

#[test]
fn a_root_can_begin_through_an_idempotent_first_practice() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("skills-first-practice".to_owned()),
            reset: false,
        })
        .expect("guest session");
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
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("skills-discovery".to_owned()),
            reset: false,
        })
        .expect("guest session");
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
    assert!(weapon.usable);
    assert!(response.data.cursor > 0);
}

#[test]
fn skills_read_rechecks_stored_history_for_new_discoveries() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("skills-history-recheck".to_owned()),
            reset: false,
        })
        .expect("guest session");
    {
        let mut state = repository.state.lock().expect("state lock");
        let identity = state
            .identities
            .get_mut(&session.data.client_key)
            .expect("identity exists");
        for skill in ["sword-fighting", "spear-fighting", "axe-fighting"] {
            identity.skills.practice.insert(skill.to_owned(), 16);
        }
        for (family, count) in [("sword", 60), ("spear", 20), ("axe", 20)] {
            identity
                .skills
                .qualifying_events
                .insert(format!("weapon_defeats:{family}"), count);
        }
    }

    let response = repository.skills(&session.data.account_token).unwrap();
    let weapon = response
        .data
        .skills
        .iter()
        .find(|skill| skill.skill_id == "weapon-fighting")
        .expect("advanced skill remains in the catalogue");
    assert_eq!(weapon.status, SkillStatus::Discovered);
    assert!(weapon.usable);
    assert!(response.data.cursor > 0);
}

#[test]
fn a_nearby_master_can_teach_a_root_once() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let teacher = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("school-teacher".to_owned()),
            reset: false,
        })
        .expect("guest session");
    let learner = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("school-learner".to_owned()),
            reset: false,
        })
        .expect("guest session");
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

#[test]
fn a_discovered_advanced_skill_can_be_taught_without_granting_mastery() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let teacher = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("advanced-school-teacher".to_owned()),
            reset: false,
        })
        .expect("guest session");
    let learner = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("advanced-school-learner".to_owned()),
            reset: false,
        })
        .expect("guest session");
    {
        let mut state = repository.state.lock().expect("state lock");
        let teacher_skills = &mut state
            .identities
            .get_mut(&teacher.data.client_key)
            .expect("teacher identity exists")
            .skills;
        for skill in ["wind-magic", "water-magic", "electricity-magic"] {
            teacher_skills.practice.insert(skill.to_owned(), 16);
        }
        teacher_skills
            .qualifying_events
            .insert("storm_interactions".to_owned(), 25);
        teacher_skills.known.push("storm-magic".to_owned());
        for _ in 0..4 {
            record_practice(&mut state, &teacher.data.client_key, "teaching");
        }
    }

    let lesson = repository
        .begin_skill_lesson(
            &teacher.data.account_token,
            SkillRequest {
                request_id: "advanced-school-lesson".to_owned(),
                action: SkillAction::BeginLesson,
                lesson_id: None,
                skill_id: Some("storm-magic".to_owned()),
                target_account_id: Some(learner.data.account_id.clone()),
            },
        )
        .unwrap()
        .data;
    assert!(lesson.accepted);
    let lesson_id = lesson
        .lesson
        .as_ref()
        .expect("advanced lesson should be open")
        .lesson_id
        .clone();

    let learner_join = repository
        .complete_skill_lesson(
            &learner.data.account_token,
            SkillRequest {
                request_id: "advanced-school-lesson-join".to_owned(),
                action: SkillAction::CompleteLesson,
                lesson_id: Some(lesson_id),
                skill_id: Some("storm-magic".to_owned()),
                target_account_id: Some(teacher.data.account_id.clone()),
            },
        )
        .unwrap()
        .data;
    assert!(learner_join.accepted);
    let storm_magic = learner_join
        .skills
        .skills
        .iter()
        .find(|skill| skill.skill_id == "storm-magic")
        .expect("storm magic should remain in the catalogue");
    assert_eq!(storm_magic.status, SkillStatus::Discovered);
    assert_eq!(storm_magic.mastery, 0);
    assert!(!storm_magic.usable);

    {
        let mut state = repository.state.lock().expect("state lock");
        let learner_skills = &mut state
            .identities
            .get_mut(&learner.data.client_key)
            .expect("learner identity exists")
            .skills;
        for skill in ["wind-magic", "water-magic", "electricity-magic"] {
            learner_skills.practice.insert(skill.to_owned(), 16);
        }
        learner_skills
            .qualifying_events
            .insert("storm_interactions".to_owned(), 25);
    }
    let ready_storm = repository
        .skills(&learner.data.account_token)
        .unwrap()
        .data
        .skills
        .into_iter()
        .find(|skill| skill.skill_id == "storm-magic")
        .expect("storm magic should remain in the catalogue");
    assert!(ready_storm.usable);
}

#[test]
fn a_discovered_advanced_skill_without_its_requirements_cannot_be_taught() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let teacher = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("unready-advanced-school-teacher".to_owned()),
            reset: false,
        })
        .expect("guest session");
    let learner = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("unready-advanced-school-learner".to_owned()),
            reset: false,
        })
        .expect("guest session");
    {
        let mut state = repository.state.lock().expect("state lock");
        state
            .identities
            .get_mut(&teacher.data.client_key)
            .expect("teacher identity exists")
            .skills
            .known
            .push("storm-magic".to_owned());
        for _ in 0..4 {
            record_practice(&mut state, &teacher.data.client_key, "teaching");
        }
    }

    let response = repository
        .begin_skill_lesson(
            &teacher.data.account_token,
            SkillRequest {
                request_id: "unready-advanced-school-lesson".to_owned(),
                action: SkillAction::BeginLesson,
                lesson_id: None,
                skill_id: Some("storm-magic".to_owned()),
                target_account_id: Some(learner.data.account_id.clone()),
            },
        )
        .unwrap()
        .data;
    assert!(!response.accepted);
    assert_eq!(
        response.reason.as_deref(),
        Some("Master the discipline before offering it as a lesson.")
    );
}
