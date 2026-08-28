use super::super::models::RepositoryState;
use tarrowyn_protocol::{FrontierEvent, TradeOffer, WorldEvent};

pub(super) fn migrate_guest_account_references(
    state: &mut RepositoryState,
    old_account_id: &str,
    new_account_id: &str,
    old_display_name: &str,
    new_display_name: &str,
) {
    if old_account_id == new_account_id {
        return;
    }

    for trade in state.trades.values_mut() {
        migrate_trade(trade, old_account_id, new_account_id, new_display_name);
    }
    migrate_phase3(
        state,
        old_account_id,
        new_account_id,
        old_display_name,
        new_display_name,
    );
    migrate_phase4(state, old_account_id, new_account_id, new_display_name);
    for order in &mut state.phase5.market_orders {
        if replace_id(&mut order.owner_account_id, old_account_id, new_account_id) {
            order.owner_name = new_display_name.to_owned();
        }
    }
    for message in &mut state.chat_history {
        migrate_chat(message, old_account_id, new_account_id, new_display_name);
    }
    for event in &mut state.events {
        migrate_event(
            &mut event.event,
            old_account_id,
            new_account_id,
            old_display_name,
            new_display_name,
        );
    }
    for audit in &mut state.phase6.audits {
        replace_id(&mut audit.actor_account_id, old_account_id, new_account_id);
        replace_id(&mut audit.target, old_account_id, new_account_id);
    }
}

fn migrate_phase3(
    state: &mut RepositoryState,
    old_account_id: &str,
    new_account_id: &str,
    old_display_name: &str,
    new_display_name: &str,
) {
    if let Some(claim) = state.phase3.claim.as_mut() {
        if replace_id(&mut claim.owner_account_id, old_account_id, new_account_id) {
            claim.owner_name = new_display_name.to_owned();
        }
    }
    if let Some(expedition) = state.phase3.expedition.as_mut() {
        migrate_expedition(expedition, old_account_id, new_account_id, new_display_name);
    }
    for entry in state
        .phase3
        .chronicle
        .iter_mut()
        .chain(state.phase3.chronicle_archive.iter_mut())
    {
        migrate_chronicle(entry, old_display_name, new_display_name);
    }
}

fn migrate_phase4(
    state: &mut RepositoryState,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    for claim in &mut state.phase4.claims {
        if claim.owner_account_id.as_deref() == Some(old_account_id) {
            claim.owner_account_id = Some(new_account_id.to_owned());
            claim.owner_name = Some(new_display_name.to_owned());
        }
        replace_option_id(&mut claim.approved_by, old_account_id, new_account_id);
    }
    for office in &mut state.phase4.governance.offices {
        if office.holder_account_id.as_deref() == Some(old_account_id) {
            office.holder_account_id = Some(new_account_id.to_owned());
            office.holder_name = Some(new_display_name.to_owned());
        }
    }
    for proposal in &mut state.phase4.governance.proposals {
        if replace_id(
            &mut proposal.proposer_account_id,
            old_account_id,
            new_account_id,
        ) {
            proposal.proposer_name = new_display_name.to_owned();
        }
        replace_option_id(&mut proposal.approved_by, old_account_id, new_account_id);
    }
    for decision in &mut state.phase4.governance.decisions {
        if replace_id(
            &mut decision.actor_account_id,
            old_account_id,
            new_account_id,
        ) {
            decision.actor_name = new_display_name.to_owned();
        }
    }
    for receipt in &mut state.phase4.governance.tax_ledger {
        if replace_id(
            &mut receipt.payer_account_id,
            old_account_id,
            new_account_id,
        ) {
            receipt.payer_name = new_display_name.to_owned();
        }
    }
    if let Some(policy) = state.phase4.governance.taxation.as_mut() {
        replace_id(&mut policy.payer, old_account_id, new_account_id);
        replace_id(&mut policy.recipient, old_account_id, new_account_id);
    }
    for order in &mut state.phase4.orders {
        if replace_id(
            &mut order.requester_account_id,
            old_account_id,
            new_account_id,
        ) {
            order.requester_name = new_display_name.to_owned();
        }
        if order.provider_account_id.as_deref() == Some(old_account_id) {
            order.provider_account_id = Some(new_account_id.to_owned());
            order.provider_name = Some(new_display_name.to_owned());
        }
    }
    for lesson in &mut state.phase4.lessons {
        if replace_id(
            &mut lesson.teacher_account_id,
            old_account_id,
            new_account_id,
        ) {
            lesson.teacher_name = new_display_name.to_owned();
        }
        if replace_id(
            &mut lesson.learner_account_id,
            old_account_id,
            new_account_id,
        ) {
            lesson.learner_name = new_display_name.to_owned();
        }
    }
    for item in &mut state.phase4.knowledge {
        for account_id in &mut item.discovered_by {
            replace_id(account_id, old_account_id, new_account_id);
        }
    }
}

