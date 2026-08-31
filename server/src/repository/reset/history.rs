use super::super::models::RepositoryState;
use super::{RESET_ACCOUNT, RESET_NAME};
use std::collections::HashSet;
use tarrowyn_protocol::{FrontierEvent, WorldEvent};

pub fn anonymize_orphaned_public_history(state: &mut RepositoryState) {
    let account_ids: HashSet<String> = state
        .identities
        .values()
        .map(|identity| identity.account_id.clone())
        .collect();
    let active_names: HashSet<String> = state
        .identities
        .values()
        .map(|identity| identity.display_name.clone())
        .collect();
    let orphaned_names = orphaned_public_display_names(state, &account_ids, &active_names);
    for message in &mut state.chat_history {
        if is_orphaned_account(&message.account_id, &account_ids) {
            let account_id = message.account_id.clone();
            super::anonymize_chat(message, &account_id);
        }
    }
    super::anonymize_orphaned_audits(&mut state.phase6.audits, &account_ids);
    anonymize_orphaned_audit_names(&mut state.phase6.audits, &orphaned_names);
    for event in &mut state.events {
        let orphaned_ids = super::orphaned_event_accounts(&event.event, &account_ids);
        for account_id in orphaned_ids {
            super::anonymize_event(&mut event.event, &account_id, "");
        }
    }
    anonymize_orphaned_chronicles(state, &orphaned_names);
}

fn orphaned_public_display_names(
    state: &RepositoryState,
    account_ids: &HashSet<String>,
    active_names: &HashSet<String>,
) -> Vec<String> {
    let mut names = Vec::new();
    for message in &state.chat_history {
        if is_orphaned_account(&message.account_id, account_ids) {
            push_orphaned_name(
                &mut names,
                &message.account_id,
                &message.display_name,
                account_ids,
                active_names,
            );
        }
    }
    for record in &state.events {
        collect_event_name(&mut names, &record.event, account_ids, active_names);
    }
    for trade in state.trades.values() {
        push_orphaned_name(
            &mut names,
            &trade.creator_account_id,
            &trade.creator_name,
            account_ids,
            active_names,
        );
        push_orphaned_name(
            &mut names,
            &trade.recipient_account_id,
            &trade.recipient_name,
            account_ids,
            active_names,
        );
    }
    collect_phase3_names(&mut names, state, account_ids, active_names);
    collect_phase4_names(&mut names, state, account_ids, active_names);
    for order in &state.phase5.market_orders {
        push_orphaned_name(
            &mut names,
            &order.owner_account_id,
            &order.owner_name,
            account_ids,
            active_names,
        );
    }
    names
}

fn collect_event_name(
    names: &mut Vec<String>,
    event: &WorldEvent,
    account_ids: &HashSet<String>,
    active_names: &HashSet<String>,
) {
    match event {
        WorldEvent::Presence(presence) => push_orphaned_name(
            names,
            &presence.account_id,
            &presence.display_name,
            account_ids,
            active_names,
        ),
        WorldEvent::Chat(message) => push_orphaned_name(
            names,
            &message.account_id,
            &message.display_name,
            account_ids,
            active_names,
        ),
        WorldEvent::Trade(trade) => {
            push_orphaned_name(
                names,
                &trade.creator_account_id,
                &trade.creator_name,
                account_ids,
                active_names,
            );
            push_orphaned_name(
                names,
                &trade.recipient_account_id,
                &trade.recipient_name,
                account_ids,
                active_names,
            );
        }
        WorldEvent::Frontier(FrontierEvent::Claim(claim)) => push_orphaned_name(
            names,
            &claim.owner_account_id,
            &claim.owner_name,
            account_ids,
            active_names,
        ),
        WorldEvent::Frontier(FrontierEvent::Expedition(expedition)) => {
            for member in &expedition.members {
                push_orphaned_name(
                    names,
                    &member.account_id,
                    &member.display_name,
                    account_ids,
                    active_names,
                );
            }
        }
        WorldEvent::Clock(_)
        | WorldEvent::Farming(_)
        | WorldEvent::TavernNotice(_)
        | WorldEvent::Chronicle(_)
        | WorldEvent::Frontier(FrontierEvent::Threat(_))
        | WorldEvent::Frontier(FrontierEvent::Opportunity(_)) => {}
    }
}

