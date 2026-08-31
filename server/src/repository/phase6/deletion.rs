use super::super::models::RepositoryState;
use super::stable_fingerprint;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tarrowyn_protocol::{
    AccountDeletionResponse, ClaimLifecycleStatus, ClaimRecord, ClaimStatus, FrontierEvent,
    GovernanceState, LandClaim, ProposalStatus, ServiceOrder, ServiceOrderStatus, WorldEvent,
};

const DELETED_ACCOUNT: &str = "former-resident";
const DELETED_NAME: &str = "Former resident";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PendingAccountDeletion {
    pub(super) request_id: String,
    pub(super) account_id: String,
    pub(super) identity_key: String,
    pub(super) character_id: String,
    #[serde(default)]
    pub(super) replay_key: String,
}

pub(super) fn replay_key(token: &str, request_id: &str) -> String {
    format!("{}:{request_id}", stable_fingerprint(token))
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
        if erase_account(state, &request) && !request.replay_key.is_empty() {
            state
                .phase6
                .deletion_results
                .insert(request.replay_key.clone(), scheduled_response(&request));
        }
    }
}

fn erase_account(state: &mut RepositoryState, request: &PendingAccountDeletion) -> bool {
    let Some(identity) = state.identities.get(&request.identity_key) else {
        return false;
    };
    if identity.account_id != request.account_id {
        return false;
    }
    let deleted_display_name = identity.display_name.clone();

    let deleted_tokens: HashSet<String> = state
        .phase6
        .sessions
        .iter()
        .filter(|(_, session)| session.account_id == request.account_id)
        .map(|(token, _)| token.clone())
        .collect();
    let deleted_refresh_replays: HashSet<String> = state
        .phase6
        .auth_refresh_accounts
        .iter()
        .filter(|(_, account_id)| *account_id == &request.account_id)
        .map(|(key, _)| key.clone())
        .collect();
    state
        .phase6
        .sessions
        .retain(|_, session| session.account_id != request.account_id);
    state
        .sessions
        .retain(|_, session| session.identity_key != request.identity_key);
    super::super::record_offline_presence_if_last_session(state, &request.identity_key);
    state.phase6.accounts.remove(&request.account_id);
    state.phase5.travel.remove(&request.identity_key);
    state.phase3.contracts.remove(&request.identity_key);
    state
        .phase3
        .expedition_credentials
        .retain(|account_id| account_id != &request.account_id);
    state.phase3.request_results.retain(|key, response| {
        !super::super::phase3::is_request_cache_for_identity(key, &request.identity_key, response)
    });
    state.phase4.request_results.retain(|key, response| {
        !super::super::phase4::is_request_cache_for_account(key, &request.account_id, response)
    });
    state.phase5.request_results.retain(|key, response| {
        !super::super::phase5::is_exact_request_cache_for_identity(
            key,
            &request.identity_key,
            response,
        )
    });

    erase_private_phase4_state(state, request);
    anonymize_phase4_replay_claims(state, &request.account_id);
    anonymize_phase4_replay_service_orders(state, &request.account_id);
    anonymize_phase4_replay_governance(state, &request.account_id);
    anonymize_phase4_replay_knowledge(state, &request.account_id);
    anonymize_phase4_replay_skill_lessons(state, &request.account_id);
    invalidate_deleted_trade_replays(state, &request.account_id);
    state.trades.retain(|_, trade| {
        trade.creator_account_id != request.account_id
            && trade.recipient_account_id != request.account_id
    });
    super::super::phase5::close_deleted_account_orders(state, &request.account_id);
    anonymize_phase5_replay_orders(state, &request.account_id);

    anonymize_public_history(state, request, &deleted_display_name);
    anonymize_phase3_replay_frontier(state, &request.account_id);
    state.phase6.auth_link_results.retain(|key, response| {
        key != &format!("{}:{}", request.identity_key, response.request_id)
            && response.account_id != request.account_id
    });
    state
        .phase6
        .auth_link_tokens
        .retain(|_, identity_key| identity_key != &request.identity_key);
    state.phase6.auth_revoke_results.retain(|key, response| {
        key != &format!("{}:{}", request.identity_key, response.request_id)
    });
    state
        .phase6
        .auth_revoke_guest_tokens
        .retain(|_, identity_key| identity_key != &request.identity_key);
    state.phase6.auth_refresh_results.retain(|key, response| {
        !deleted_refresh_replays.contains(key)
            && !deleted_tokens.contains(&response.session.account_token)
    });
    state
        .phase6
        .auth_refresh_accounts
        .retain(|key, _| !deleted_refresh_replays.contains(key));
    state.phase6.moderation_results.retain(|key, response| {
        key != &format!(
            "moderation:{}:{}",
            request.identity_key, response.request_id
        )
    });
    state
        .phase6
        .moderation_last_report_ticks
        .remove(&request.identity_key);
    state.phase6.request_results.retain(|key, response| {
        !super::is_support_replay_key_for_account(key, &request.account_id, response)
    });
    state.phase6.audits.retain(|record| {
        record.actor_account_id != request.account_id && record.target != request.account_id
    });
    anonymize_audit_targets(
        &mut state.phase6.audits,
        &request.account_id,
        &deleted_display_name,
    );
    super::audit(
        state,
        DELETED_ACCOUNT,
        "account.delete.completed",
        DELETED_ACCOUNT,
        "accepted",
        "Private account data was removed; public settlement history was anonymised.",
    );
    state.identities.remove(&request.identity_key);
    true
}

