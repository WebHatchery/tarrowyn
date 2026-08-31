use super::super::models::RepositoryState;
use tarrowyn_protocol::{
    ClaimRecord, FrontierEvent, GovernanceState, KnowledgeItem, ServiceOrder, SkillLesson,
    TradeOffer, WorldEvent,
};

mod endpoints;

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

    migrate_identity_replay_caches(state, old_account_id, new_account_id, new_display_name);
    migrate_phase4_replay_caches(state, old_account_id, new_account_id, new_display_name);
    migrate_phase6_replay_caches(state, old_account_id, new_account_id);
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
    for settlement in &mut state.phase5.settlements {
        for entry in &mut settlement.chronicle {
            migrate_chronicle(entry, old_display_name, new_display_name);
        }
    }
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
        migrate_audit_target(&mut audit.target, old_account_id, new_account_id);
        migrate_audit_note(
            &mut audit.note,
            old_account_id,
            new_account_id,
            old_display_name,
            new_display_name,
        );
    }
}

fn migrate_phase6_replay_caches(
    state: &mut RepositoryState,
    old_account_id: &str,
    new_account_id: &str,
) {
    let mut migrated = std::collections::HashMap::new();
    for (key, response) in std::mem::take(&mut state.phase6.request_results) {
        if super::is_support_replay_key_for_account(&key, old_account_id, &response) {
            migrated.insert(
                format!("repair:{new_account_id}:{}", response.request_id),
                response,
            );
        } else {
            migrated.insert(key, response);
        }
    }
    state.phase6.request_results = migrated;
}

fn migrate_phase4_replay_caches(
    state: &mut RepositoryState,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    let mut migrated = std::collections::HashMap::new();
    for (key, mut response) in std::mem::take(&mut state.phase4.request_results) {
        let replacement =
            super::super::phase4::replay_prefix_for_account(&key, old_account_id, &response).map(
                |prefix| {
                    format!(
                        "{prefix}{new_account_id}:{}",
                        super::super::phase4::replay_request_id(&response)
                    )
                },
            );
        if let Some(replacement) = replacement {
            migrate_phase4_response(
                &mut response,
                old_account_id,
                new_account_id,
                new_display_name,
            );
            migrated.insert(replacement, response);
        } else {
            migrated.insert(key, response);
        }
    }
    state.phase4.request_results = migrated;
}

fn migrate_phase4_response(
    response: &mut super::super::phase4::Phase4Response,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    match response {
        super::super::phase4::Phase4Response::Governance(response) => migrate_governance(
            &mut response.governance,
            old_account_id,
            new_account_id,
            new_display_name,
        ),
        super::super::phase4::Phase4Response::Claim(response) => {
            if let Some(claim) = response.claim.as_mut() {
                migrate_claim(claim, old_account_id, new_account_id, new_display_name);
            }
            for claim in &mut response.claims.claims {
                migrate_claim(claim, old_account_id, new_account_id, new_display_name);
            }
        }
        super::super::phase4::Phase4Response::Profession(response) => {
            if let Some(order) = response.order.as_mut() {
                migrate_service_order(order, old_account_id, new_account_id, new_display_name);
            }
            for order in &mut response.professions.orders {
                migrate_service_order(order, old_account_id, new_account_id, new_display_name);
            }
        }
        super::super::phase4::Phase4Response::Knowledge(response) => {
            for item in &mut response.knowledge.items {
                migrate_knowledge_item(item, old_account_id, new_account_id);
            }
        }
        super::super::phase4::Phase4Response::Combat(response) => migrate_player(
            &mut response.player,
            old_account_id,
            new_account_id,
            new_display_name,
        ),
        super::super::phase4::Phase4Response::Skill(response) => {
            if let Some(lesson) = response.lesson.as_mut() {
                migrate_lesson(lesson, old_account_id, new_account_id, new_display_name);
            }
            for lesson in &mut response.skills.lessons {
                migrate_lesson(lesson, old_account_id, new_account_id, new_display_name);
            }
        }
    }
}

