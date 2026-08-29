use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::SkillLesson;

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
