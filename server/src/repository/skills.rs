//! Server-owned classless skill catalogue and authoritative practice ledger.

use super::models::{RepositoryState, SkillLedger};
use super::*;
use std::collections::HashSet;
use tarrowyn_protocol::{
    ApiResponse, SkillLesson, SkillRequest, SkillResponse, SkillStatus, SkillView, SkillsResponse,
    WeaponKind,
};

mod catalog;
pub(crate) use catalog::validate_catalog;
use catalog::{catalog, SkillDefinition};
#[cfg(test)]
use catalog::{validate_manifest, SkillManifest, SKILLS_JSON};

const MAX_SKILL_LEDGER_ENTRIES: usize = 128;

pub(crate) fn skill_ledger_integrity_ok(ledger: &SkillLedger) -> bool {
    let manifest = catalog();
    let skill_ids: HashSet<&str> = manifest
        .skills
        .iter()
        .map(|definition| definition.id.as_str())
        .collect();
    let qualifying_events: HashSet<&str> = manifest
        .skills
        .iter()
        .filter_map(|definition| definition.qualifying_event.as_deref())
        .collect();
    let practice_ok = ledger.practice.len() <= MAX_SKILL_LEDGER_ENTRIES
        && ledger.practice.iter().all(|(skill_id, practice)| {
            *practice > 0
                && manifest
                    .skills
                    .iter()
                    .any(|definition| definition.id == *skill_id && definition.depth == 1)
        });
    let known_ok = ledger.known.len() <= MAX_SKILL_LEDGER_ENTRIES
        && unique_skill_ids(&ledger.known)
        && ledger.known.iter().all(|skill_id| {
            manifest
                .skills
                .iter()
                .find(|definition| definition.id == *skill_id && definition.depth > 1)
                .is_some_and(|definition| {
                    definition
                        .prerequisites
                        .iter()
                        .all(|prerequisite| mastery(ledger, prerequisite) >= 5)
                        && qualifying_requirements_met(ledger, definition)
                })
        });
    let qualifying_ok = ledger.qualifying_events.len() <= MAX_SKILL_LEDGER_ENTRIES
        && ledger.qualifying_events.iter().all(|(event, count)| {
            *count > 0
                && bounded_skill_text(event)
                && (qualifying_events.contains(event.as_str())
                    || event.split_once(':').is_some_and(|(kind, value)| {
                        qualifying_events.contains(kind)
                            && bounded_skill_text(value)
                            && !value.contains(':')
                    }))
        });
    practice_ok
        && known_ok
        && qualifying_ok
        && ledger
            .practice
            .keys()
            .all(|skill_id| skill_ids.contains(skill_id.as_str()))
}

fn unique_skill_ids(skill_ids: &[String]) -> bool {
    let mut seen = HashSet::new();
    skill_ids
        .iter()
        .all(|skill_id| seen.insert(skill_id.as_str()))
}