fn migrate_identity_replay_caches(
    state: &mut RepositoryState,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    if let Some(identity) = state
        .identities
        .values_mut()
        .find(|identity| identity.account_id == old_account_id)
    {
        for response in identity.farming_results.values_mut() {
            migrate_player(
                &mut response.player,
                old_account_id,
                new_account_id,
                new_display_name,
            );
        }
        for response in identity.chat_results.values_mut() {
            if let Some(message) = response.message.as_mut() {
                migrate_chat(message, old_account_id, new_account_id, new_display_name);
            }
        }
    }
    for identity in state.identities.values_mut() {
        for response in identity.trade_results.values_mut() {
            if let Some(trade) = response.trade.as_mut() {
                migrate_trade(trade, old_account_id, new_account_id, new_display_name);
            }
        }
    }
    for response in state.phase3.request_results.values_mut() {
        match response {
            super::super::phase3::Phase3Response::Contract(response) => migrate_player(
                &mut response.player,
                old_account_id,
                new_account_id,
                new_display_name,
            ),
            super::super::phase3::Phase3Response::Combat(response) => migrate_player(
                &mut response.player,
                old_account_id,
                new_account_id,
                new_display_name,
            ),
            super::super::phase3::Phase3Response::Recovery(response) => migrate_player(
                &mut response.player,
                old_account_id,
                new_account_id,
                new_display_name,
            ),
            super::super::phase3::Phase3Response::Claim(response) => {
                if let Some(claim) = response.claim.as_mut() {
                    if replace_id(&mut claim.owner_account_id, old_account_id, new_account_id) {
                        claim.owner_name = new_display_name.to_owned();
                    }
                }
            }
            super::super::phase3::Phase3Response::Expedition(response) => {
                if let Some(expedition) = response.expedition.as_mut() {
                    migrate_expedition(
                        expedition,
                        old_account_id,
                        new_account_id,
                        new_display_name,
                    );
                }
            }
        }
    }
    for response in state.phase5.request_results.values_mut() {
        if let super::super::phase5::Phase5Response::Market(response) = response {
            if let Some(order) = response.order.as_mut() {
                if replace_id(&mut order.owner_account_id, old_account_id, new_account_id) {
                    order.owner_name = new_display_name.to_owned();
                }
            }
        }
    }
}

fn migrate_player(
    player: &mut tarrowyn_protocol::PlayerProjection,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    if replace_id(&mut player.account_id, old_account_id, new_account_id) {
        player.display_name = new_display_name.to_owned();
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
    for participant in &mut state.phase3.expedition_credentials {
        replace_id(participant, old_account_id, new_account_id);
    }
    state.phase3.expedition_credentials.sort_unstable();
    state.phase3.expedition_credentials.dedup();
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
        migrate_claim(claim, old_account_id, new_account_id, new_display_name);
    }
    migrate_governance(
        &mut state.phase4.governance,
        old_account_id,
        new_account_id,
        new_display_name,
    );
    for order in &mut state.phase4.orders {
        migrate_service_order(order, old_account_id, new_account_id, new_display_name);
    }
    for lesson in &mut state.phase4.lessons {
        migrate_lesson(lesson, old_account_id, new_account_id, new_display_name);
    }
    for item in &mut state.phase4.knowledge {
        migrate_knowledge_item(item, old_account_id, new_account_id);
    }
}

fn migrate_claim(
    claim: &mut ClaimRecord,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    if claim.owner_account_id.as_deref() == Some(old_account_id) {
        claim.owner_account_id = Some(new_account_id.to_owned());
        claim.owner_name = Some(new_display_name.to_owned());
    }
    replace_option_id(&mut claim.approved_by, old_account_id, new_account_id);
}

fn migrate_governance(
    governance: &mut GovernanceState,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    for office in &mut governance.offices {
        if office.holder_account_id.as_deref() == Some(old_account_id) {
            office.holder_account_id = Some(new_account_id.to_owned());
            office.holder_name = Some(new_display_name.to_owned());
        }
    }
    for proposal in &mut governance.proposals {
        if replace_id(
            &mut proposal.proposer_account_id,
            old_account_id,
            new_account_id,
        ) {
            proposal.proposer_name = new_display_name.to_owned();
        }
        replace_option_id(&mut proposal.approved_by, old_account_id, new_account_id);
    }
    for decision in &mut governance.decisions {
        if replace_id(
            &mut decision.actor_account_id,
            old_account_id,
            new_account_id,
        ) {
            decision.actor_name = new_display_name.to_owned();
        }
    }
    for receipt in &mut governance.tax_ledger {
        if replace_id(
            &mut receipt.payer_account_id,
            old_account_id,
            new_account_id,
        ) {
            receipt.payer_name = new_display_name.to_owned();
        }
    }
    if let Some(policy) = governance.taxation.as_mut() {
        replace_id(&mut policy.payer, old_account_id, new_account_id);
        replace_id(&mut policy.recipient, old_account_id, new_account_id);
    }
}

fn migrate_service_order(
    order: &mut ServiceOrder,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
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

fn migrate_knowledge_item(item: &mut KnowledgeItem, old_account_id: &str, new_account_id: &str) {
    for account_id in &mut item.discovered_by {
        replace_id(account_id, old_account_id, new_account_id);
    }
}

fn migrate_lesson(
    lesson: &mut SkillLesson,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
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

fn migrate_audit_target(target: &mut String, old_account_id: &str, new_account_id: &str) {
    if replace_id(target, old_account_id, new_account_id) {
        return;
    }
    let prefix = format!("{old_account_id} (");
    if let Some(suffix) = target.strip_prefix(&prefix) {
        *target = format!("{new_account_id} ({suffix}");
    }
}

fn migrate_audit_note(
    note: &mut String,
    old_account_id: &str,
    new_account_id: &str,
    old_display_name: &str,
    new_display_name: &str,
) {
    *note = note.replace(old_account_id, new_account_id);
    if !old_display_name.is_empty() && old_display_name != new_display_name {
        *note = note.replace(old_display_name, new_display_name);
    }
    *note = note.chars().take(240).collect();
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