fn anonymize_phase5_replay_orders(state: &mut RepositoryState, account_id: &str) {
    for response in state.phase5.request_results.values_mut() {
        let super::super::phase5::Phase5Response::Market(response) = response else {
            continue;
        };
        let Some(order) = response.order.as_mut() else {
            continue;
        };
        if order.owner_account_id == account_id {
            order.owner_account_id = DELETED_ACCOUNT.to_owned();
            order.owner_name = DELETED_NAME.to_owned();
        }
    }
}

fn anonymize_phase4_replay_claims(state: &mut RepositoryState, account_id: &str) {
    for response in state.phase4.request_results.values_mut() {
        let super::super::phase4::Phase4Response::Claim(response) = response else {
            continue;
        };
        let mut freed_positions = Vec::new();
        if let Some(claim) = response.claim.as_mut() {
            if anonymize_phase4_replay_claim(claim, account_id, state.tick) {
                freed_positions.push(claim.position);
            }
        }
        for claim in &mut response.claims.claims {
            if anonymize_phase4_replay_claim(claim, account_id, state.tick) {
                freed_positions.push(claim.position);
            }
        }
        for position in freed_positions {
            if !response.claims.available_plots.contains(&position) {
                response.claims.available_plots.push(position);
            }
        }
    }
}

fn anonymize_phase4_replay_claim(
    claim: &mut ClaimRecord,
    account_id: &str,
    current_tick: u64,
) -> bool {
    let owner_deleted = claim.owner_account_id.as_deref() == Some(account_id);
    if owner_deleted {
        claim.owner_account_id = None;
        claim.owner_name = None;
        claim.status = ClaimLifecycleStatus::Reclaimed;
        claim.building_access = false;
        claim.last_active_tick = current_tick;
        claim.inspection_note =
            "The former holder left the settlement; this plot is available again.".to_owned();
    }
    if claim.approved_by.as_deref() == Some(account_id) {
        claim.approved_by = None;
    }
    owner_deleted
}

fn anonymize_phase4_replay_service_orders(state: &mut RepositoryState, account_id: &str) {
    for response in state.phase4.request_results.values_mut() {
        let super::super::phase4::Phase4Response::Profession(response) = response else {
            continue;
        };
        if let Some(order) = response.order.as_mut() {
            anonymize_phase4_replay_service_order(order, account_id);
        }
        for order in &mut response.professions.orders {
            anonymize_phase4_replay_service_order(order, account_id);
        }
    }
}

fn anonymize_phase4_replay_service_order(order: &mut ServiceOrder, account_id: &str) {
    if order.requester_account_id == account_id {
        order.requester_account_id = DELETED_ACCOUNT.to_owned();
        order.requester_name = DELETED_NAME.to_owned();
        if matches!(
            order.status,
            ServiceOrderStatus::Open | ServiceOrderStatus::Accepted
        ) {
            order.status = ServiceOrderStatus::Cancelled;
        }
    }
    if order.provider_account_id.as_deref() == Some(account_id) {
        order.provider_account_id = None;
        order.provider_name = None;
        if order.status == ServiceOrderStatus::Accepted {
            order.status = ServiceOrderStatus::Cancelled;
        }
    }
}

fn anonymize_phase4_replay_governance(state: &mut RepositoryState, account_id: &str) {
    for response in state.phase4.request_results.values_mut() {
        let super::super::phase4::Phase4Response::Governance(response) = response else {
            continue;
        };
        anonymize_governance(&mut response.governance, account_id);
    }
}

