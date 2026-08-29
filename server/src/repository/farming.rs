use super::*;
use tarrowyn_protocol::{CropKind, CropState, FarmingAction};

impl WorldRepository {
    pub fn farming(
        &self,
        token: &str,
        request: FarmingRequest,
    ) -> Result<ApiResponse<FarmingResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let identity_key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        if let Some(previous) = state
            .identities
            .get(&identity_key)
            .and_then(|identity| identity.farming_results.get(&request.request_id))
            .cloned()
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous,
            });
        }
        let current = state
            .identities
            .get(&identity_key)
            .expect("identity exists")
            .position;
        let mut response = FarmingResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            action: request.action,
            plot: None,
            animal: None,
            player: player_projection(&state, &identity_key),
            reason: None,
        };
        if request.action == FarmingAction::TendAnimal {
            let Some(animal_index) = state.phase4.animals.iter().position(|animal| {
                animal.position == request.position
                    || animal.position.manhattan_distance(request.position) <= 1
            }) else {
                response.reason = Some(
                    "Stand beside the Bellweather goat near the shared fields before caring for it."
                        .to_owned(),
                );
                return self.store_farming_result(&mut state, identity_key, response);
            };
            let distance = current.manhattan_distance(request.position);
            if distance > 1 {
                response.reason = Some("Stand beside the animal before caring for it.".to_owned());
                response.animal = Some(state.phase4.animals[animal_index].clone());
                return self.store_farming_result(&mut state, identity_key, response);
            }
            let result = self.tend_animal(&mut state, &identity_key, animal_index);
            response.accepted = result.0;
            response.reason = result.1;
            response.animal = Some(state.phase4.animals[animal_index].clone());
            response.player = player_projection(&state, &identity_key);
            if response.accepted {
                super::skills::record_practice(&mut state, &identity_key, "animal-husbandry");
                add_notice(
                    &mut state,
                    "fields",
                    super::world::farming_notice(request.action),
                );
            }
            return self.store_farming_result(&mut state, identity_key, response);
        }
        let Some(plot_index) = state
            .plots
            .iter()
            .position(|plot| plot.position == request.position)
        else {
            response.reason = Some("That position is not a shared field plot.".to_owned());
            return self.store_farming_result(&mut state, identity_key, response);
        };
        let distance = current.manhattan_distance(request.position);
        if distance > 1 {
            response.reason = Some("Stand beside the field plot before tending it.".to_owned());
            return self.store_farming_result(&mut state, identity_key, response);
        }
        let result = match request.action {
            FarmingAction::Plant => self.plant(&mut state, &identity_key, plot_index),
            FarmingAction::Tend => self.tend(&mut state, &identity_key, plot_index),
            FarmingAction::Harvest => self.harvest(&mut state, &identity_key, plot_index),
            FarmingAction::TendAnimal => unreachable!("animal care is handled above"),
        };
        response.accepted = result.0;
        response.reason = result.1;
        response.plot = Some(state.plots[plot_index]);
        response.player = player_projection(&state, &identity_key);
        if response.accepted {
            super::skills::record_practice(&mut state, &identity_key, "crop-tending");
            let plot = state.plots[plot_index];
            push_event(&mut state, WorldEvent::Farming(plot));
            add_notice(
                &mut state,
                "fields",
                super::world::farming_notice(request.action),
            );
        }
        self.store_farming_result(&mut state, identity_key, response)
    }

    fn tend_animal(
        &self,
        state: &mut RepositoryState,
        identity_key: &str,
        animal_index: usize,
    ) -> (bool, Option<String>) {
        let animal = &mut state.phase4.animals[animal_index];
        if animal.condition >= animal.max_condition {
            return (
                false,
                Some(
                    "Bellweather is already thriving; return when the goat needs care.".to_owned(),
                ),
            );
        }
        animal.condition = animal.max_condition;
        animal.last_cared_tick = state.tick;
        animal.last_cared_day = state.clock.day;
        state
            .identities
            .get_mut(identity_key)
            .expect("identity exists")
            .skill += 1;
        (true, None)
    }

    fn plant(
        &self,
        state: &mut RepositoryState,
        identity_key: &str,
        plot_index: usize,
    ) -> (bool, Option<String>) {
        let identity = state
            .identities
            .get_mut(identity_key)
            .expect("identity exists");
        if identity.inventory.seeds == 0 {
            return (
                false,
                Some("The seed pouch is empty; trade for more at the Hearth.".to_owned()),
            );
        }
        if state.plots[plot_index].crop.is_some() {
            return (
                false,
                Some("That field plot is already occupied.".to_owned()),
            );
        }
        let kind = crate::content::crop_kind_for_seed(identity.seeds_planted);
        identity.inventory.seeds -= 1;
        identity.seeds_planted += 1;
        state.plots[plot_index].crop = Some(CropState {
            kind,
            stage: 0,
            quality: 1,
            planted_tick: state.tick,
            last_tended_tick: None,
        });
        (true, None)
    }

    fn tend(
        &self,
        state: &mut RepositoryState,
        identity_key: &str,
        plot_index: usize,
    ) -> (bool, Option<String>) {
        if state
            .identities
            .get(identity_key)
            .is_some_and(|identity| identity.field_tool_condition == 0)
        {
            return (
                false,
                Some(
                    "The field tool is worn out; commission a repair before tending again."
                        .to_owned(),
                ),
            );
        }
        let Some(mut crop) = state.plots[plot_index].crop else {
            return (false, Some("That plot is empty.".to_owned()));
        };
        if crop.mature() {
            return (false, Some("That crop is ready to harvest.".to_owned()));
        }
        crop.stage = crop.stage.saturating_add(1).min(CropState::MATURE_STAGE);
        crop.quality = crop.quality.saturating_add(1).min(3);
        crop.last_tended_tick = Some(state.tick);
        state.plots[plot_index].crop = Some(crop);
        let identity = state
            .identities
            .get_mut(identity_key)
            .expect("identity exists");
        identity.field_tool_condition = identity.field_tool_condition.saturating_sub(1);
        identity.skill += 1;
        (true, None)
    }

    fn harvest(
        &self,
        state: &mut RepositoryState,
        identity_key: &str,
        plot_index: usize,
    ) -> (bool, Option<String>) {
        let Some(crop) = state.plots[plot_index].crop else {
            return (false, Some("That plot is empty.".to_owned()));
        };
        if !crop.mature() {
            return (
                false,
                Some("The crop is not ready; tend it or let the shared clock work.".to_owned()),
            );
        }
        let identity = state
            .identities
            .get_mut(identity_key)
            .expect("identity exists");
        match crop.kind {
            CropKind::Wheat => identity.inventory.wheat += 1,
            CropKind::Turnip => identity.inventory.turnips += 1,
            CropKind::Moonberry => identity.inventory.moonberries += 1,
        }
        let base_value = crop.kind.value() + crop.quality as u32;
        identity.gold += super::phase3::harvest_price_bonus(&state.phase3, base_value);
        identity.skill += 1;
        state.plots[plot_index].crop = None;
        (true, None)
    }

    fn store_farming_result(
        &self,
        state: &mut RepositoryState,
        identity_key: String,
        response: FarmingResponse,
    ) -> Result<ApiResponse<FarmingResponse>, RepositoryError> {
        let request_id = response.request_id.clone();
        state
            .identities
            .get_mut(&identity_key)
            .expect("identity exists")
            .farming_results
            .insert(request_id.clone(), response.clone());
        record_command_outcome(state, response.accepted);
        self.persist(state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request_id), Some(state.cursor)),
            data: response,
        })
    }
}
