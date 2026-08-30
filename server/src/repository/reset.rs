use super::models::RepositoryState;
use std::collections::HashSet;
use tarrowyn_protocol::{
    ClaimLifecycleStatus, ClaimStatus, ExpeditionStatus, FrontierEvent, ProposalStatus,
    ServiceOrderStatus, WorldEvent,
};

const RESET_ACCOUNT: &str = "former-resident";
const RESET_NAME: &str = "Former resident";

pub(super) fn anonymize_orphaned_public_history(state: &mut RepositoryState) {
    let account_ids: HashSet<String> = state
        .identities
        .values()
        .map(|identity| identity.account_id.clone())
        .collect();
    for message in &mut state.chat_history {
        if is_orphaned_account(&message.account_id, &account_ids) {
            let account_id = message.account_id.clone();
            anonymize_chat(message, &account_id);
        }
    }
    for audit in &mut state.phase6.audits {
        if is_orphaned_account(&audit.actor_account_id, &account_ids) {
            audit.actor_account_id = RESET_ACCOUNT.to_owned();
        }
    }
    for event in &mut state.events {
        let orphaned_ids = orphaned_event_accounts(&event.event, &account_ids);
        for account_id in orphaned_ids {
            anonymize_event(&mut event.event, &account_id, "");
        }
    }
}

fn orphaned_event_accounts(event: &WorldEvent, account_ids: &HashSet<String>) -> Vec<String> {
    let candidates: Vec<&str> = match event {
        WorldEvent::Presence(presence) => vec![&presence.account_id],
        WorldEvent::Chat(message) => vec![&message.account_id],
        WorldEvent::Trade(trade) => vec![&trade.creator_account_id, &trade.recipient_account_id],
        WorldEvent::Frontier(FrontierEvent::Claim(claim)) => vec![&claim.owner_account_id],
        WorldEvent::Frontier(FrontierEvent::Expedition(expedition)) => {
            std::iter::once(expedition.leader_account_id.as_str())
                .chain(
                    expedition
                        .members
                        .iter()
                        .map(|member| member.account_id.as_str()),
                )
                .collect()
        }
        WorldEvent::Clock(_)
        | WorldEvent::Farming(_)
        | WorldEvent::TavernNotice(_)
        | WorldEvent::Chronicle(_)
        | WorldEvent::Frontier(FrontierEvent::Threat(_))
        | WorldEvent::Frontier(FrontierEvent::Opportunity(_)) => Vec::new(),
    };
    candidates
        .into_iter()
        .filter(|account_id| is_orphaned_account(account_id, account_ids))
        .map(str::to_owned)
        .collect()
}

fn is_orphaned_account(account_id: &str, account_ids: &HashSet<String>) -> bool {
    account_id != RESET_ACCOUNT && !account_ids.contains(account_id)
}

pub(super) fn reset_guest(state: &mut RepositoryState, identity_key: &str) {
    let Some(identity) = state.identities.get(identity_key) else {
        return;
    };
    let old_account_id = identity.account_id.clone();
    let old_display_name = identity.display_name.clone();

    state
        .sessions
        .retain(|_, session| session.identity_key != identity_key);
    super::session::record_offline_presence_if_last_session(state, identity_key);
    state.phase3.contracts.remove(identity_key);
    state
        .phase3
        .expedition_credentials
        .retain(|account_id| account_id != &old_account_id);
    state.phase3.request_results.retain(|key, response| {
        !super::phase3::is_request_cache_for_identity(key, identity_key, response)
    });
    state.phase4.profiles.remove(identity_key);
    state.phase4.materials.remove(identity_key);
    state.phase4.credentials.remove(identity_key);
    state.phase4.known_by.remove(identity_key);
    state.phase4.combat.remove(identity_key);
    state.phase4.lessons.retain(|lesson| {
        lesson.teacher_account_id != old_account_id && lesson.learner_account_id != old_account_id
    });
    state.phase4.request_results.retain(|key, response| {
        !super::phase4::is_request_cache_for_account(key, &old_account_id, response)
    });
    state.phase5.travel.remove(identity_key);
    state.phase5.request_results.retain(|key, response| {
        !super::phase5::is_exact_request_cache_for_identity(key, identity_key, response)
    });
    state.trades.retain(|_, trade| {
        trade.creator_account_id != old_account_id && trade.recipient_account_id != old_account_id
    });
    super::phase5::close_deleted_account_orders(state, &old_account_id);
    reset_phase3_public_ownership(state, &old_account_id);
    reset_phase4_public_ownership(state, &old_account_id);
    anonymize_public_history(state, &old_account_id, &old_display_name);
    state.phase6.audits.retain(|record| {
        record.actor_account_id != old_account_id && record.target != old_account_id
    });
    state.phase6.moderation_results.retain(|key, response| {
        key != &format!("moderation:{identity_key}:{}", response.request_id)
    });
    state
        .phase6
        .auth_revoke_results
        .retain(|key, response| key != &format!("{identity_key}:{}", response.request_id));
    state
        .phase6
        .auth_revoke_guest_tokens
        .retain(|_, revoked_identity_key| revoked_identity_key != identity_key);
    state
        .phase6
        .moderation_last_report_ticks
        .remove(identity_key);
    state.phase6.request_results.retain(|key, response| {
        !super::phase6::is_support_replay_key_for_account(key, &old_account_id, response)
    });
    state.identities.remove(identity_key);
}

