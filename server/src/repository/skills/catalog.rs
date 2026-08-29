//! Data loading and validation for the server-owned skill catalogue.

use macroquad_toolkit::data_loader::parse_json_labeled;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::OnceLock;
use tarrowyn_protocol::SkillFamily;

const MAX_SKILL_ID_CHARS: usize = 160;
const MAX_SKILL_NAME_CHARS: usize = 120;
const MAX_SKILL_TEXT_CHARS: usize = 512;
const MAX_PREREQUISITES: usize = 16;

pub(super) const SKILLS_JSON: &str =
    macroquad_toolkit::include_json_str!("../../../../assets/data/skills.json");

#[derive(Debug, Deserialize)]
pub(super) struct SkillManifest {
    version: u32,
    pub(super) skills: Vec<SkillDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct SkillDefinition {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) family: SkillFamily,
    pub(super) depth: u8,
    pub(super) description: String,
    pub(super) entry_hint: String,
    #[serde(default)]
    pub(super) practice_key: Option<String>,
    #[serde(default)]
    pub(super) prerequisites: Vec<String>,
    #[serde(default)]
    pub(super) qualifying_event: Option<String>,
    #[serde(default)]
    pub(super) qualifying_count: Option<u32>,
    #[serde(default)]
    pub(super) minimum_per_prerequisite: Option<u32>,
    #[serde(default)]
    pub(super) directly_teachable: bool,
}

static CATALOG: OnceLock<SkillManifest> = OnceLock::new();

pub(super) fn catalog() -> &'static SkillManifest {
    CATALOG.get_or_init(|| {
        let manifest: SkillManifest = parse_json_labeled("skills.json", SKILLS_JSON)
            .expect("skills content JSON must be valid");
        validate_manifest(&manifest).expect("skills content must satisfy its schema");
        manifest
    })
}

pub(crate) fn validate_catalog() -> Result<(), String> {
    let manifest: SkillManifest = parse_json_labeled("skills.json", SKILLS_JSON)
        .map_err(|error| format!("skills JSON is invalid: {error}"))?;
    validate_manifest(&manifest)
}

pub(super) fn validate_manifest(manifest: &SkillManifest) -> Result<(), String> {
    if manifest.version == 0 || manifest.skills.is_empty() {
        return Err("skills content needs a positive version and at least one skill".to_owned());
    }
    let ids: HashSet<&str> = manifest
        .skills
        .iter()
        .map(|skill| skill.id.as_str())
        .collect();
    if ids.len() != manifest.skills.len() || ids.iter().any(|id| !bounded(id, MAX_SKILL_ID_CHARS)) {
        return Err("skill IDs must be unique and non-empty".to_owned());
    }
    for skill in &manifest.skills {
        if !(1..=5).contains(&skill.depth)
            || !bounded(&skill.name, MAX_SKILL_NAME_CHARS)
            || !bounded(&skill.description, MAX_SKILL_TEXT_CHARS)
            || !bounded(&skill.entry_hint, MAX_SKILL_TEXT_CHARS)
            || skill.prerequisites.len() > MAX_PREREQUISITES
        {
            return Err(format!(
                "skill {} has invalid identity, depth, description, or entry hint",
                skill.id
            ));
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
            || skill.qualifying_count.is_none_or(|count| count == 0)
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
        let mut prerequisites = HashSet::new();
        if skill
            .prerequisites
            .iter()
            .any(|id| !bounded(id, MAX_SKILL_ID_CHARS) || !prerequisites.insert(id.as_str()))
        {
            return Err(format!(
                "skill {} has invalid or duplicate prerequisites",
                skill.id
            ));
        }
        if skill
            .practice_key
            .as_deref()
            .is_some_and(|key| !bounded(key, MAX_SKILL_ID_CHARS) || !ids.contains(key))
        {
            return Err(format!("skill {} names an invalid practice key", skill.id));
        }
        if skill
            .qualifying_event
            .as_deref()
            .is_some_and(|event| !bounded(event, MAX_SKILL_ID_CHARS))
        {
            return Err(format!(
                "skill {} names an invalid qualifying event",
                skill.id
            ));
        }
        if skill.minimum_per_prerequisite == Some(0) {
            return Err(format!(
                "skill {} has a zero prerequisite threshold",
                skill.id
            ));
        }
    }
    let mut visited = HashSet::new();
    for skill in &manifest.skills {
        if has_prerequisite_cycle(
            &skill.id,
            &manifest.skills,
            &mut HashSet::new(),
            &mut visited,
        ) {
            return Err(format!(
                "skill {} participates in a prerequisite cycle",
                skill.id
            ));
        }
    }
    Ok(())
}

fn bounded(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}

fn has_prerequisite_cycle(
    skill_id: &str,
    skills: &[SkillDefinition],
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) -> bool {
    if visited.contains(skill_id) {
        return false;
    }
    if !visiting.insert(skill_id.to_owned()) {
        return true;
    }
    let cycle = skills
        .iter()
        .find(|skill| skill.id == skill_id)
        .is_some_and(|skill| {
            skill
                .prerequisites
                .iter()
                .any(|prerequisite| has_prerequisite_cycle(prerequisite, skills, visiting, visited))
        });
    visiting.remove(skill_id);
    if !cycle {
        visited.insert(skill_id.to_owned());
    }
    cycle
}
