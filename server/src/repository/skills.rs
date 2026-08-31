//! Server-owned classless skill catalogue and authoritative practice ledger.

use super::*;
use tarrowyn_protocol::{ApiResponse, SkillLesson, SkillRequest, SkillResponse, SkillsResponse};

mod catalog;
mod integrity;
mod logic;
use catalog::catalog;
pub(crate) use catalog::validate_catalog;
#[cfg(test)]
use catalog::{validate_manifest, SkillManifest, SKILLS_JSON};
pub(crate) use integrity::skill_ledger_integrity_ok;
pub(super) use logic::{
    discover_eligible, finish_skill_action, mastery, qualifying_requirements_met, record_practice,
    record_qualifying_event, record_weapon_defeat, skills_view, storm_magic_discovered,
    storm_prerequisites_mastered, teacher_can_teach,
};

impl WorldRepository {
    pub fn skill_action(
        &self,
        token: &str,
        request: SkillRequest,
    ) -> Result<ApiResponse<SkillResponse>, RepositoryError> {
        match request.action {
            tarrowyn_protocol::SkillAction::Practice => self.practice_skill(token, request),
            tarrowyn_protocol::SkillAction::BeginLesson | tarrowyn_protocol::SkillAction::Teach => {
                self.begin_skill_lesson(token, request)
            }
            tarrowyn_protocol::SkillAction::CompleteLesson => {
                self.complete_skill_lesson(token, request)
            }
        }
    }

