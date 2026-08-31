use super::super::super::{
    authenticate, meta, player_projection, record_command_outcome, validate_bounded_text,
    validate_request_id, RepositoryError, WorldRepository,
};
use super::super::{deletion, MAX_PENDING_DELETIONS, PRIVACY_POLICY_VERSION};
use tarrowyn_protocol::{
    AccountDeletionRequest, AccountDeletionResponse, AccountResponse, ApiResponse,
};

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
        let pending = super::super::deletion::PendingAccountDeletion {
            request_id: request.request_id.clone(),
            account_id: account.clone(),
            identity_key: key,
            character_id: character_id.clone(),
            replay_key: deletion_replay_key.clone(),
        };
        state.phase6.deletion_requests.insert(cache_key, pending);
        super::super::audit(
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
