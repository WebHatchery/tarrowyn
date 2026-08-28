use super::{account_id, account_name, cache_key, default_capability, record, validate_request_id};
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
        super::super::expire_sessions(&mut state, &self.config);
        let key = super::super::authenticate(&mut state, token, &self.config)?;
        ensure_player(&mut state, &key);
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
        super::super::expire_sessions(&mut state, &self.config);
        let key = super::super::authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
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
                    state
                        .phase4
                        .credentials
                        .entry(key.clone())
                        .or_default()
                        .push(format!("{:?} apprentice credential", profession));
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
                let profession = request.profession.unwrap_or(ProfessionKind::Carpenter);
                let stock = state
                    .phase4
                    .materials
                    .get_mut(&key)
                    .expect("materials exist");
                let required = MaterialStock {
                    wood: 1,
                    iron: 1,
                    ..MaterialStock::default()
                };
                if !has_materials(*stock, required) || stock.tools < 1 {
                    response.reason = Some(
                        "This order shows its wood, iron, and tool requirements before creation."
                            .to_owned(),
                    );
                } else {
                    subtract_materials(stock, required);
                    stock.tools -= 1;
                    let order = ServiceOrder {
                        order_id: format!("service-order-{}", state.phase4.next_order_id),
                        requester_account_id: account_id(&state, &key),
                        requester_name: account_name(&state, &key),
                        provider_account_id: None,
                        provider_name: None,
                        service: request.service.unwrap_or_else(|| "Repair a field tool".to_owned()),
                        required_profession: profession,
                        materials: required,
                        tools_required: 1,
                        reward_gold: 5,
                        benefit: "The requesting farmer receives a reliable tool and a visible quality improvement.".to_owned(),
                        status: ServiceOrderStatus::Open,
                        quality: 0,
                        created_tick: state.tick,
                        completed_tick: None,
                    };
                    state.phase4.next_order_id += 1;
                    response.order = Some(order.clone());
                    state.phase4.orders.push(order);
                    state.phase4.orders.truncate(64);
                    response.accepted = true;
                    record(&mut state, "service order created", "A need becomes work another profession can answer", "A player escrowed materials and posted a repair order on the settlement board.");
                }
            }
            ProfessionAction::AcceptOrder => {
                let Some(order_id) = request.order_id.as_deref() else {
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
                let Some(order_id) = request.order_id.as_deref() else {
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
                if order.status != ServiceOrderStatus::Accepted
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
                        order.quality = (70 + level.saturating_mul(10)).min(100);
                        order.completed_tick = Some(tick);
                        order.clone()
                    };
                    response.order = Some(completed_order.clone());
                    let identity = state.identities.get_mut(&key).expect("identity exists");
                    identity.gold = identity.gold.saturating_add(completed_order.reward_gold);
                    identity.skill = identity.skill.saturating_add(1);
                    state
                        .phase4
                        .credentials
                        .entry(key.clone())
                        .or_default()
                        .push(format!("completed {}", completed_order.service));
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
        .or_insert_with(|| {
            vec![ProfessionProfile {
                profession: ProfessionKind::Farmer,
                level: 1,
                reputation: 0,
                credential: Some("settlement-fieldhand credential".to_owned()),
                capabilities: vec![default_capability(ProfessionKind::Farmer)],
            }]
        });
    state
        .phase4
        .materials
        .entry(key.to_owned())
        .or_insert(MaterialStock {
            wood: 3,
            iron: 2,
            cloth: 1,
            bandages: 1,
            tools: 1,
        });
    state.phase4.credentials.entry(key.to_owned()).or_default();
}

fn view(state: &super::super::models::RepositoryState, key: &str) -> ProfessionsResponse {
    ProfessionsResponse {
        profiles: state.phase4.profiles.get(key).cloned().unwrap_or_default(),
        orders: state.phase4.orders.clone(),
        materials: state.phase4.materials.get(key).copied().unwrap_or_default(),
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
