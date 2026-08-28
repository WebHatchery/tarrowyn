//! Server-owned classless skill catalogue and authoritative practice ledger.

use super::models::{RepositoryState, SkillLedger};
use super::*;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::OnceLock;
use tarrowyn_protocol::{
    ApiResponse, SkillFamily, SkillStatus, SkillView, SkillsResponse, WeaponKind,
};

const SKILLS_JSON: &str = include_str!("../../../assets/data/skills.json");

#[derive(Debug, Deserialize)]
struct SkillManifest {
    version: u32,
    skills: Vec<SkillDefinition>,
}

#[derive(Debug, Deserialize)]
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
        let identity = state.identities.get(&key).expect("identity exists");
        let skills = catalog()
            .skills
            .iter()
            .map(|definition| skill_view(&identity.skills, definition))
            .collect();
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: SkillsResponse {
                skills,
                cursor: state.cursor,
            },
        })
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
