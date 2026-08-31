use super::super::models::{RepositoryState, SkillLedger};
use super::super::{add_notice, meta, record_command_outcome, RepositoryError, WorldRepository};
use super::catalog::{catalog, SkillDefinition};
use tarrowyn_protocol::{
    ApiResponse, SkillResponse, SkillStatus, SkillView, SkillsResponse, WeaponKind,
};

pub(crate) fn finish_skill_action(
    repository: &WorldRepository,
    state: &mut RepositoryState,
    cache: String,
    response: SkillResponse,
) -> Result<ApiResponse<SkillResponse>, RepositoryError> {
    state.phase4.request_results.insert(
        cache,
        super::super::phase4::Phase4Response::Skill(response.clone()),
    );
    record_command_outcome(state, response.accepted);
    repository.persist(state)?;
    Ok(ApiResponse {
        meta: meta(
            state.tick,
            Some(response.request_id.clone()),
            Some(state.cursor),
        ),
        data: response,
    })
}

pub(crate) fn skills_view(state: &RepositoryState, key: &str) -> SkillsResponse {
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

pub(crate) fn record_practice(state: &mut RepositoryState, key: &str, skill_id: &str) {
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

pub(crate) fn record_qualifying_event(state: &mut RepositoryState, key: &str, event: &str) -> bool {
    let identity = state.identities.get_mut(key).expect("identity exists");
    let count = identity
        .skills
        .qualifying_events
        .entry(event.to_owned())
        .or_insert(0);
    *count = count.saturating_add(1);
    discover_eligible(state, key)
}

pub(crate) fn storm_magic_discovered(state: &RepositoryState, key: &str) -> bool {
    let ledger = &state.identities.get(key).expect("identity exists").skills;
    let Some(definition) = catalog()
        .skills
        .iter()
        .find(|skill| skill.id == "storm-magic")
    else {
        return false;
    };
    ledger
        .known
        .iter()
        .any(|skill_id| skill_id == "storm-magic")
        && advanced_skill_ready(ledger, definition)
}

pub(crate) fn storm_prerequisites_mastered(state: &RepositoryState, key: &str) -> bool {
    let ledger = &state.identities.get(key).expect("identity exists").skills;
    ["wind-magic", "water-magic", "electricity-magic"]
        .iter()
        .all(|skill_id| mastery(ledger, skill_id) >= 5)
}

pub(crate) fn record_weapon_defeat(state: &mut RepositoryState, key: &str, weapon: WeaponKind) {
    if let Some(family) = weapon.weapon_fighting_family() {
        record_qualifying_event(state, key, &format!("weapon_defeats:{family}"));
    }
}

pub(crate) fn discover_eligible(state: &mut RepositoryState, key: &str) -> bool {
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
                && advanced_skill_ready(&identity.skills, definition)
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

pub(crate) fn qualifying_requirements_met(
    ledger: &SkillLedger,
    definition: &SkillDefinition,
) -> bool {
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

fn advanced_skill_ready(ledger: &SkillLedger, definition: &SkillDefinition) -> bool {
    definition.depth > 1
        && definition
            .prerequisites
            .iter()
            .all(|prerequisite| mastery(ledger, prerequisite) >= 5)
        && qualifying_requirements_met(ledger, definition)
}

fn skill_view(ledger: &SkillLedger, definition: &SkillDefinition) -> SkillView {
    let skill_mastery = mastery(ledger, &definition.id);
    let discovered = ledger.known.iter().any(|known| known == &definition.id);
    let usable = if definition.depth == 1 {
        skill_mastery > 0
    } else {
        discovered && advanced_skill_ready(ledger, definition)
    };
    let status = if definition.depth == 1 {
        if skill_mastery == 0 {
            SkillStatus::Available
        } else if skill_mastery >= 5 {
            SkillStatus::Mastered
        } else {
            SkillStatus::Practising
        }
    } else if discovered {
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
        usable,
        status,
        description: definition.description.clone(),
        entry_hint,
    }
}

pub(crate) fn mastery(ledger: &SkillLedger, skill_id: &str) -> u8 {
    let practice = ledger.practice.get(skill_id).copied().unwrap_or(0);
    if practice == 0 {
        0
    } else {
        (practice / 4 + 1).min(5) as u8
    }
}

pub(crate) fn teacher_can_teach(ledger: &SkillLedger, definition: &SkillDefinition) -> bool {
    if definition.depth == 1 {
        mastery(ledger, &definition.id) >= 5
    } else {
        definition.directly_teachable
            && ledger
                .known
                .iter()
                .any(|skill_id| skill_id == &definition.id)
            && advanced_skill_ready(ledger, definition)
    }
}