fn anonymize_public_history(state: &mut RepositoryState, old_account_id: &str, old_name: &str) {
    for message in &mut state.chat_history {
        anonymize_chat(message, old_account_id);
    }
    for event in &mut state.events {
        anonymize_event(&mut event.event, old_account_id, old_name);
    }
    for entry in state
        .phase3
        .chronicle
        .iter_mut()
        .chain(state.phase3.chronicle_archive.iter_mut())
    {
        anonymize_chronicle(entry, old_name);
    }
    for settlement in &mut state.phase5.settlements {
        for entry in &mut settlement.chronicle {
            anonymize_chronicle(entry, old_name);
        }
    }
}

fn anonymize_event(event: &mut WorldEvent, old_account_id: &str, old_name: &str) {
    match event {
        WorldEvent::Presence(presence) => {
            if presence.account_id == old_account_id {
                presence.account_id = RESET_ACCOUNT.to_owned();
                presence.display_name = RESET_NAME.to_owned();
                presence.online = false;
            }
        }
        WorldEvent::Chat(message) => anonymize_chat(message, old_account_id),
        WorldEvent::Trade(trade) => {
            if trade.creator_account_id == old_account_id {
                trade.creator_account_id = RESET_ACCOUNT.to_owned();
                trade.creator_name = RESET_NAME.to_owned();
            }
            if trade.recipient_account_id == old_account_id {
                trade.recipient_account_id = RESET_ACCOUNT.to_owned();
                trade.recipient_name = RESET_NAME.to_owned();
            }
        }
        WorldEvent::Frontier(FrontierEvent::Claim(claim)) => {
            if claim.owner_account_id == old_account_id {
                claim.owner_account_id = RESET_ACCOUNT.to_owned();
                claim.owner_name = RESET_NAME.to_owned();
                claim.status = ClaimStatus::Reclaimed;
            }
        }
        WorldEvent::Frontier(FrontierEvent::Expedition(expedition)) => {
            reset_expedition(expedition, old_account_id)
        }
        WorldEvent::Chronicle(entry) => anonymize_chronicle(entry, old_name),
        WorldEvent::Clock(_) | WorldEvent::Farming(_) | WorldEvent::TavernNotice(_) => {}
        WorldEvent::Frontier(FrontierEvent::Threat(_))
        | WorldEvent::Frontier(FrontierEvent::Opportunity(_)) => {}
    }
}

fn anonymize_chat(message: &mut tarrowyn_protocol::ChatMessage, old_account_id: &str) {
    if message.account_id == old_account_id {
        message.account_id = RESET_ACCOUNT.to_owned();
        message.display_name = RESET_NAME.to_owned();
        message.text = "[message removed after development identity reset]".to_owned();
    }
}

fn anonymize_chronicle(entry: &mut tarrowyn_protocol::ChronicleEntry, old_name: &str) {
    if !old_name.is_empty() {
        entry.title = entry.title.replace(old_name, RESET_NAME);
        entry.text = entry.text.replace(old_name, RESET_NAME);
    }
}

fn reset_expedition(expedition: &mut tarrowyn_protocol::Expedition, old_account_id: &str) {
    let leader_reset = expedition.leader_account_id == old_account_id;
    expedition
        .members
        .retain(|member| member.account_id != old_account_id);
    if leader_reset {
        expedition.leader_account_id = expedition
            .members
            .first()
            .map(|member| member.account_id.clone())
            .unwrap_or_else(|| RESET_ACCOUNT.to_owned());
    }
    if expedition.members.is_empty()
        && matches!(
            expedition.status,
            ExpeditionStatus::Planning | ExpeditionStatus::Launched
        )
    {
        expedition.status = ExpeditionStatus::Retreated;
        expedition.outcome =
            Some("The party returned when its development identity was reset.".to_owned());
    }
}