fn collect_phase3_names(
    names: &mut Vec<String>,
    state: &RepositoryState,
    account_ids: &HashSet<String>,
    active_names: &HashSet<String>,
) {
    if let Some(claim) = state.phase3.claim.as_ref() {
        push_orphaned_name(
            names,
            &claim.owner_account_id,
            &claim.owner_name,
            account_ids,
            active_names,
        );
    }
    if let Some(expedition) = state.phase3.expedition.as_ref() {
        for member in &expedition.members {
            push_orphaned_name(
                names,
                &member.account_id,
                &member.display_name,
                account_ids,
                active_names,
            );
        }
    }
}

fn collect_phase4_names(
    names: &mut Vec<String>,
    state: &RepositoryState,
    account_ids: &HashSet<String>,
    active_names: &HashSet<String>,
) {
    for office in &state.phase4.governance.offices {
        if let (Some(account_id), Some(display_name)) = (
            office.holder_account_id.as_deref(),
            office.holder_name.as_deref(),
        ) {
            push_orphaned_name(names, account_id, display_name, account_ids, active_names);
        }
    }
    for proposal in &state.phase4.governance.proposals {
        push_orphaned_name(
            names,
            &proposal.proposer_account_id,
            &proposal.proposer_name,
            account_ids,
            active_names,
        );
    }
    for decision in &state.phase4.governance.decisions {
        push_orphaned_name(
            names,
            &decision.actor_account_id,
            &decision.actor_name,
            account_ids,
            active_names,
        );
    }
    for collection in &state.phase4.governance.tax_ledger {
        push_orphaned_name(
            names,
            &collection.payer_account_id,
            &collection.payer_name,
            account_ids,
            active_names,
        );
    }
    for claim in &state.phase4.claims {
        if let (Some(account_id), Some(display_name)) = (
            claim.owner_account_id.as_deref(),
            claim.owner_name.as_deref(),
        ) {
            push_orphaned_name(names, account_id, display_name, account_ids, active_names);
        }
    }
    for order in &state.phase4.orders {
        push_orphaned_name(
            names,
            &order.requester_account_id,
            &order.requester_name,
            account_ids,
            active_names,
        );
        if let (Some(account_id), Some(display_name)) = (
            order.provider_account_id.as_deref(),
            order.provider_name.as_deref(),
        ) {
            push_orphaned_name(names, account_id, display_name, account_ids, active_names);
        }
    }
    for lesson in &state.phase4.lessons {
        push_orphaned_name(
            names,
            &lesson.teacher_account_id,
            &lesson.teacher_name,
            account_ids,
            active_names,
        );
        push_orphaned_name(
            names,
            &lesson.learner_account_id,
            &lesson.learner_name,
            account_ids,
            active_names,
        );
    }
}

fn push_orphaned_name(
    names: &mut Vec<String>,
    account_id: &str,
    display_name: &str,
    account_ids: &HashSet<String>,
    active_names: &HashSet<String>,
) {
    if !is_orphaned_account(account_id, account_ids)
        || display_name.trim().is_empty()
        || active_names.contains(display_name)
        || names.iter().any(|name| name == display_name)
    {
        return;
    }
    names.push(display_name.to_owned());
}

fn anonymize_orphaned_chronicles(state: &mut RepositoryState, orphaned_names: &[String]) {
    for entry in state
        .phase3
        .chronicle
        .iter_mut()
        .chain(state.phase3.chronicle_archive.iter_mut())
    {
        for old_name in orphaned_names {
            anonymize_chronicle(entry, old_name);
        }
    }
    for settlement in &mut state.phase5.settlements {
        for entry in &mut settlement.chronicle {
            for old_name in orphaned_names {
                anonymize_chronicle(entry, old_name);
            }
        }
    }
    for record in &mut state.events {
        if let WorldEvent::Chronicle(entry) = &mut record.event {
            for old_name in orphaned_names {
                anonymize_chronicle(entry, old_name);
            }
        }
    }
}

fn anonymize_orphaned_audit_names(
    audits: &mut std::collections::VecDeque<tarrowyn_protocol::AuditRecord>,
    orphaned_names: &[String],
) {
    for audit in audits {
        for old_name in orphaned_names {
            audit.note = audit.note.replace(old_name, RESET_NAME);
        }
        audit.note = audit.note.chars().take(240).collect();
    }
}

fn is_orphaned_account(account_id: &str, account_ids: &HashSet<String>) -> bool {
    account_id != RESET_ACCOUNT && !account_ids.contains(account_id)
}

pub(super) fn anonymize_chronicle(entry: &mut tarrowyn_protocol::ChronicleEntry, old_name: &str) {
    super::super::phase6::replace_bounded_chronicle_text(&mut entry.title, old_name, RESET_NAME);
    super::super::phase6::replace_bounded_chronicle_text(&mut entry.text, old_name, RESET_NAME);
}