    pub fn skills(&self, token: &str) -> Result<ApiResponse<SkillsResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        let lessons_pruned = super::phase4::prune_school_lessons(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        let discovered = discover_eligible(&mut state, &key);
        if discovered || lessons_pruned {
            self.persist(&mut state)?;
        }
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: skills_view(&state, &key),
        })
    }

    pub fn practice_skill(
        &self,
        token: &str,
        request: SkillRequest,
    ) -> Result<ApiResponse<SkillResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        super::phase4::prune_school_lessons(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        super::phase4::validate_request_id(&request.request_id)?;
        let skill_id = super::phase4::validate_optional_identifier(
            request.skill_id.as_deref(),
            "invalid_skill_id",
            "A skill selector must be bounded and contain no control characters.",
        )?;
        let actor_account = super::phase4::account_id(&state, &key);
        let cache = format!("skill-practice:{actor_account}:{}", request.request_id);
        if let Some(super::phase4::Phase4Response::Skill(response)) =
            state.phase4.request_results.get(&cache)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let mut response = SkillResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            skill_id: skill_id.clone(),
            target_account_id: None,
            skills: skills_view(&state, &key),
            lesson: None,
            message: "Choose a depth-one discipline and take its first practical step.".to_owned(),
            reason: None,
        };
        let Some(skill_id) = skill_id.as_deref() else {
            response.reason = Some("Name the discipline for this first practice.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        };
        let Some(definition) = catalog()
            .skills
            .iter()
            .find(|skill| skill.id == skill_id)
            .cloned()
        else {
            response.reason = Some("That discipline is not in the current catalogue.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        };
        if definition.depth != 1 || definition.practice_key.as_deref() != Some(skill_id) {
            response.reason = Some(
                "Advanced disciplines emerge from play; choose a depth-one practice instead."
                    .to_owned(),
            );
            return finish_skill_action(self, &mut state, cache, response);
        }
        let previous_practice = state
            .identities
            .get(&key)
            .expect("identity exists")
            .skills
            .practice
            .get(skill_id)
            .copied()
            .unwrap_or(0);
        let actor_name = super::phase4::account_name(&state, &key);
        record_practice(&mut state, &key, skill_id);
        if previous_practice == 0 {
            super::phase4::record(
                &mut state,
                "skill practice",
                "A new discipline begins at the first step",
                &format!(
                    "{} began studying {} through its dependable entry path.",
                    actor_name, definition.name
                ),
            );
        }
        response.accepted = true;
        response.message = format!("You began {}. {}", definition.name, definition.entry_hint);
        response.skills = skills_view(&state, &key);
        finish_skill_action(self, &mut state, cache, response)
    }

    pub fn begin_skill_lesson(
        &self,
        token: &str,
        request: SkillRequest,
    ) -> Result<ApiResponse<SkillResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        super::phase4::prune_school_lessons(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        super::phase4::validate_request_id(&request.request_id)?;
        let skill_id = super::phase4::validate_optional_identifier(
            request.skill_id.as_deref(),
            "invalid_skill_id",
            "A skill selector must be bounded and contain no control characters.",
        )?;
        let target_account_id = super::phase4::validate_optional_identifier(
            request.target_account_id.as_deref(),
            "invalid_target_account_id",
            "A target account selector must be bounded and contain no control characters.",
        )?;
        let actor_account = super::phase4::account_id(&state, &key);
        let cache = format!("skill-lesson-begin:{actor_account}:{}", request.request_id);
        if let Some(super::phase4::Phase4Response::Skill(response)) =
            state.phase4.request_results.get(&cache)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let mut response = SkillResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            skill_id: skill_id.clone(),
            target_account_id: target_account_id.clone(),
            skills: skills_view(&state, &key),
            lesson: None,
            message: "A school lesson needs a mastered discipline and a willing neighbour."
                .to_owned(),
            reason: None,
        };
        let Some(skill_id) = skill_id.as_deref() else {
            response.reason = Some("Name the mastered skill to teach.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        };
        let Some(target_account) = target_account_id.as_deref() else {
            response.reason = Some("Teaching needs a receiving account.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        };
        let Some(target_key) = super::phase4::key_for_account(&state, target_account) else {
            response.reason =
                Some("The receiving player must have a recognised account.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        };
        if target_key == key {
            response.reason =
                Some("A school lesson needs another player to receive it.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        }
        let target_online = state
            .sessions
            .values()
            .any(|session| session.identity_key == target_key);
        if !target_online {
            response.reason =
                Some("The receiving player must be present for the lesson.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        }
        let teacher_position = state
            .identities
            .get(&key)
            .expect("identity exists")
            .position;
        let target_position = state
            .identities
            .get(&target_key)
            .expect("target identity exists")
            .position;
        if teacher_position != target_position {
            response.reason =
                Some("Stand beside the receiving player to demonstrate the skill.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        }
        let Some(definition) = catalog()
            .skills
            .iter()
            .find(|skill| skill.id == skill_id)
            .cloned()
        else {
            response.reason =
                Some("That skill is not part of the current school catalogue.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        };
        if !definition.directly_teachable {
            response.reason = Some("That discipline is not directly teachable.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        }
        let teacher_skills = &state.identities.get(&key).expect("identity exists").skills;
        if !teacher_can_teach(teacher_skills, &definition) {
            response.reason =
                Some("Master the discipline before offering it as a lesson.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        }
        if mastery(teacher_skills, "teaching") < definition.depth {
            response.reason = Some(format!(
                "Teaching mastery must reach depth {} before this lesson can be formal.",
                definition.depth
            ));
            return finish_skill_action(self, &mut state, cache, response);
        }
        let learner_practises_root = state
            .identities
            .get(&target_key)
            .expect("target identity exists")
            .skills
            .practice
            .contains_key(skill_id);
        let learner_knows_discovery = state
            .identities
            .get(&target_key)
            .expect("target identity exists")
            .skills
            .known
            .iter()
            .any(|known| known == skill_id);
        if definition.depth == 1 && learner_practises_root {
            response.reason =
                Some("The learner must practise this discipline before another lesson.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        }
        if definition.depth > 1 && learner_knows_discovery {
            response.reason = Some("The learner already carries that discovery.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        }
        if state.phase4.lessons.iter().any(|lesson| {
            lesson.teacher_account_id == actor_account
                && lesson.learner_account_id == target_account
                && lesson.skill_id == skill_id
        }) {
            response.reason =
                Some("That learner already has an open lesson in this discipline.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        }
        if !super::phase4::school_lesson_room(&mut state) {
            response.reason = Some(
                "The school ledger is full; complete or wait for an existing lesson before opening another."
                    .to_owned(),
            );
            return finish_skill_action(self, &mut state, cache, response);
        }
        let lesson_id = format!("school-lesson-{}", state.phase4.next_lesson_id);
        state.phase4.next_lesson_id = state.phase4.next_lesson_id.saturating_add(1);
        let lesson = SkillLesson {
            lesson_id,
            teacher_account_id: actor_account,
            teacher_name: super::phase4::account_name(&state, &key),
            learner_account_id: target_account.to_owned(),
            learner_name: super::phase4::account_name(&state, &target_key),
            skill_id: skill_id.to_owned(),
            skill_name: definition.name.clone(),
            started_tick: state.tick,
            expires_tick: state.tick.saturating_add(20),
        };
        state.phase4.lessons.push(lesson.clone());
        response.accepted = true;
        response.lesson = Some(lesson);
        response.message = format!(
            "{} opened a {} lesson; the learner must tap School to join the demonstration.",
            super::phase4::account_name(&state, &key),
            definition.name
        );
        super::phase4::record(
            &mut state,
            "school lesson opened",
            "A teacher opens a lesson beside a neighbour",
            &format!(
                "A nearby learner was invited to a formal {} lesson.",
                definition.name,
            ),
        );
        response.skills = skills_view(&state, &key);
        finish_skill_action(self, &mut state, cache, response)
    }

    pub fn complete_skill_lesson(
        &self,
        token: &str,
        request: SkillRequest,
    ) -> Result<ApiResponse<SkillResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        super::phase4::prune_school_lessons(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        super::phase4::validate_request_id(&request.request_id)?;
        let lesson_id = super::phase4::validate_optional_identifier(
            request.lesson_id.as_deref(),
            "invalid_lesson_id",
            "A lesson selector must be bounded and contain no control characters.",
        )?;
        let skill_id = super::phase4::validate_optional_identifier(
            request.skill_id.as_deref(),
            "invalid_skill_id",
            "A skill selector must be bounded and contain no control characters.",
        )?;
        let target_account_id = super::phase4::validate_optional_identifier(
            request.target_account_id.as_deref(),
            "invalid_target_account_id",
            "A target account selector must be bounded and contain no control characters.",
        )?;
        let actor_account = super::phase4::account_id(&state, &key);
        let cache = format!(
            "skill-lesson-complete:{actor_account}:{}",
            request.request_id
        );
        if let Some(super::phase4::Phase4Response::Skill(response)) =
            state.phase4.request_results.get(&cache)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let mut response = SkillResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            skill_id: skill_id.clone(),
            target_account_id: target_account_id.clone(),
            skills: skills_view(&state, &key),
            lesson: None,
            message: "Join an open lesson beside its teacher to take part.".to_owned(),
            reason: None,
        };
        let Some(lesson_id) = lesson_id.as_deref() else {
            response.reason = Some("Choose the open lesson to join.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        };
        let Some(lesson_index) = state.phase4.lessons.iter().position(|lesson| {
            lesson.lesson_id == lesson_id
                && lesson.learner_account_id == actor_account
                && target_account_id.as_deref() == Some(lesson.teacher_account_id.as_str())
        }) else {
            response.reason = Some("That lesson is not open for this learner.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        };
        let lesson = state.phase4.lessons[lesson_index].clone();
        let Some(teacher_key) = super::phase4::key_for_account(&state, &lesson.teacher_account_id)
        else {
            response.reason = Some("The teacher's account is no longer recognised.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        };
        let teacher_online = state
            .sessions
            .values()
            .any(|session| session.identity_key == teacher_key);
        if !teacher_online {
            response.reason =
                Some("The teacher must remain present for the demonstration.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        }
        let learner_position = state
            .identities
            .get(&key)
            .expect("identity exists")
            .position;
        let teacher_position = state
            .identities
            .get(&teacher_key)
            .expect("teacher identity exists")
            .position;
        if learner_position != teacher_position {
            response.reason =
                Some("Stand beside the teacher before joining the lesson.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        }
        let Some(definition) = catalog()
            .skills
            .iter()
            .find(|skill| skill.id == lesson.skill_id)
            .cloned()
        else {
            response.reason =
                Some("That lesson's discipline is no longer in the catalogue.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        };
        let teacher_skills = &state
            .identities
            .get(&teacher_key)
            .expect("teacher identity exists")
            .skills;
        if !teacher_can_teach(teacher_skills, &definition)
            || mastery(teacher_skills, "teaching") < definition.depth
        {
            response.reason =
                Some("The teacher no longer meets the lesson's mastery threshold.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        }
        let learner_skills = &state.identities.get(&key).expect("identity exists").skills;
        if (definition.depth == 1 && learner_skills.practice.contains_key(&lesson.skill_id))
            || (definition.depth > 1
                && learner_skills
                    .known
                    .iter()
                    .any(|known| known == &lesson.skill_id))
        {
            response.reason =
                Some("You have already begun or received this discipline.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        }
        if definition.depth == 1 {
            record_practice(&mut state, &key, &lesson.skill_id);
        } else {
            state
                .identities
                .get_mut(&key)
                .expect("identity exists")
                .skills
                .known
                .push(lesson.skill_id.clone());
        }
        state.phase4.lessons.remove(lesson_index);
        response.accepted = true;
        response.skill_id = Some(lesson.skill_id.clone());
        response.target_account_id = Some(lesson.teacher_account_id.clone());
        response.lesson = Some(lesson.clone());
        response.message = format!(
            "You joined {}'s {} lesson; the discipline now begins with your own practice.",
            lesson.teacher_name, lesson.skill_name
        );
        super::phase4::record(
            &mut state,
            "school lesson completed",
            "A learner joins a teacher's demonstration",
            &format!(
                "{} completed a shared {} lesson beside {}.",
                lesson.learner_name, lesson.skill_name, lesson.teacher_name
            ),
        );
        response.skills = skills_view(&state, &key);
        finish_skill_action(self, &mut state, cache, response)
    }
}

#[cfg(test)]
mod tests;
