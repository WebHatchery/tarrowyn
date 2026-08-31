use super::*;

#[test]
fn malformed_phase4_lesson_text_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    seeded_phase4_lesson(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.lessons.last_mut().expect("lesson").skill_name = "x".repeat(161);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn future_phase4_lesson_start_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    seeded_phase4_lesson(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let future_tick = state.tick.saturating_add(1);
        state
            .phase4
            .lessons
            .last_mut()
            .expect("lesson")
            .started_tick = future_tick;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn expired_before_start_phase4_lesson_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    seeded_phase4_lesson(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let lesson = state.phase4.lessons.last_mut().expect("lesson");
        lesson.expires_tick = lesson.started_tick;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn over_capacity_phase4_lessons_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    seeded_phase4_lesson(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        let template = state.phase4.lessons.first().cloned().expect("lesson");
        for index in 0..128 {
            let mut lesson = template.clone();
            lesson.lesson_id = format!("phase4-lesson-{index}");
            state.phase4.lessons.push(lesson);
        }
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
