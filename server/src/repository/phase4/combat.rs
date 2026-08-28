use super::{account_id, cache_key, record, validate_request_id};
use tarrowyn_protocol::{
    ApiResponse, LocalCombatAction, LocalCombatRequest, LocalCombatResponse, LocalCombatState,
    LocalCombatStatus, WeaponKind,
};

impl super::super::WorldRepository {
    pub fn combat_status(
        &self,
        token: &str,
    ) -> Result<ApiResponse<LocalCombatState>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        super::super::expire_sessions(&mut state, &self.config);
        let key = super::super::authenticate(&mut state, token, &self.config)?;
        let combat = state
            .phase4
            .combat
            .entry(key)
            .or_insert_with(default_combat)
            .clone();
        Ok(ApiResponse {
            meta: super::super::meta(state.tick, None, Some(state.cursor)),
            data: combat,
        })
    }

    pub fn local_combat(
        &self,
        token: &str,
        request: LocalCombatRequest,
    ) -> Result<ApiResponse<LocalCombatResponse>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        super::super::expire_sessions(&mut state, &self.config);
        let key = super::super::authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let actor_id = account_id(&state, &key);
        let cache = cache_key(&actor_id, &request.request_id);
        if let Some(super::Phase4Response::Combat(response)) =
            state.phase4.request_results.get(&cache)
        {
            return Ok(ApiResponse {
                meta: super::super::meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let mut combat = state
            .phase4
            .combat
            .get(&key)
            .cloned()
            .unwrap_or_else(default_combat);
        let mut response = LocalCombatResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            combat: combat.clone(),
            player: super::super::player_projection(&state, &key),
            prompt: "Tap PREPARE before choosing a weapon; stored property is safe.".to_owned(),
            reason: None,
        };
        let position = state
            .identities
            .get(&key)
            .expect("identity exists")
            .position;
        let zone_position = state.phase3.zone.position;
        match request.action {
            LocalCombatAction::Prepare => {
                if position.manhattan_distance(zone_position) > 2 {
                    response.reason = Some(
                        "Stand near Whisperwood Edge before preparing for the local encounter."
                            .to_owned(),
                    );
                } else if matches!(
                    combat.status,
                    LocalCombatStatus::Victorious | LocalCombatStatus::Retreated
                ) {
                    combat = default_combat();
                    combat.status = LocalCombatStatus::Engaged;
                    combat.weapon = request.weapon;
                    response.accepted = true;
                    response.prompt = format!(
                        "The encounter is ready with the {}. Tap TECHNIQUE, STRIKE, GUARD, or RETREAT.",
                        request.weapon.label()
                    );
                } else {
                    combat.status = LocalCombatStatus::Engaged;
                    combat.weapon = request.weapon;
                    response.accepted = true;
                    response.prompt = format!(
                        "The local threat is watching your {}. Tap TECHNIQUE, STRIKE, GUARD, or RETREAT.",
                        request.weapon.label()
                    );
                }
            }
            LocalCombatAction::Retreat => {
                if combat.status == LocalCombatStatus::Engaged {
                    combat.status = LocalCombatStatus::Retreated;
                    response.accepted = true;
                    response.prompt =
                        "You retreated; no stored property was touched. Prepare again when ready."
                            .to_owned();
                    record(
                        &mut state,
                        "combat retreat",
                        "A traveller chooses a reversible retreat",
                        "The local encounter ended without deleting property or progression.",
                    );
                } else {
                    response.reason =
                        Some("There is no active local encounter to retreat from.".to_owned());
                }
            }
            LocalCombatAction::Guard => {
                if combat.status != LocalCombatStatus::Engaged {
                    response.reason =
                        Some("Prepare the local encounter before guarding.".to_owned());
                } else {
                    combat.turn = combat.turn.saturating_add(1);
                    combat.player_health = combat.player_health.saturating_sub(0);
                    response.accepted = true;
                    response.prompt =
                        "Your guard holds. The threat has not advanced; choose STRIKE or RETREAT."
                            .to_owned();
                }
            }
            LocalCombatAction::UseItem => {
                if combat.status != LocalCombatStatus::Engaged {
                    response.reason =
                        Some("Prepare the local encounter before using a bandage.".to_owned());
                } else if combat.player_health >= 2 {
                    response.reason = Some("A bandage is only needed after an injury.".to_owned());
                } else {
                    let identity = state.identities.get_mut(&key).expect("identity exists");
                    if identity.inventory.bandages == 0 {
                        response.reason = Some(
                            "The bandage pouch is empty; commission or trade for another."
                                .to_owned(),
                        );
                    } else {
                        identity.inventory.bandages -= 1;
                        combat.player_health = combat.player_health.saturating_add(1).min(2);
                        combat.turn = combat.turn.saturating_add(1);
                        response.accepted = true;
                        response.prompt =
                            "The bandage closes the injury. Choose STRIKE, GUARD, or RETREAT."
                                .to_owned();
                    }
                }
            }
            LocalCombatAction::Strike | LocalCombatAction::Technique => {
                if combat.status != LocalCombatStatus::Engaged {
                    response.reason =
                        Some("Prepare the local encounter before striking.".to_owned());
                } else if request.action == LocalCombatAction::Technique && combat.turn > 0 {
                    response.reason = Some(
                        "The opening for that weapon technique has passed; choose STRIKE or GUARD."
                            .to_owned(),
                    );
                } else {
                    combat.weapon = request.weapon;
                    combat.turn = combat.turn.saturating_add(1);
                    let damage = if request.action == LocalCombatAction::Technique {
                        technique_damage(request.weapon)
                    } else {
                        request.weapon.damage().clamp(1, 2)
                    };
                    combat.enemy_health = combat.enemy_health.saturating_sub(damage);
                    if combat.enemy_health == 0 {
                        combat.status = LocalCombatStatus::Victorious;
                        response.accepted = true;
                        response.prompt =
                            "Victory. The road is safer; stored property remained safe.".to_owned();
                        let identity = state.identities.get_mut(&key).expect("identity exists");
                        identity.gold = identity.gold.saturating_add(3);
                        identity.skill = identity.skill.saturating_add(1);
                        super::super::skills::record_practice(
                            &mut state,
                            &key,
                            request.weapon.skill_id(),
                        );
                        super::super::skills::record_weapon_defeat(
                            &mut state,
                            &key,
                            request.weapon,
                        );
                        record(
                            &mut state,
                            "combat victory",
                            "A local threat yields to a readable choice",
                            &format!(
                                "The {} ended the encounter after {} turns.",
                                request.weapon.label(),
                                combat.turn
                            ),
                        );
                    } else if matches!(
                        request.weapon,
                        WeaponKind::ImprovisedClub | WeaponKind::Shield
                    ) || combat.turn > 2
                    {
                        combat.player_health = combat.player_health.saturating_sub(1);
                        if combat.player_health == 0 {
                            combat.status = LocalCombatStatus::KnockedOut;
                            let identity = state.identities.get_mut(&key).expect("identity exists");
                            identity.knocked_out = true;
                            identity.injuries = identity.injuries.saturating_add(1).min(3);
                            identity.recovery_cost = 4;
                            identity.position = tarrowyn_protocol::Position { x: 8, y: 5 };
                            if identity.inventory.seeds > 0 {
                                identity.inventory.seeds -= 1;
                            }
                            response.accepted = true;
                            response.prompt = "Knockout. Choose the visible recovery action; carried risk is one seed at most and stored property is safe.".to_owned();
                            record(&mut state, "combat knockout", "A bounded local defeat sends a traveller home", "The improvised weapon could not finish the encounter; only the displayed carried risk applies.");
                        } else {
                            response.accepted = true;
                            response.prompt = "The threat answered. Your bounded injury is visible; STRIKE again or RETREAT.".to_owned();
                        }
                    } else if request.action == LocalCombatAction::Technique {
                        response.accepted = true;
                        response.prompt = format!(
                            "Your {} technique opens the threat's guard. Choose STRIKE, GUARD, or RETREAT.",
                            request.weapon.label()
                        );
                    } else {
                        response.accepted = true;
                        response.prompt = format!(
                            "Your {} lands cleanly. The threat is still standing; choose STRIKE or RETREAT.",
                            request.weapon.label()
                        );
                    }
                }
            }
        }
        state.phase4.combat.insert(key.clone(), combat.clone());
        response.combat = combat;
        response.player = super::super::player_projection(&state, &key);
        state
            .phase4
            .request_results
            .insert(cache, super::Phase4Response::Combat(response.clone()));
        self.persist(&state);
        Ok(ApiResponse {
            meta: super::super::meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }
}

fn default_combat() -> LocalCombatState {
    LocalCombatState {
        encounter_id: "whisperwood-local-1".to_owned(),
        enemy_name: "Brambleback scout".to_owned(),
        enemy_health: 3,
        player_health: 2,
        turn: 0,
        status: LocalCombatStatus::Ready,
        weapon: WeaponKind::ImprovisedClub,
        injury_limit: 3,
        stored_property_safe: true,
        carried_risk: "At most one carried seed is risked on knockout; the choice is shown first."
            .to_owned(),
        recovery_cost: 4,
    }
}

fn technique_damage(weapon: WeaponKind) -> u8 {
    match weapon {
        WeaponKind::Spear | WeaponKind::Axe => 3,
        WeaponKind::IronSword | WeaponKind::Bow => 2,
        WeaponKind::Shield | WeaponKind::ImprovisedClub => 1,
    }
}
