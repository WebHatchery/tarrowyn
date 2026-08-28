use super::models::RepositoryState;
use tarrowyn_protocol::{
    ClaimLifecycleStatus, ClaimStatus, ExpeditionStatus, ProposalStatus, ServiceOrderStatus,
};

const RESET_ACCOUNT: &str = "former-resident";
const RESET_NAME: &str = "Former resident";

pub(super) fn reset_guest(state: &mut RepositoryState, identity_key: &str) {
    let Some(old_account_id) = state
        .identities
        .get(identity_key)
        .map(|identity| identity.account_id.clone())
    else {
        return;
    };

    state
        .sessions
        .retain(|_, session| session.identity_key != identity_key);
    state.phase3.contracts.remove(identity_key);
    state
        .phase3
        .request_results
        .retain(|key, _| !key.starts_with(&format!("{identity_key}:")));
    state.phase4.profiles.remove(identity_key);
    state.phase4.materials.remove(identity_key);
    state.phase4.credentials.remove(identity_key);
    state.phase4.known_by.remove(identity_key);
    state.phase4.combat.remove(identity_key);
    state.phase4.lessons.retain(|lesson| {
        lesson.teacher_account_id != old_account_id && lesson.learner_account_id != old_account_id
    });
    state.phase4.request_results.retain(|key, _| {
        ![
            format!("phase4:{old_account_id}:"),
            format!("skill-practice:{old_account_id}:"),
            format!("skill-lesson-begin:{old_account_id}:"),
            format!("skill-lesson-complete:{old_account_id}:"),
        ]
        .iter()
        .any(|prefix| key.starts_with(prefix))
    });
    state.phase5.travel.remove(identity_key);
    state
        .phase5
        .request_results
        .retain(|key, _| !super::phase5::is_request_cache_for_identity(key, identity_key));
    state.trades.retain(|_, trade| {
        trade.creator_account_id != old_account_id && trade.recipient_account_id != old_account_id
    });
    super::phase5::close_deleted_account_orders(state, &old_account_id);
    reset_phase3_public_ownership(state, &old_account_id);
    reset_phase4_public_ownership(state, &old_account_id);
    state.phase6.audits.retain(|record| {
        record.actor_account_id != old_account_id && record.target != old_account_id
    });
    state
        .phase6
        .moderation_results
        .retain(|key, _| !key.contains(&format!(":{identity_key}:")));
    state
        .phase6
        .moderation_last_report_ticks
        .remove(identity_key);
    state
        .phase6
        .request_results
        .retain(|key, _| !key.starts_with(&format!("repair:{old_account_id}:")));
    state.identities.remove(identity_key);
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
