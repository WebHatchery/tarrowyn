use super::{
    account_id, account_name, cache_key, default_capability, record, validate_optional_identifier,
    validate_request_id,
};
use tarrowyn_protocol::{
    ApiResponse, MaterialStock, ProfessionAction, ProfessionKind, ProfessionProfile,
    ProfessionRequest, ProfessionResponse, ProfessionsResponse, ServiceOrder, ServiceOrderStatus,
};

impl super::super::WorldRepository {
    pub fn professions(
        &self,
        token: &str,
    ) -> Result<ApiResponse<ProfessionsResponse>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let key = super::super::authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: super::super::meta(state.tick, None, Some(state.cursor)),
            data: view(&state, &key),
        })
    }

    pub fn profession_order(
        &self,
        token: &str,
        request: ProfessionRequest,
    ) -> Result<ApiResponse<ProfessionResponse>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let key = super::super::authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let order_id = validate_optional_identifier(
            request.order_id.as_deref(),
            "invalid_order_id",
            "A service-order selector must be bounded and contain no control characters.",
        )?;
        let _capability_id = validate_optional_identifier(
            request.capability_id.as_deref(),
            "invalid_capability_id",
            "A capability selector must be bounded and contain no control characters.",
        )?;
        ensure_player(&mut state, &key);
        let actor_id = account_id(&state, &key);
        let cache = cache_key(&actor_id, &request.request_id);
        if let Some(super::Phase4Response::Profession(response)) =
            state.phase4.request_results.get(&cache)
        {
            return Ok(ApiResponse {
                meta: super::super::meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let mut response = ProfessionResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            professions: view(&state, &key),
            order: None,
            reason: None,
        };
        let action = request.action;
        match action {
            ProfessionAction::Inspect => response.accepted = true,
            ProfessionAction::LearnCapability => {
                let profession = request.profession.unwrap_or(ProfessionKind::Carpenter);
                let actor_name = account_name(&state, &key);
                let profiles = state.phase4.profiles.get_mut(&key).expect("profile exists");
                if profiles
                    .iter()
                    .any(|profile| profile.profession == profession)
                {
                    response.reason =
                        Some("That professional capability is already recorded.".to_owned());
                } else {
                    profiles.push(ProfessionProfile {
                        profession,
                        level: 1,
                        reputation: 0,
                        credential: Some(
                            format!("settlement-credential-{:?}", profession).to_lowercase(),
                        ),
                        capabilities: vec![default_capability(profession)],
                    });
                    let credentials = state.phase4.credentials.entry(key.clone()).or_default();
                    remember_credential(
                        credentials,
                        format!("{:?} apprentice credential", profession),
                    );
                    response.accepted = true;
                    record(
                        &mut state,
                        "capability learned",
                        "A craft is written into a player's hands",
                        &format!("{} learned the {:?} capability; a new character can now enter that order.", actor_name, profession),
                    );
                }
            }
            ProfessionAction::CreateOrder => {
                let recipe = crate::content::recipe_template("field-tool-repair");
                let profession = request.profession.unwrap_or(recipe.profession);
                let required = recipe.materials;
                let has_required_materials =
                    state.phase4.materials.get(&key).is_some_and(|stock| {
                        has_materials(*stock, required) && stock.tools >= recipe.tools_required
                    });
                let board_has_room = state.phase4.orders.len() < super::MAX_SERVICE_ORDERS
                    || state.phase4.orders.iter().any(|order| {
                        matches!(
                            order.status,
                            ServiceOrderStatus::Completed | ServiceOrderStatus::Cancelled
                        )
                    });
                if profession != recipe.profession {
                    response.reason = Some(
                        "The current service order recipe requires the Carpenter profession."
                            .to_owned(),
                    );
                } else if !board_has_room {
                    response.reason = Some(
                        "The service order board is full; complete an existing order before adding another."
                            .to_owned(),
                    );
                } else if !has_required_materials {
                    response.reason = Some(
                        "This order shows its wood, iron, and tool requirements before creation."
                            .to_owned(),
                    );
                } else if !super::service_order_room(&mut state.phase4) {
                    response.reason = Some(
                        "The service order board is full; complete an existing order before adding another."
                            .to_owned(),
                    );
                } else {
                    let stock = state
                        .phase4
                        .materials
                        .get_mut(&key)
                        .expect("materials exist");
                    subtract_materials(stock, required);
                    stock.tools -= recipe.tools_required;
                    let order = ServiceOrder {
                        order_id: format!("service-order-{}", state.phase4.next_order_id),
                        requester_account_id: account_id(&state, &key),
                        requester_name: account_name(&state, &key),
                        provider_account_id: None,
                        provider_name: None,
                        service: recipe.service.clone(),
                        required_profession: recipe.profession,
                        materials: required,
                        tools_required: recipe.tools_required,
                        reward_gold: recipe.reward_gold,
                        benefit: recipe.benefit,
                        status: ServiceOrderStatus::Open,
                        quality: 0,
                        created_tick: state.tick,
                        completed_tick: None,
                    };
                    state.phase4.next_order_id = state.phase4.next_order_id.saturating_add(1);
                    response.order = Some(order.clone());
                    state.phase4.orders.push(order);
                    response.accepted = true;
                    record(&mut state, "service order created", "A need becomes work another profession can answer", "A player escrowed materials and posted a repair order on the settlement board.");
                }
            }
            ProfessionAction::AcceptOrder => {
                let Some(order_id) = order_id.as_deref() else {
                    response.reason = Some("Choose the service order to accept.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let Some(index) = state
                    .phase4
                    .orders
                    .iter()
                    .position(|order| order.order_id == order_id)
                else {
                    response.reason = Some("That service order is not on the board.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let order = state.phase4.orders[index].clone();
                if order.status != ServiceOrderStatus::Open {
                    response.reason = Some("Only an open order can be accepted.".to_owned());
                } else if order.requester_account_id == actor_id {
                    response.reason =
                        Some("A service order must be answered by another role.".to_owned());
                } else if !has_profession(&state, &key, order.required_profession) {
                    response.reason = Some("Learn or earn the displayed professional credential before accepting this order.".to_owned());
                } else {
                    let provider_name = account_name(&state, &key);
                    let order = &mut state.phase4.orders[index];
                    order.status = ServiceOrderStatus::Accepted;
                    order.provider_account_id = Some(actor_id.clone());
                    order.provider_name = Some(provider_name);
                    response.order = Some(order.clone());
                    response.accepted = true;
                    record(
                        &mut state,
                        "service order accepted",
                        "A second profession answers the settlement's need",
                        "A credentialed player took responsibility for an escrowed service order.",
                    );
                }
            }
            ProfessionAction::CompleteOrder => {
                let Some(order_id) = order_id.as_deref() else {
                    response.reason =
                        Some("Choose the accepted service order to complete.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let Some(index) = state
                    .phase4
                    .orders
                    .iter()
                    .position(|order| order.order_id == order_id)
                else {
                    response.reason = Some("That service order is not on the board.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let order = state.phase4.orders[index].clone();
                if request.timing_score.is_some_and(|score| score > 100) {
                    response.reason =
                        Some("Timing quality must be a visible score from 0 to 100.".to_owned());
                } else if order.status != ServiceOrderStatus::Accepted
                    || order.provider_account_id.as_deref() != Some(actor_id.as_str())
                {
                    response.reason =
                        Some("Only the named provider may complete an accepted order.".to_owned());
                } else {
                    let level = state
                        .phase4
                        .profiles
                        .get(&key)
                        .and_then(|profiles| {
                            profiles
                                .iter()
                                .find(|profile| profile.profession == order.required_profession)
                        })
                        .map(|profile| profile.level)
                        .unwrap_or(1);
                    let tick = state.tick;
                    let completed_order = {
                        let order = &mut state.phase4.orders[index];
                        order.status = ServiceOrderStatus::Completed;
                        let timing_score = request.timing_score.unwrap_or(50);
                        order.quality = (50 + u32::from(timing_score) / 2 + u32::from(level) * 5)
                            .min(100) as u8;
                        order.completed_tick = Some(tick);
                        order.clone()
                    };
                    response.order = Some(completed_order.clone());
                    let identity = state.identities.get_mut(&key).expect("identity exists");
                    identity.gold = identity.gold.saturating_add(completed_order.reward_gold);
                    identity.skill = identity.skill.saturating_add(1);
                    if completed_order
                        .service
                        .to_ascii_lowercase()
                        .contains("tool")
                    {
                        if let Some(requester_key) =
                            super::key_for_account(&state, &completed_order.requester_account_id)
                        {
                            state
                                .identities
                                .get_mut(&requester_key)
                                .expect("requester identity exists")
                                .field_tool_condition = super::super::FIELD_TOOL_MAX_CONDITION;
                        }
                    }
                    let credentials = state.phase4.credentials.entry(key.clone()).or_default();
                    remember_credential(
                        credentials,
                        format!("completed {}", completed_order.service),
                    );
                    response.accepted = true;
                    let actor_name = account_name(&state, &key);
                    record(&mut state, "service order completed", "Craft and demand meet at the Hearth", &format!("{} completed {} at {} quality; the requesting role receives the listed benefit.", actor_name, completed_order.service, completed_order.quality));
                }
            }
        }
        if response.accepted {
            let skill_id = match action {
                ProfessionAction::LearnCapability | ProfessionAction::CompleteOrder => {
                    Some("carpentry")
                }
                _ => None,
            };
            if let Some(skill_id) = skill_id {
                super::super::skills::record_practice(&mut state, &key, skill_id);
            }
        }
        response.professions = view(&state, &key);
        finish(self, &mut state, cache, request.request_id, response)
    }
}

fn finish(
    repository: &super::super::WorldRepository,
    state: &mut super::super::models::RepositoryState,
    cache: String,
    request_id: String,
    response: ProfessionResponse,
) -> Result<ApiResponse<ProfessionResponse>, super::super::RepositoryError> {
    state
        .phase4
        .request_results
        .insert(cache, super::Phase4Response::Profession(response.clone()));
    super::super::record_command_outcome(state, response.accepted);
    repository.persist(state);
    Ok(ApiResponse {
        meta: super::super::meta(state.tick, Some(request_id), Some(state.cursor)),
        data: response,
    })
}

fn ensure_player(state: &mut super::super::models::RepositoryState, key: &str) {
    state
        .phase4
        .profiles
        .entry(key.to_owned())
        .or_insert_with(default_profiles);
    state
        .phase4
        .materials
        .entry(key.to_owned())
        .or_insert_with(default_materials);
    state.phase4.credentials.entry(key.to_owned()).or_default();
}

fn default_profiles() -> Vec<ProfessionProfile> {
    vec![ProfessionProfile {
        profession: ProfessionKind::Farmer,
        level: 1,
        reputation: 0,
        credential: Some("settlement-fieldhand credential".to_owned()),
        capabilities: vec![default_capability(ProfessionKind::Farmer)],
    }]
}

fn default_materials() -> MaterialStock {
    MaterialStock {
        wood: 3,
        iron: 2,
        cloth: 1,
        bandages: 1,
        tools: 1,
    }
}

pub(super) fn remember_credential(credentials: &mut Vec<String>, credential: String) {
    if !credentials.iter().any(|existing| existing == &credential) {
        credentials.push(credential);
    }
}

fn view(state: &super::super::models::RepositoryState, key: &str) -> ProfessionsResponse {
    ProfessionsResponse {
        profiles: state
            .phase4
            .profiles
            .get(key)
            .cloned()
            .unwrap_or_else(default_profiles),
        orders: state.phase4.orders.clone(),
        materials: state
            .phase4
            .materials
            .get(key)
            .copied()
            .unwrap_or_else(default_materials),
        credentials: state
            .phase4
            .credentials
            .get(key)
            .cloned()
            .unwrap_or_default(),
        cursor: state.cursor,
    }
}

fn has_profession(
    state: &super::super::models::RepositoryState,
    key: &str,
    profession: ProfessionKind,
) -> bool {
    state.phase4.profiles.get(key).is_some_and(|profiles| {
        profiles
            .iter()
            .any(|profile| profile.profession == profession)
    })
}

fn has_materials(stock: MaterialStock, required: MaterialStock) -> bool {
    stock.wood >= required.wood
        && stock.iron >= required.iron
        && stock.cloth >= required.cloth
        && stock.bandages >= required.bandages
        && stock.tools >= required.tools
}

fn subtract_materials(stock: &mut MaterialStock, required: MaterialStock) {
    stock.wood -= required.wood;
    stock.iron -= required.iron;
    stock.cloth -= required.cloth;
    stock.bandages -= required.bandages;
}
