use super::super::models::RepositoryState;
use super::super::{
    authenticate, meta, player_projection, record_command_outcome, validate_bounded_text,
    validate_request_id, RepositoryError, WorldRepository,
};
use super::{deletion, MAX_PENDING_DELETIONS, PRIVACY_POLICY_VERSION};
use tarrowyn_protocol::{
    AccountDeletionRequest, AccountDeletionResponse, AccountResponse, ApiResponse, ClaimRecord,
    FrontierEvent, GovernanceState, KnowledgeItem, ServiceOrder, SkillLesson, TradeOffer,
    WorldEvent,
};

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
        replace_id(&mut audit.target, old_account_id, new_account_id);
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
        for response in identity.trade_results.values_mut() {
            if let Some(trade) = response.trade.as_mut() {
                migrate_trade(trade, old_account_id, new_account_id, new_display_name);
            }
        }
        for response in identity.chat_results.values_mut() {
            if let Some(message) = response.message.as_mut() {
                migrate_chat(message, old_account_id, new_account_id, new_display_name);
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

impl WorldRepository {
    pub fn account(&self, token: &str) -> Result<ApiResponse<AccountResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        let identity = state.identities.get(&key).expect("identity exists");
        let production = state.phase6.accounts.get(&identity.account_id);
        let expires = state
            .phase6
            .sessions
            .get(token)
            .map(|session| session.expires_at_tick)
            .unwrap_or_else(|| state.tick.saturating_add(self.config.session_ttl_ticks()));
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: AccountResponse {
                account_id: identity.account_id.clone(),
                provider: production
                    .map(|account| account.provider.clone())
                    .unwrap_or_else(|| "development-guest".to_owned()),
                character_id: identity.character_id.clone(),
                display_name: identity.display_name.clone(),
                guest_fixture: production.is_none(),
                privacy_policy_version: PRIVACY_POLICY_VERSION.to_owned(),
                retention_note: "Account identity is retained until deletion; chat reports are retained for 90 days; settlement history is retained as public world history with account identifiers minimised.".to_owned(),
                session_expires_at_tick: expires,
                character: player_projection(&state, &key),
            },
        })
    }

    pub fn account_delete(
        &self,
        token: &str,
        request: AccountDeletionRequest,
    ) -> Result<ApiResponse<AccountDeletionResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        validate_request_id(&request.request_id)?;
        let requested_account_id = validate_bounded_text(
            &request.account_id,
            160,
            "invalid_account_id",
            "The account ID to delete must be bounded and contain no control characters.",
        )?;
        let deletion_replay_key = deletion::replay_key(token, &request.request_id);
        if let Some(previous) = state
            .phase6
            .deletion_results
            .get(&deletion_replay_key)
            .filter(|response| response.account_id == requested_account_id)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous.clone(),
            });
        }
        let key = authenticate(&mut state, token, &self.config)?;
        let account = state
            .identities
            .get(&key)
            .expect("identity exists")
            .account_id
            .clone();
        if account != requested_account_id {
            return Err(RepositoryError::new(
                403,
                "account_boundary_violation",
                "An account may delete only its own character boundary.",
            ));
        }
        if !state.phase6.accounts.contains_key(&account) {
            return Err(RepositoryError::new(
                409,
                "guest_account_deletion_not_supported",
                "Link this development guest to the identity gateway before requesting deletion.",
            ));
        }
        let cache_key = format!("delete:{account}:{}", request.request_id);
        if let Some(pending) = state.phase6.deletion_requests.get(&cache_key) {
            let mut response = deletion::scheduled_response(pending);
            response.request_id = request.request_id.clone();
            state
                .phase6
                .deletion_results
                .insert(deletion_replay_key, response.clone());
            self.persist(&state);
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response,
            });
        }
        if let Some(pending) = state
            .phase6
            .deletion_requests
            .values()
            .find(|pending| pending.account_id == account)
            .cloned()
        {
            let mut response = deletion::scheduled_response(&pending);
            response.request_id = request.request_id;
            state
                .phase6
                .deletion_results
                .insert(deletion_replay_key, response.clone());
            self.persist(&state);
            return Ok(ApiResponse {
                meta: meta(
                    state.tick,
                    Some(response.request_id.clone()),
                    Some(state.cursor),
                ),
                data: response,
            });
        }
        let character_id = state
            .identities
            .get(&key)
            .expect("identity exists")
            .character_id
            .clone();
        if state.phase6.deletion_requests.len() >= MAX_PENDING_DELETIONS {
            let response = AccountDeletionResponse {
                request_id: request.request_id,
                account_id: account,
                character_id,
                accepted: false,
                status: "blocked".to_owned(),
                reason: Some(
                    "The account-deletion queue is full; wait for the next authoritative tick before trying again."
                        .to_owned(),
                ),
            };
            record_command_outcome(&mut state, false);
            self.persist(&state);
            return Ok(ApiResponse {
                meta: meta(
                    state.tick,
                    Some(response.request_id.clone()),
                    Some(state.cursor),
                ),
                data: response,
            });
        }
        let pending = super::deletion::PendingAccountDeletion {
            request_id: request.request_id.clone(),
            account_id: account.clone(),
            identity_key: key,
            character_id: character_id.clone(),
            replay_key: deletion_replay_key.clone(),
        };
        state.phase6.deletion_requests.insert(cache_key, pending);
        super::audit(
            &mut state,
            &account,
            "account.delete.requested",
            &account,
            "accepted",
            "Account deletion was queued for the next authoritative tick.",
        );
        let response = AccountDeletionResponse {
            request_id: request.request_id.clone(),
            account_id: account,
            character_id,
            accepted: true,
            status: "scheduled".to_owned(),
            reason: None,
        };
        state
            .phase6
            .deletion_results
            .insert(deletion_replay_key, response.clone());
        record_command_outcome(&mut state, true);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }
}
