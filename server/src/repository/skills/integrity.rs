use super::super::models::SkillLedger;
use super::catalog::catalog;
use std::collections::HashSet;

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
                        .all(|prerequisite| super::mastery(ledger, prerequisite) >= 5)
                        && super::qualifying_requirements_met(ledger, definition)
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