fn anonymize_governance(governance: &mut GovernanceState, account_id: &str) {
    for office in &mut governance.offices {
        if office.holder_account_id.as_deref() == Some(account_id) {
            office.holder_account_id = None;
            office.holder_name = None;
            office.vacant = true;
            office.vacancy_reason = Some(
                "The former office-holder left the settlement; a new player may take responsibility."
                    .to_owned(),
            );
        }
    }
    for proposal in &mut governance.proposals {
        if proposal.proposer_account_id == account_id {
            proposal.proposer_account_id = DELETED_ACCOUNT.to_owned();
            proposal.proposer_name = DELETED_NAME.to_owned();
            if proposal.status == ProposalStatus::Proposed {
                proposal.status = ProposalStatus::Rejected;
            }
        }
        if proposal.approved_by.as_deref() == Some(account_id) {
            proposal.approved_by = None;
        }
    }
    for decision in &mut governance.decisions {
        if decision.actor_account_id == account_id {
            decision.actor_account_id = DELETED_ACCOUNT.to_owned();
            decision.actor_name = DELETED_NAME.to_owned();
        }
    }
    for receipt in &mut governance.tax_ledger {
        if receipt.payer_account_id == account_id {
            receipt.payer_account_id = DELETED_ACCOUNT.to_owned();
            receipt.payer_name = DELETED_NAME.to_owned();
        }
    }
    if let Some(policy) = governance.taxation.as_mut() {
        if policy.payer == account_id {
            policy.payer = DELETED_ACCOUNT.to_owned();
        }
        if policy.recipient == account_id {
            policy.recipient = DELETED_ACCOUNT.to_owned();
        }
    }
}

fn anonymize_phase4_replay_knowledge(state: &mut RepositoryState, account_id: &str) {
    for response in state.phase4.request_results.values_mut() {
        let super::super::phase4::Phase4Response::Knowledge(response) = response else {
            continue;
        };
        for item in &mut response.knowledge.items {
            item.discovered_by
                .retain(|discovered_by| discovered_by != account_id);
        }
    }
}

fn anonymize_phase4_replay_skill_lessons(state: &mut RepositoryState, account_id: &str) {
    for response in state.phase4.request_results.values_mut() {
        let super::super::phase4::Phase4Response::Skill(response) = response else {
            continue;
        };
        let lesson_was_removed = response.lesson.as_ref().is_some_and(|lesson| {
            lesson.teacher_account_id == account_id || lesson.learner_account_id == account_id
        });
        response.skills.lessons.retain(|lesson| {
            lesson.teacher_account_id != account_id && lesson.learner_account_id != account_id
        });
        if lesson_was_removed {
            response.lesson = None;
            response.target_account_id = None;
            response.message =
                "This school lesson is no longer available after an account departure.".to_owned();
        } else if response.target_account_id.as_deref() == Some(account_id) {
            response.target_account_id = None;
        }
    }
}

fn invalidate_deleted_trade_replays(state: &mut RepositoryState, account_id: &str) {
    for identity in state.identities.values_mut() {
        for response in identity.trade_results.values_mut() {
            let Some(trade) = response.trade.as_ref() else {
                continue;
            };
            if trade.creator_account_id == account_id || trade.recipient_account_id == account_id {
                response.accepted = false;
                response.trade = None;
                response.reason = Some(
                    "That trade is no longer available after an account departure.".to_owned(),
                );
            }
        }
    }
}

fn anonymize_phase3_replay_frontier(state: &mut RepositoryState, account_id: &str) {
    for response in state.phase3.request_results.values_mut() {
        match response {
            super::super::phase3::Phase3Response::Claim(response) => {
                if let Some(claim) = response.claim.as_mut() {
                    anonymize_phase3_replay_claim(claim, account_id);
                }
            }
            super::super::phase3::Phase3Response::Expedition(response) => {
                if let Some(expedition) = response.expedition.as_mut() {
                    anonymize_expedition(expedition, account_id);
                }
            }
            super::super::phase3::Phase3Response::Contract(_)
            | super::super::phase3::Phase3Response::Combat(_)
            | super::super::phase3::Phase3Response::Recovery(_) => {}
        }
    }
}

fn anonymize_phase3_replay_claim(claim: &mut LandClaim, account_id: &str) {
    if claim.owner_account_id == account_id {
        claim.owner_account_id = DELETED_ACCOUNT.to_owned();
        claim.owner_name = DELETED_NAME.to_owned();
        claim.status = ClaimStatus::Abandoned;
    }
}

fn anonymize_audit_targets(
    audits: &mut std::collections::VecDeque<tarrowyn_protocol::AuditRecord>,
    account_id: &str,
    display_name: &str,
) {
    let prefix = format!("{account_id} (");
    for audit in audits {
        if let Some(suffix) = audit.target.strip_prefix(&prefix) {
            audit.target = format!("{DELETED_ACCOUNT} ({suffix}");
        }
        audit.note = audit.note.replace(account_id, DELETED_ACCOUNT);
        if !display_name.is_empty() {
            audit.note = audit.note.replace(display_name, DELETED_NAME);
        }
        audit.note = audit.note.chars().take(240).collect();
    }
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
    anonymize_governance(&mut state.phase4.governance, &request.account_id);
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
