use super::super::models::RepositoryState;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tarrowyn_protocol::{
    AccountDeletionResponse, ClaimLifecycleStatus, ClaimStatus, FrontierEvent, ProposalStatus,
    ServiceOrderStatus, WorldEvent,
};

const DELETED_ACCOUNT: &str = "former-resident";
const DELETED_NAME: &str = "Former resident";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PendingAccountDeletion {
    pub(super) request_id: String,
    pub(super) account_id: String,
    pub(super) identity_key: String,
    pub(super) character_id: String,
}

pub(super) fn scheduled_response(pending: &PendingAccountDeletion) -> AccountDeletionResponse {
    AccountDeletionResponse {
        request_id: pending.request_id.clone(),
        account_id: pending.account_id.clone(),
        character_id: pending.character_id.clone(),
        accepted: true,
        status: "scheduled".to_owned(),
        reason: None,
    }
}

pub(super) fn process(state: &mut RepositoryState) {
    let pending = std::mem::take(&mut state.phase6.deletion_requests);
    for request in pending.into_values() {
        erase_account(state, &request);
    }
}

fn erase_account(state: &mut RepositoryState, request: &PendingAccountDeletion) {
    let Some(identity) = state.identities.get(&request.identity_key) else {
        return;
    };
    if identity.account_id != request.account_id {
        return;
    }
    let deleted_display_name = identity.display_name.clone();

    let deleted_tokens: HashSet<String> = state
        .phase6
        .sessions
        .iter()
        .filter(|(_, session)| session.account_id == request.account_id)
        .map(|(token, _)| token.clone())
        .collect();
    state
        .phase6
        .sessions
        .retain(|_, session| session.account_id != request.account_id);
    state
        .sessions
        .retain(|_, session| session.identity_key != request.identity_key);
    state.phase6.accounts.remove(&request.account_id);
    state.phase5.travel.remove(&request.identity_key);
    state.phase3.contracts.remove(&request.identity_key);
    state
        .phase3
        .expedition_credentials
        .retain(|account_id| account_id != &request.account_id);
    state
        .phase3
        .request_results
        .retain(|key, _| !key.starts_with(&format!("{}:", request.identity_key)));
    state.phase4.request_results.retain(|key, _| {
        !super::account::is_phase4_replay_key_for_account(key, &request.account_id)
    });
    state.phase5.request_results.retain(|key, _| {
        !super::super::phase5::is_request_cache_for_identity(key, &request.identity_key)
    });

    erase_private_phase4_state(state, request);
    state.trades.retain(|_, trade| {
        trade.creator_account_id != request.account_id
            && trade.recipient_account_id != request.account_id
    });
    super::super::phase5::close_deleted_account_orders(state, &request.account_id);

    anonymize_public_history(state, request, &deleted_display_name);
    state.phase6.auth_link_results.retain(|key, response| {
        !key.starts_with(&format!("{}:", request.identity_key))
            && response.account_id != request.account_id
    });
    state
        .phase6
        .auth_revoke_results
        .retain(|key, _| !key.starts_with(&format!("{}:", request.identity_key)));
    state
        .phase6
        .auth_refresh_results
        .retain(|_, response| !deleted_tokens.contains(&response.session.account_token));
    state
        .phase6
        .moderation_results
        .retain(|key, _| !key.contains(&format!(":{}:", request.identity_key)));
    state
        .phase6
        .moderation_last_report_ticks
        .remove(&request.identity_key);
    state
        .phase6
        .request_results
        .retain(|key, _| !key.starts_with(&format!("repair:{}:", request.account_id)));
    state.phase6.audits.retain(|record| {
        record.actor_account_id != request.account_id && record.target != request.account_id
    });
    super::audit(
        state,
        DELETED_ACCOUNT,
        "account.delete.completed",
        DELETED_ACCOUNT,
        "accepted",
        "Private account data was removed; public settlement history was anonymised.",
    );
    state.identities.remove(&request.identity_key);
}