fn reset_phase3_public_ownership(state: &mut RepositoryState, old_account_id: &str) {
    if let Some(claim) = state.phase3.claim.as_mut() {
        if claim.owner_account_id == old_account_id {
            claim.owner_account_id = RESET_ACCOUNT.to_owned();
            claim.owner_name = RESET_NAME.to_owned();
            claim.status = ClaimStatus::Reclaimed;
        }
    }
    if let Some(expedition) = state.phase3.expedition.as_mut() {
        reset_expedition(expedition, old_account_id);
    }
}

fn reset_phase4_public_ownership(state: &mut RepositoryState, old_account_id: &str) {
    for claim in &mut state.phase4.claims {
        if claim.owner_account_id.as_deref() == Some(old_account_id) {
            claim.owner_account_id = None;
            claim.owner_name = None;
            claim.status = ClaimLifecycleStatus::Reclaimed;
            claim.approved_by = None;
            claim.building_access = false;
            claim.last_active_tick = state.tick;
            claim.inspection_note =
                "The development identity was reset; this plot is available again.".to_owned();
            if !state.phase4.available_plots.contains(&claim.position) {
                state.phase4.available_plots.push(claim.position);
            }
        } else if claim.approved_by.as_deref() == Some(old_account_id) {
            claim.approved_by = None;
        }
    }
    for office in &mut state.phase4.governance.offices {
        if office.holder_account_id.as_deref() == Some(old_account_id) {
            office.holder_account_id = None;
            office.holder_name = None;
            office.vacant = true;
            office.vacancy_reason = Some(
                "The development office-holder was reset; a new player may take responsibility."
                    .to_owned(),
            );
        }
    }
    for proposal in &mut state.phase4.governance.proposals {
        if proposal.proposer_account_id == old_account_id {
            proposal.proposer_account_id = RESET_ACCOUNT.to_owned();
            proposal.proposer_name = RESET_NAME.to_owned();
            if proposal.status == ProposalStatus::Proposed {
                proposal.status = ProposalStatus::Rejected;
            }
        }
        if proposal.approved_by.as_deref() == Some(old_account_id) {
            proposal.approved_by = None;
        }
    }
    for decision in &mut state.phase4.governance.decisions {
        if decision.actor_account_id == old_account_id {
            decision.actor_account_id = RESET_ACCOUNT.to_owned();
            decision.actor_name = RESET_NAME.to_owned();
        }
    }
    for receipt in &mut state.phase4.governance.tax_ledger {
        if receipt.payer_account_id == old_account_id {
            receipt.payer_account_id = RESET_ACCOUNT.to_owned();
            receipt.payer_name = RESET_NAME.to_owned();
        }
    }
    if let Some(policy) = state.phase4.governance.taxation.as_mut() {
        if policy.payer == old_account_id {
            policy.payer = RESET_ACCOUNT.to_owned();
        }
        if policy.recipient == old_account_id {
            policy.recipient = RESET_ACCOUNT.to_owned();
        }
    }
    for index in 0..state.phase4.orders.len() {
        let requester_reset = state.phase4.orders[index].requester_account_id == old_account_id;
        let provider_reset =
            state.phase4.orders[index].provider_account_id.as_deref() == Some(old_account_id);
        if requester_reset {
            let order = &mut state.phase4.orders[index];
            order.requester_account_id = RESET_ACCOUNT.to_owned();
            order.requester_name = RESET_NAME.to_owned();
            if matches!(
                order.status,
                ServiceOrderStatus::Open | ServiceOrderStatus::Accepted
            ) {
                order.status = ServiceOrderStatus::Cancelled;
            }
        }
        if provider_reset {
            if state.phase4.orders[index].status == ServiceOrderStatus::Accepted {
                let order = state.phase4.orders[index].clone();
                super::phase4::restore_service_order_escrow(state, &order);
            }
            let order = &mut state.phase4.orders[index];
            order.provider_account_id = None;
            order.provider_name = None;
            if order.status == ServiceOrderStatus::Accepted {
                order.status = ServiceOrderStatus::Cancelled;
            }
        }
    }
    for item in &mut state.phase4.knowledge {
        item.discovered_by
            .retain(|account_id| account_id != old_account_id);
    }
}
