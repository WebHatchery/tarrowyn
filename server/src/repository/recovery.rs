use super::phase3::{cache_key, record, Phase3Response};
use super::{
    authenticate, expire_sessions, meta, player_projection, presence, push_event,
    record_command_outcome, RepositoryError, WorldEvent, WorldRepository,
};
use tarrowyn_protocol::{ApiResponse, Position, RecoveryChoice, RecoveryRequest, RecoveryResponse};

impl WorldRepository {
    pub fn recovery(
        &self,
        token: &str,
        request: RecoveryRequest,
    ) -> Result<ApiResponse<RecoveryResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
        super::validate_request_id(&request.request_id)?;
        let cache_key = cache_key(&key, &request.request_id);
        if let Some(Phase3Response::Recovery(response)) =
            state.phase3.request_results.get(&cache_key)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let mut response = RecoveryResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            choice: request.choice,
            player: player_projection(&state, &key),
            consequence: String::new(),
            reason: None,
        };
        let is_knocked_out = state
            .identities
            .get(&key)
            .expect("identity exists")
            .knocked_out;
        if !is_knocked_out {
            response.reason = Some("There is no knockout to recover from.".to_owned());
        } else {
            let recovery_available = {
                let identity = state.identities.get(&key).expect("identity exists");
                match request.choice {
                    RecoveryChoice::SelfRecover => identity.inventory.seeds > 0,
                    RecoveryChoice::AskRescuer => true,
                    RecoveryChoice::PayHealer => identity.gold >= identity.recovery_cost,
                }
            };
            if !recovery_available {
                response.reason = Some(match request.choice {
                    RecoveryChoice::SelfRecover => {
                        "Self-recovery requires one carried seed; choose Rescuer or Healer."
                            .to_owned()
                    }
                    RecoveryChoice::PayHealer => {
                        "The healer requires the recovery cost shown in your ledger.".to_owned()
                    }
                    RecoveryChoice::AskRescuer => {
                        unreachable!("rescuer recovery is always available")
                    }
                });
            } else {
                let identity = state.identities.get_mut(&key).expect("identity exists");
                identity.knocked_out = false;
                identity.position = Position { x: 8, y: 5 };
                match request.choice {
                    RecoveryChoice::SelfRecover => {
                        identity.injuries = identity.injuries.saturating_sub(1);
                        identity.inventory.seeds -= 1;
                        response.consequence =
                            "You recover alone; one carried seed is spent on the journey back."
                                .to_owned();
                    }
                    RecoveryChoice::AskRescuer => {
                        identity.reputation = identity.reputation.saturating_add(1);
                        identity.injuries = identity.injuries.saturating_sub(1);
                        response.consequence = "A Hearth rescuer brings you home; the settlement remembers the kindness."
                            .to_owned();
                    }
                    RecoveryChoice::PayHealer => {
                        identity.gold -= identity.recovery_cost;
                        identity.injuries = 0;
                        response.consequence =
                            "The Hearth healer closes the injury for the listed cost.".to_owned();
                    }
                }
            }
            response.accepted = response.reason.is_none();
            if response.accepted {
                state
                    .identities
                    .get_mut(&key)
                    .expect("identity exists")
                    .recovery_cost = 0;
                record(
                    &mut state,
                    "recovery",
                    "A knocked-out traveller returns to the Hearth",
                    &response.consequence,
                );
                let presence_event = {
                    let identity = state.identities.get(&key).expect("identity exists");
                    WorldEvent::Presence(presence(identity, state.tick, true))
                };
                push_event(&mut state, presence_event);
            }
        }
        if response.accepted {
            let recovery_tick = state.tick;
            if let Some(local) = state.phase4.combat.get_mut(&key) {
                local.status = tarrowyn_protocol::LocalCombatStatus::Ready;
                local.enemy_health = 3;
                local.player_health = 2;
                local.turn = 0;
                local.action_available_at_tick = recovery_tick;
            }
        }
        response.player = player_projection(&state, &key);
        state
            .phase3
            .request_results
            .insert(cache_key, Phase3Response::Recovery(response.clone()));
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }
}
