//! Server-owned classless skill catalogue and authoritative practice ledger.

use super::models::{RepositoryState, SkillLedger};
use super::*;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::OnceLock;
use tarrowyn_protocol::{
    ApiResponse, SkillFamily, SkillRequest, SkillResponse, SkillStatus, SkillView, SkillsResponse,
    WeaponKind,
};

const SKILLS_JSON: &str = include_str!("../../../assets/data/skills.json");

#[derive(Debug, Deserialize)]
struct SkillManifest {
    version: u32,
    skills: Vec<SkillDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
struct SkillDefinition {
    id: String,
    name: String,
    family: SkillFamily,
    depth: u8,
    description: String,
    entry_hint: String,
    #[serde(default)]
    practice_key: Option<String>,
    #[serde(default)]
    prerequisites: Vec<String>,
    #[serde(default)]
    qualifying_event: Option<String>,
    #[serde(default)]
    qualifying_count: Option<u32>,
    #[serde(default)]
    minimum_per_prerequisite: Option<u32>,
    #[serde(default)]
    directly_teachable: bool,
}

static CATALOG: OnceLock<SkillManifest> = OnceLock::new();

fn catalog() -> &'static SkillManifest {
    CATALOG.get_or_init(|| {
        let manifest: SkillManifest =
            serde_json::from_str(SKILLS_JSON).expect("skills content JSON must be valid");
        validate_manifest(&manifest).expect("skills content must satisfy its schema");
        manifest
    })
}

pub(crate) fn validate_catalog() -> Result<(), String> {
    let manifest: SkillManifest = serde_json::from_str(SKILLS_JSON)
        .map_err(|error| format!("skills JSON is invalid: {error}"))?;
    validate_manifest(&manifest)
}

fn validate_manifest(manifest: &SkillManifest) -> Result<(), String> {
    if manifest.version == 0 || manifest.skills.is_empty() {
        return Err("skills content needs a positive version and at least one skill".to_owned());
    }
    let ids: HashSet<&str> = manifest
        .skills
        .iter()
        .map(|skill| skill.id.as_str())
        .collect();
    if ids.len() != manifest.skills.len() || ids.iter().any(|id| id.trim().is_empty()) {
        return Err("skill IDs must be unique and non-empty".to_owned());
    }
    for skill in &manifest.skills {
        if !(1..=5).contains(&skill.depth) || skill.name.trim().is_empty() {
            return Err(format!("skill {} has invalid identity or depth", skill.id));
        }
        if skill.depth == 1 {
            if !skill.directly_teachable
                || skill.practice_key.as_deref() != Some(skill.id.as_str())
                || !skill.prerequisites.is_empty()
            {
                return Err(format!(
                    "root skill {} needs a direct practice path",
                    skill.id
                ));
            }
        } else if skill.prerequisites.is_empty()
            || skill.qualifying_event.is_none()
            || skill.qualifying_count.is_none()
            || (skill.qualifying_event.as_deref() == Some("weapon_defeats")
                && skill.minimum_per_prerequisite.is_none())
        {
            return Err(format!(
                "advanced skill {} needs discovery requirements",
                skill.id
            ));
        }
        if skill
            .prerequisites
            .iter()
            .any(|id| !ids.contains(id.as_str()))
        {
            return Err(format!("skill {} names an unknown prerequisite", skill.id));
        }
    }
    Ok(())
}

impl WorldRepository {
    pub fn skills(&self, token: &str) -> Result<ApiResponse<SkillsResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
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
        let key = authenticate(&mut state, token, &self.config)?;
        super::phase4::validate_request_id(&request.request_id)?;
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
            skill_id: request.skill_id.clone(),
            target_account_id: None,
            skills: skills_view(&state, &key),
            message: "Choose a depth-one discipline and take its first practical step.".to_owned(),
            reason: None,
        };
        let Some(skill_id) = request.skill_id.as_deref() else {
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

    pub fn teach_skill(
        &self,
        token: &str,
        request: SkillRequest,
    ) -> Result<ApiResponse<SkillResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
        super::phase4::validate_request_id(&request.request_id)?;
        let actor_account = super::phase4::account_id(&state, &key);
        let cache = format!("skill-teach:{actor_account}:{}", request.request_id);
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
            skill_id: request.skill_id.clone(),
            target_account_id: request.target_account_id.clone(),
            skills: skills_view(&state, &key),
            message: "A school lesson needs a mastered discipline and a willing neighbour."
                .to_owned(),
            reason: None,
        };
        let Some(skill_id) = request.skill_id.as_deref() else {
            response.reason = Some("Name the mastered skill to teach.".to_owned());
            return finish_skill_action(self, &mut state, cache, response);
        };
        let Some(target_account) = request.target_account_id.as_deref() else {
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
        if definition.depth == 1 {
            record_practice(&mut state, &target_key, skill_id);
        } else {
            state
                .identities
                .get_mut(&target_key)
                .expect("target identity exists")
                .skills
                .known
                .push(skill_id.to_owned());
        }
        response.accepted = true;
        response.message = format!(
            "{} demonstrated {}; the learner still needs to practise it.",
            super::phase4::account_name(&state, &key),
            definition.name
        );
        super::phase4::record(
            &mut state,
            "school lesson",
            "A mastered discipline crosses between neighbours",
            &format!(
                "A nearby player received a formal {} lesson.",
                definition.name
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
    SkillsResponse {
        skills: catalog()
            .skills
            .iter()
            .map(|definition| skill_view(&identity.skills, definition))
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

pub(super) fn record_qualifying_event(state: &mut RepositoryState, key: &str, event: &str) {
    let identity = state.identities.get_mut(key).expect("identity exists");
    let count = identity
        .skills
        .qualifying_events
        .entry(event.to_owned())
        .or_insert(0);
    *count = count.saturating_add(1);
    discover_eligible(state, key);
}

pub(super) fn record_weapon_defeat(state: &mut RepositoryState, key: &str, weapon: WeaponKind) {
    let family = match weapon {
        WeaponKind::IronSword => "sword",
        WeaponKind::ImprovisedClub => "unarmed",
    };
    record_qualifying_event(state, key, &format!("weapon_defeats:{family}"));
}

fn discover_eligible(state: &mut RepositoryState, key: &str) {
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
        return;
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
            counts.iter().sum::<u32>() >= needed && counts.iter().all(|count| *count >= minimum)
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