fn bounded_skill_text(value: &str) -> bool {
    !value.trim().is_empty() && value.chars().count() <= 160 && !value.chars().any(char::is_control)
}

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
        expire_sessions(&mut state, &self.config);
        super::phase4::prune_school_lessons(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        let discovered = discover_eligible(&mut state, &key);
        if discovered {
            self.persist(&state);
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
        expire_sessions(&mut state, &self.config);
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
        expire_sessions(&mut state, &self.config);
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
        if mastery(teacher_skills, skill_id) < 5 {
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
        expire_sessions(&mut state, &self.config);
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
        if mastery(teacher_skills, &lesson.skill_id) < 5
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

fn finish_skill_action(
    repository: &WorldRepository,
    state: &mut RepositoryState,
    cache: String,
    response: SkillResponse,
) -> Result<ApiResponse<SkillResponse>, RepositoryError> {
    state.phase4.request_results.insert(
        cache,
        super::phase4::Phase4Response::Skill(response.clone()),
    );
    record_command_outcome(state, response.accepted);
    repository.persist(state);
    Ok(ApiResponse {
        meta: meta(
            state.tick,
            Some(response.request_id.clone()),
            Some(state.cursor),
        ),
        data: response,
    })
}

fn skills_view(state: &RepositoryState, key: &str) -> SkillsResponse {
    let identity = state.identities.get(key).expect("identity exists");
    let account_id = identity.account_id.clone();
    SkillsResponse {
        skills: catalog()
            .skills
            .iter()
            .map(|definition| skill_view(&identity.skills, definition))
            .collect(),
        lessons: state
            .phase4
            .lessons
            .iter()
            .filter(|lesson| {
                lesson.teacher_account_id == account_id || lesson.learner_account_id == account_id
            })
            .cloned()
            .collect(),
        cursor: state.cursor,
    }
}

pub(super) fn record_practice(state: &mut RepositoryState, key: &str, skill_id: &str) {
    let Some(definition) = catalog().skills.iter().find(|skill| skill.id == skill_id) else {
        return;
    };
    if definition.depth != 1 {
        return;
    }
    let practice = {
        let identity = state.identities.get_mut(key).expect("identity exists");
        let entry = identity
            .skills
            .practice
            .entry(skill_id.to_owned())
            .or_insert(0);
        *entry = entry.saturating_add(1);
        *entry
    };
    if practice == 1 {
        let message = format!(
            "{} has begun through a direct, dependable practice.",
            definition.name
        );
        add_notice(state, "skills", &message);
    }
    discover_eligible(state, key);
}

pub(super) fn record_qualifying_event(state: &mut RepositoryState, key: &str, event: &str) -> bool {
    let identity = state.identities.get_mut(key).expect("identity exists");
    let count = identity
        .skills
        .qualifying_events
        .entry(event.to_owned())
        .or_insert(0);
    *count = count.saturating_add(1);
    discover_eligible(state, key)
}

pub(super) fn storm_magic_discovered(state: &RepositoryState, key: &str) -> bool {
    state
        .identities
        .get(key)
        .expect("identity exists")
        .skills
        .known
        .iter()
        .any(|skill_id| skill_id == "storm-magic")
}

pub(super) fn storm_prerequisites_mastered(state: &RepositoryState, key: &str) -> bool {
    let ledger = &state.identities.get(key).expect("identity exists").skills;
    ["wind-magic", "water-magic", "electricity-magic"]
        .iter()
        .all(|skill_id| mastery(ledger, skill_id) >= 5)
}

pub(super) fn record_weapon_defeat(state: &mut RepositoryState, key: &str, weapon: WeaponKind) {
    if let Some(family) = weapon.weapon_fighting_family() {
        record_qualifying_event(state, key, &format!("weapon_defeats:{family}"));
    }
}

fn discover_eligible(state: &mut RepositoryState, key: &str) -> bool {
    let identity = state.identities.get(key).expect("identity exists");
    let newly_discovered: Vec<String> = catalog()
        .skills
        .iter()
        .filter(|definition| {
            definition.depth > 1
                && !identity
                    .skills
                    .known
                    .iter()
                    .any(|known| known == &definition.id)
                && definition
                    .prerequisites
                    .iter()
                    .all(|prerequisite| mastery(&identity.skills, prerequisite) >= 5)
                && qualifying_requirements_met(&identity.skills, definition)
        })
        .map(|definition| definition.id.clone())
        .collect();
    if newly_discovered.is_empty() {
        return false;
    }
    for skill_id in newly_discovered {
        state
            .identities
            .get_mut(key)
            .expect("identity exists")
            .skills
            .known
            .push(skill_id.clone());
        add_notice(
            state,
            "discovery",
            &format!("A hidden skill has been discovered: {skill_id}."),
        );
    }
    true
}

fn qualifying_requirements_met(ledger: &SkillLedger, definition: &SkillDefinition) -> bool {
    let needed = definition.qualifying_count.unwrap_or(u32::MAX);
    match definition.qualifying_event.as_deref() {
        Some("weapon_defeats") => {
            let family_counts = definition.prerequisites.iter().map(|skill_id| {
                let family = skill_id.trim_end_matches("-fighting");
                ledger
                    .qualifying_events
                    .get(&format!("weapon_defeats:{family}"))
                    .copied()
                    .unwrap_or(0)
            });
            let counts: Vec<u32> = family_counts.collect();
            let minimum = definition.minimum_per_prerequisite.unwrap_or(0);
            counts.iter().copied().fold(0, u32::saturating_add) >= needed
                && counts.iter().all(|count| *count >= minimum)
        }
        Some(event) => ledger.qualifying_events.get(event).copied().unwrap_or(0) >= needed,
        None => false,
    }
}

fn skill_view(ledger: &SkillLedger, definition: &SkillDefinition) -> SkillView {
    let skill_mastery = mastery(ledger, &definition.id);
    let status = if definition.depth == 1 {
        if skill_mastery == 0 {
            SkillStatus::Available
        } else if skill_mastery >= 5 {
            SkillStatus::Mastered
        } else {
            SkillStatus::Practising
        }
    } else if ledger.known.iter().any(|known| known == &definition.id) {
        SkillStatus::Discovered
    } else if definition
        .prerequisites
        .iter()
        .all(|prerequisite| mastery(ledger, prerequisite) >= 5)
    {
        SkillStatus::Resonating
    } else {
        SkillStatus::Available
    };
    let entry_hint = if definition.depth > 1 && status == SkillStatus::Available {
        "No clear resonance has appeared yet; keep learning the practices that interest you."
            .to_owned()
    } else {
        definition.entry_hint.clone()
    };
    SkillView {
        skill_id: definition.id.clone(),
        name: definition.name.clone(),
        family: definition.family,
        depth: definition.depth,
        mastery: skill_mastery,
        status,
        description: definition.description.clone(),
        entry_hint,
    }
}

fn mastery(ledger: &SkillLedger, skill_id: &str) -> u8 {
    let practice = ledger.practice.get(skill_id).copied().unwrap_or(0);
    if practice == 0 {
        0
    } else {
        (practice / 4 + 1).min(5) as u8
    }
}

#[cfg(test)]
mod tests;
