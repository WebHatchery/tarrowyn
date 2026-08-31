use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{GuestSessionRequest, SkillLesson};

fn lesson(index: u64) -> SkillLesson {
    SkillLesson {
        lesson_id: format!("lesson-{index}"),
        teacher_account_id: "teacher".to_owned(),
        teacher_name: "Teacher".to_owned(),
        learner_account_id: format!("learner-{index}"),
        learner_name: "Learner".to_owned(),
        skill_id: "wind-magic".to_owned(),
        skill_name: "Wind Magic".to_owned(),
        started_tick: index,
        expires_tick: 200,
    }
}

#[test]
fn school_lessons_keep_a_bounded_newest_active_window() {
    let repository = WorldRepository::new(ServerConfig::default());
    let mut state = repository.state.lock().expect("repository lock");
    state.phase4.lessons = (0..(super::super::MAX_SCHOOL_LESSONS as u64 + 3))
        .map(lesson)
        .collect();

    super::super::trim_school_lessons(&mut state.phase4, 100);

    assert_eq!(state.phase4.lessons.len(), super::super::MAX_SCHOOL_LESSONS);
    assert_eq!(state.phase4.lessons.first().unwrap().lesson_id, "lesson-3");
    assert!(!super::super::school_lesson_room(&mut state));
}

#[test]
fn skills_read_persists_expired_lesson_pruning_before_a_restart() {
    let state_path = std::env::temp_dir().join(format!(
        "tarrowyn-skills-lesson-prune-{}.json",
        std::process::id()
    ));
    let config = ServerConfig {
        persistence_path: Some(state_path.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    };
    let repository = WorldRepository::new(config.clone());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("skills-lesson-prune".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.tick = 1;
        let mut expired = lesson(1);
        expired.expires_tick = 1;
        state.phase4.lessons.push(expired);
        repository.persist(&mut state).expect("fixture persistence");
    }

    let response = repository
        .skills(&session.account_token)
        .expect("skills read")
        .data;
    assert!(response.lessons.is_empty());
    drop(repository);

    let restored = WorldRepository::new(config);
    assert!(restored
        .state
        .lock()
        .expect("repository lock")
        .phase4
        .lessons
        .is_empty());
    let _ = std::fs::remove_file(state_path);
}