fn migrate_event(
    event: &mut WorldEvent,
    old_account_id: &str,
    new_account_id: &str,
    old_display_name: &str,
    new_display_name: &str,
) {
    match event {
        WorldEvent::Presence(presence) => {
            if replace_id(&mut presence.account_id, old_account_id, new_account_id) {
                presence.display_name = new_display_name.to_owned();
            }
        }
        WorldEvent::Chat(message) => {
            migrate_chat(message, old_account_id, new_account_id, new_display_name)
        }
        WorldEvent::Trade(trade) => {
            migrate_trade(trade, old_account_id, new_account_id, new_display_name)
        }
        WorldEvent::Frontier(frontier) => {
            migrate_frontier(frontier, old_account_id, new_account_id, new_display_name)
        }
        WorldEvent::Chronicle(entry) => {
            migrate_chronicle(entry, old_display_name, new_display_name)
        }
        WorldEvent::Clock(_) | WorldEvent::Farming(_) | WorldEvent::TavernNotice(_) => {}
    }
}

fn migrate_frontier(
    event: &mut FrontierEvent,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    match event {
        FrontierEvent::Claim(claim) => {
            if replace_id(&mut claim.owner_account_id, old_account_id, new_account_id) {
                claim.owner_name = new_display_name.to_owned();
            }
        }
        FrontierEvent::Expedition(expedition) => {
            migrate_expedition(expedition, old_account_id, new_account_id, new_display_name)
        }
        FrontierEvent::Threat(_) | FrontierEvent::Opportunity(_) => {}
    }
}

fn migrate_expedition(
    expedition: &mut tarrowyn_protocol::Expedition,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    replace_id(
        &mut expedition.leader_account_id,
        old_account_id,
        new_account_id,
    );
    for member in &mut expedition.members {
        if replace_id(&mut member.account_id, old_account_id, new_account_id) {
            member.display_name = new_display_name.to_owned();
        }
    }
}

fn migrate_trade(
    trade: &mut TradeOffer,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    if replace_id(
        &mut trade.creator_account_id,
        old_account_id,
        new_account_id,
    ) {
        trade.creator_name = new_display_name.to_owned();
    }
    if replace_id(
        &mut trade.recipient_account_id,
        old_account_id,
        new_account_id,
    ) {
        trade.recipient_name = new_display_name.to_owned();
    }
}

fn migrate_chat(
    message: &mut tarrowyn_protocol::ChatMessage,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    if replace_id(&mut message.account_id, old_account_id, new_account_id) {
        message.display_name = new_display_name.to_owned();
    }
}

fn migrate_chronicle(
    entry: &mut tarrowyn_protocol::ChronicleEntry,
    old_display_name: &str,
    new_display_name: &str,
) {
    if !old_display_name.is_empty() && old_display_name != new_display_name {
        entry.title = entry.title.replace(old_display_name, new_display_name);
        entry.text = entry.text.replace(old_display_name, new_display_name);
    }
}

fn replace_option_id(value: &mut Option<String>, old_account_id: &str, new_account_id: &str) {
    if value.as_deref() == Some(old_account_id) {
        *value = Some(new_account_id.to_owned());
    }
}

fn replace_id(value: &mut String, old_account_id: &str, new_account_id: &str) -> bool {
    if value == old_account_id {
        *value = new_account_id.to_owned();
        true
    } else {
        false
    }
}