fn erase_private_phase4_state(state: &mut RepositoryState, request: &PendingAccountDeletion) {
    state.phase4.profiles.remove(&request.identity_key);
    state.phase4.materials.remove(&request.identity_key);
    state.phase4.credentials.remove(&request.identity_key);
    state.phase4.known_by.remove(&request.identity_key);
    state.phase4.combat.remove(&request.identity_key);
    state.phase4.lessons.retain(|lesson| {
        lesson.teacher_account_id != request.account_id
            && lesson.learner_account_id != request.account_id
    });
    for claim in &mut state.phase4.claims {
        if claim.owner_account_id.as_deref() == Some(request.account_id.as_str()) {
            claim.owner_account_id = None;
            claim.owner_name = None;
            claim.status = ClaimLifecycleStatus::Reclaimed;
            claim.building_access = false;
            claim.last_active_tick = state.tick;
            claim.inspection_note =
                "The former holder left the settlement; this plot is available again.".to_owned();
            if !state.phase4.available_plots.contains(&claim.position) {
                state.phase4.available_plots.push(claim.position);
            }
        }
        if claim.approved_by.as_deref() == Some(request.account_id.as_str()) {
            claim.approved_by = None;
        }
    }
    for office in &mut state.phase4.governance.offices {
        if office.holder_account_id.as_deref() == Some(request.account_id.as_str()) {
            office.holder_account_id = None;
            office.holder_name = None;
            office.vacant = true;
            office.vacancy_reason = Some(
                "The former office-holder left the settlement; a new player may take responsibility."
                    .to_owned(),
            );
        }
    }
    for proposal in &mut state.phase4.governance.proposals {
        if proposal.proposer_account_id == request.account_id {
            proposal.proposer_account_id = DELETED_ACCOUNT.to_owned();
            proposal.proposer_name = DELETED_NAME.to_owned();
            if proposal.status == ProposalStatus::Proposed {
                proposal.status = ProposalStatus::Rejected;
            }
        }
        if proposal.approved_by.as_deref() == Some(request.account_id.as_str()) {
            proposal.approved_by = None;
        }
    }
    for decision in &mut state.phase4.governance.decisions {
        if decision.actor_account_id == request.account_id {
            decision.actor_account_id = DELETED_ACCOUNT.to_owned();
            decision.actor_name = DELETED_NAME.to_owned();
        }
    }
    for receipt in &mut state.phase4.governance.tax_ledger {
        if receipt.payer_account_id == request.account_id {
            receipt.payer_account_id = DELETED_ACCOUNT.to_owned();
            receipt.payer_name = DELETED_NAME.to_owned();
        }
    }
    if state
        .phase4
        .governance
        .taxation
        .as_ref()
        .is_some_and(|policy| {
            policy.payer == request.account_id || policy.recipient == request.account_id
        })
    {
        if let Some(policy) = state.phase4.governance.taxation.as_mut() {
            if policy.payer == request.account_id {
                policy.payer = DELETED_ACCOUNT.to_owned();
            }
            if policy.recipient == request.account_id {
                policy.recipient = DELETED_ACCOUNT.to_owned();
            }
        }
    }
    for index in 0..state.phase4.orders.len() {
        let requester_deleted =
            state.phase4.orders[index].requester_account_id == request.account_id;
        let provider_deleted = state.phase4.orders[index].provider_account_id.as_deref()
            == Some(request.account_id.as_str());
        if requester_deleted {
            let order = &mut state.phase4.orders[index];
            order.requester_account_id = DELETED_ACCOUNT.to_owned();
            order.requester_name = DELETED_NAME.to_owned();
            if matches!(
                order.status,
                ServiceOrderStatus::Open | ServiceOrderStatus::Accepted
            ) {
                order.status = ServiceOrderStatus::Cancelled;
            }
        }
        if provider_deleted {
            if state.phase4.orders[index].status == ServiceOrderStatus::Accepted {
                let order = state.phase4.orders[index].clone();
                super::super::phase4::restore_service_order_escrow(state, &order);
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
            .retain(|account_id| account_id != &request.account_id);
    }
}

fn anonymize_public_history(
    state: &mut RepositoryState,
    request: &PendingAccountDeletion,
    deleted_display_name: &str,
) {
    if let Some(claim) = state.phase3.claim.as_mut() {
        if claim.owner_account_id == request.account_id {
            claim.owner_account_id = DELETED_ACCOUNT.to_owned();
            claim.owner_name = DELETED_NAME.to_owned();
            claim.status = ClaimStatus::Abandoned;
        }
    }
    if let Some(expedition) = state.phase3.expedition.as_mut() {
        anonymize_expedition(expedition, request.account_id.as_str());
    }
    for message in &mut state.chat_history {
        anonymize_chat(message, request.account_id.as_str());
    }
    for event in &mut state.events {
        anonymize_event(
            &mut event.event,
            request.account_id.as_str(),
            deleted_display_name,
        );
    }
    for entry in state
        .phase3
        .chronicle
        .iter_mut()
        .chain(state.phase3.chronicle_archive.iter_mut())
    {
        anonymize_chronicle(entry, deleted_display_name);
    }
    for settlement in &mut state.phase5.settlements {
        for entry in &mut settlement.chronicle {
            anonymize_chronicle(entry, deleted_display_name);
        }
    }
}

fn anonymize_event(event: &mut WorldEvent, account_id: &str, deleted_display_name: &str) {
    match event {
        WorldEvent::Presence(presence) => {
            if presence.account_id == account_id {
                presence.account_id = DELETED_ACCOUNT.to_owned();
                presence.display_name = DELETED_NAME.to_owned();
                presence.online = false;
            }
        }
        WorldEvent::Chat(message) => anonymize_chat(message, account_id),
        WorldEvent::Trade(trade) => {
            if trade.creator_account_id == account_id {
                trade.creator_account_id = DELETED_ACCOUNT.to_owned();
                trade.creator_name = DELETED_NAME.to_owned();
            }
            if trade.recipient_account_id == account_id {
                trade.recipient_account_id = DELETED_ACCOUNT.to_owned();
                trade.recipient_name = DELETED_NAME.to_owned();
            }
        }
        WorldEvent::Frontier(FrontierEvent::Claim(claim)) => {
            if claim.owner_account_id == account_id {
                claim.owner_account_id = DELETED_ACCOUNT.to_owned();
                claim.owner_name = DELETED_NAME.to_owned();
                claim.status = ClaimStatus::Abandoned;
            }
        }
        WorldEvent::Frontier(FrontierEvent::Expedition(expedition)) => {
            anonymize_expedition(expedition, account_id)
        }
        WorldEvent::Frontier(FrontierEvent::Threat(_))
        | WorldEvent::Frontier(FrontierEvent::Opportunity(_)) => {}
        WorldEvent::Clock(_) | WorldEvent::Farming(_) | WorldEvent::TavernNotice(_) => {}
        WorldEvent::Chronicle(entry) => anonymize_chronicle(entry, deleted_display_name),
    }
}

fn anonymize_chronicle(entry: &mut tarrowyn_protocol::ChronicleEntry, deleted_display_name: &str) {
    if deleted_display_name.is_empty() {
        return;
    }
    entry.title = entry.title.replace(deleted_display_name, DELETED_NAME);
    entry.text = entry.text.replace(deleted_display_name, DELETED_NAME);
}

fn anonymize_chat(message: &mut tarrowyn_protocol::ChatMessage, account_id: &str) {
    if message.account_id == account_id {
        message.account_id = DELETED_ACCOUNT.to_owned();
        message.display_name = DELETED_NAME.to_owned();
        message.text = "[message removed after account deletion]".to_owned();
    }
}

fn anonymize_expedition(expedition: &mut tarrowyn_protocol::Expedition, account_id: &str) {
    let leader_deleted = expedition.leader_account_id == account_id;
    expedition
        .members
        .retain(|member| member.account_id != account_id);
    if leader_deleted {
        expedition.leader_account_id = expedition
            .members
            .first()
            .map(|member| member.account_id.clone())
            .unwrap_or_else(|| DELETED_ACCOUNT.to_owned());
    }
    if expedition.members.is_empty()
        && matches!(
            expedition.status,
            tarrowyn_protocol::ExpeditionStatus::Planning
                | tarrowyn_protocol::ExpeditionStatus::Launched
        )
    {
        expedition.status = tarrowyn_protocol::ExpeditionStatus::Retreated;
        expedition.outcome =
            Some("The party returned when its leader left the settlement.".to_owned());
    }
}
