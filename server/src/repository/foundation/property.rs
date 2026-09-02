use super::super::models::{trim_replay_cache, RepositoryState};
use crate::config::ServerConfig;
use tarrowyn_protocol::{
    ApiResponse, FoundationPropertyAccess, FoundationPropertyAction, FoundationPropertyContract,
    FoundationPropertyPlacementPreview, FoundationPropertyProjection, FoundationPropertyRequest,
    FoundationPropertyResponse, FoundationPropertyStage, FoundationPropertyState,
    FoundationPropertySummary, FoundationResourceKind, Inventory,
};

mod lifecycle;
mod spatial;
pub(crate) use lifecycle::{
    integrity_ok, migrate_account, movement_blocked, remove_account, restore_properties,
};
use spatial::*;

const MAX_PROPERTIES: usize = 128;
const MAX_PROPERTY_ID_CHARS: usize = 160;
const MAX_CONDITION: u8 = 100;

impl super::super::WorldRepository {
    pub fn foundation_properties(
        &self,
        token: &str,
    ) -> Result<ApiResponse<FoundationPropertyProjection>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        let identity_key = super::super::authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: super::super::meta(state.tick, None, Some(state.cursor)),
            data: projection(&state, &identity_key, None),
        })
    }

    pub fn foundation_property(
        &self,
        token: &str,
        request: FoundationPropertyRequest,
    ) -> Result<ApiResponse<FoundationPropertyResponse>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        let identity_key = super::super::authenticate(&mut state, token, &self.config)?;
        super::super::validate_request_id(&request.request_id)?;
        if let Some(property_id) = &request.property_id {
            super::super::validate_bounded_text(
                property_id,
                MAX_PROPERTY_ID_CHARS,
                "invalid_property_id",
                "A property selector must contain 1 to 160 characters and no control characters.",
            )?;
        }
        if let Some(previous) = state
            .identities
            .get(&identity_key)
            .and_then(|identity| {
                identity
                    .foundation_property_results
                    .get(&request.request_id)
            })
            .cloned()
        {
            return Ok(ApiResponse {
                meta: super::super::meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous,
            });
        }
        let mut response = FoundationPropertyResponse {
            request_id: request.request_id.clone(),
            action: request.action,
            accepted: false,
            projection: projection(&state, &identity_key, None),
            player: super::super::player_projection(&state, &identity_key),
            reason: None,
        };
        if state
            .identities
            .get(&identity_key)
            .is_some_and(|identity| identity.knocked_out)
            && !matches!(
                request.action,
                FoundationPropertyAction::PreviewPlacement | FoundationPropertyAction::Inspect
            )
        {
            response.reason =
                Some("Recover at the First Beacon before working on personal shelter.".to_owned());
            return finish(self, &mut state, identity_key, response, None);
        }

        let preview = match request.action {
            FoundationPropertyAction::PreviewPlacement => Some(preview_placement(
                &state,
                &identity_key,
                &request,
                &self.config,
                None,
            )),
            FoundationPropertyAction::PlaceTent => place_tent(
                &mut state,
                &identity_key,
                &request,
                &self.config,
                &mut response,
            ),
            FoundationPropertyAction::Inspect => {
                inspect(&state, &identity_key, &request, &mut response);
                None
            }
            FoundationPropertyAction::UpgradeWithMaterials => upgrade(
                &mut state,
                &identity_key,
                &request,
                &self.config,
                false,
                &mut response,
            ),
            FoundationPropertyAction::HireBuilder => upgrade(
                &mut state,
                &identity_key,
                &request,
                &self.config,
                true,
                &mut response,
            ),
            FoundationPropertyAction::SetAccess => {
                set_access(&mut state, &identity_key, &request, &mut response);
                None
            }
            FoundationPropertyAction::Store => {
                transfer(&mut state, &identity_key, &request, true, &mut response);
                None
            }
            FoundationPropertyAction::Collect => {
                transfer(&mut state, &identity_key, &request, false, &mut response);
                None
            }
            FoundationPropertyAction::Maintain => {
                maintain(&mut state, &identity_key, &request, &mut response);
                None
            }
        };
        finish(self, &mut state, identity_key, response, preview)
    }
}

fn place_tent(
    state: &mut RepositoryState,
    identity_key: &str,
    request: &FoundationPropertyRequest,
    config: &ServerConfig,
    response: &mut FoundationPropertyResponse,
) -> Option<FoundationPropertyPlacementPreview> {
    let preview = preview_placement(state, identity_key, request, config, None);
    if !preview.accepted {
        response.reason = Some(preview.message.clone());
        return Some(preview);
    }
    let identity = state.identities.get(identity_key).expect("identity exists");
    if state
        .foundation_properties
        .iter()
        .any(|property| property.owner_account_id == identity.account_id)
    {
        response.reason = Some("Each resident may place one personal shelter.".to_owned());
        return Some(preview);
    }
    if state.foundation_properties.len() >= MAX_PROPERTIES {
        response.reason = Some("The bounded property ledger is full.".to_owned());
        return Some(preview);
    }
    let property_id = format!("personal-property-{}", state.next_property);
    state.next_property = state.next_property.saturating_add(1);
    let tent = &FoundationPropertyContract::default().stages[0];
    state.foundation_properties.push(FoundationPropertyState {
        property_id,
        owner_account_id: identity.account_id.clone(),
        owner_name: identity.display_name.clone(),
        stage: FoundationPropertyStage::Tent,
        anchor: preview.anchor,
        entrance: preview.entrance,
        access: FoundationPropertyAccess::OwnerOnly,
        revision: 1,
        condition: MAX_CONDITION,
        last_maintained_unix_millis: super::super::models::unix_time_millis(),
        storage: Inventory::default(),
        storage_capacity: tent.storage_capacity,
    });
    response.accepted = true;
    Some(preview)
}

fn inspect(
    state: &RepositoryState,
    identity_key: &str,
    request: &FoundationPropertyRequest,
    response: &mut FoundationPropertyResponse,
) {
    let Some(index) = selected_property_index(state, identity_key, request.property_id.as_deref())
    else {
        response.reason = Some("That personal shelter does not exist.".to_owned());
        return;
    };
    if !near_property(state, identity_key, &state.foundation_properties[index]) {
        response.reason = Some("Walk beside that personal shelter first.".to_owned());
        return;
    }
    response.accepted = true;
}

fn upgrade(
    state: &mut RepositoryState,
    identity_key: &str,
    request: &FoundationPropertyRequest,
    config: &ServerConfig,
    hire_builder: bool,
    response: &mut FoundationPropertyResponse,
) -> Option<FoundationPropertyPlacementPreview> {
    let Some(index) = selected_property_index(state, identity_key, request.property_id.as_deref())
    else {
        response.reason = Some("Place a personal tent before improving it.".to_owned());
        return None;
    };
    let account_id = state.identities[identity_key].account_id.clone();
    if state.foundation_properties[index].owner_account_id != account_id {
        response.reason = Some("Only the owner may improve this shelter.".to_owned());
        return None;
    }
    if hire_builder {
        let mara = crate::content::foundation_baseline()
            .landmarks
            .into_iter()
            .find(|landmark| landmark.id == "builder-mara")
            .expect("validated Mara landmark");
        if state.identities[identity_key]
            .position
            .manhattan_distance(mara.position)
            > 1
        {
            response.reason =
                Some("Talk to Mara beside the Beacon to hire builder help.".to_owned());
            return None;
        }
    } else if !near_property(state, identity_key, &state.foundation_properties[index]) {
        response.reason = Some("Walk beside your shelter to improve it yourself.".to_owned());
        return None;
    }
    let next_stage = match state.foundation_properties[index].stage {
        FoundationPropertyStage::Tent => FoundationPropertyStage::Camp,
        FoundationPropertyStage::Camp => FoundationPropertyStage::House,
        FoundationPropertyStage::House => {
            response.reason = Some("This shelter is already a first house.".to_owned());
            return None;
        }
    };
    let contract = FoundationPropertyContract::default();
    let definition = contract
        .stages
        .iter()
        .find(|stage| stage.stage == next_stage)
        .expect("complete property contract");
    let property = &state.foundation_properties[index];
    let preview_request = FoundationPropertyRequest {
        request_id: request.request_id.clone(),
        action: FoundationPropertyAction::PreviewPlacement,
        property_id: Some(property.property_id.clone()),
        anchor: Some(property.anchor),
        entrance: Some(property.entrance),
        access: None,
        resource: None,
        amount: 0,
    };
    let preview = preview_for_footprint(
        state,
        identity_key,
        &preview_request,
        config,
        definition.footprint,
        Some(property.property_id.as_str()),
    );
    if !preview.accepted {
        response.reason = Some(format!(
            "The larger shelter cannot fit here: {}",
            preview.message
        ));
        return Some(preview);
    }
    if hire_builder {
        let inventory = state.identities[identity_key].inventory;
        let gold_cost = definition.material_costs.iter().fold(0_u32, |total, cost| {
            let missing = cost
                .amount
                .saturating_sub(inventory_amount(&inventory, cost.kind));
            total.saturating_add(missing.saturating_mul(builder_gold_per_unit(cost.kind)))
        });
        if state.identities[identity_key].gold < gold_cost {
            response.reason = Some(format!(
                "Mara needs {} gold to supply the missing materials for this stage.",
                gold_cost
            ));
            return Some(preview);
        }
        let identity = state
            .identities
            .get_mut(identity_key)
            .expect("identity exists");
        identity.gold -= gold_cost;
        for cost in &definition.material_costs {
            let carried = inventory_amount(&identity.inventory, cost.kind);
            *inventory_amount_mut(&mut identity.inventory, cost.kind) =
                carried.saturating_sub(cost.amount);
        }
    } else {
        let inventory = &state.identities[identity_key].inventory;
        if let Some(missing) = definition
            .material_costs
            .iter()
            .find(|cost| inventory_amount(inventory, cost.kind) < cost.amount)
        {
            response.reason = Some(format!(
                "This stage needs {} {}.",
                missing.amount,
                resource_label(missing.kind)
            ));
            return Some(preview);
        }
        for cost in &definition.material_costs {
            *inventory_amount_mut(
                &mut state
                    .identities
                    .get_mut(identity_key)
                    .expect("identity exists")
                    .inventory,
                cost.kind,
            ) -= cost.amount;
        }
    }
    let property = &mut state.foundation_properties[index];
    property.stage = next_stage;
    property.storage_capacity = definition.storage_capacity;
    property.revision = property.revision.saturating_add(1);
    property.condition = effective_condition(property, super::super::models::unix_time_millis());
    response.accepted = true;
    Some(preview)
}

fn set_access(
    state: &mut RepositoryState,
    identity_key: &str,
    request: &FoundationPropertyRequest,
    response: &mut FoundationPropertyResponse,
) {
    let Some(index) = selected_property_index(state, identity_key, request.property_id.as_deref())
    else {
        response.reason = Some("That personal shelter does not exist.".to_owned());
        return;
    };
    if state.foundation_properties[index].owner_account_id
        != state.identities[identity_key].account_id
    {
        response.reason = Some("Only the owner may change shelter access.".to_owned());
        return;
    }
    if !near_property(state, identity_key, &state.foundation_properties[index]) {
        response.reason = Some("Walk beside your shelter before changing access.".to_owned());
        return;
    }
    let Some(access) = request.access else {
        response.reason = Some("Choose owner-only or guest access.".to_owned());
        return;
    };
    let property = &mut state.foundation_properties[index];
    property.access = access;
    property.revision = property.revision.saturating_add(1);
    response.accepted = true;
}

fn transfer(
    state: &mut RepositoryState,
    identity_key: &str,
    request: &FoundationPropertyRequest,
    storing: bool,
    response: &mut FoundationPropertyResponse,
) {
    let Some(index) = selected_property_index(state, identity_key, request.property_id.as_deref())
    else {
        response.reason = Some("That personal shelter does not exist.".to_owned());
        return;
    };
    if !near_property(state, identity_key, &state.foundation_properties[index]) {
        response.reason = Some("Walk beside that shelter's chest first.".to_owned());
        return;
    }
    let owner = state.foundation_properties[index].owner_account_id
        == state.identities[identity_key].account_id;
    if !owner
        && (!storing
            || state.foundation_properties[index].access != FoundationPropertyAccess::GuestsAllowed)
    {
        response.reason = Some(
            if storing {
                "The owner has not opened this chest to guest deposits."
            } else {
                "Only the owner may collect from this chest."
            }
            .to_owned(),
        );
        return;
    }
    let Some(resource) = request.resource else {
        response.reason = Some("Choose timber, stone, or iron ore.".to_owned());
        return;
    };
    if request.amount == 0 || request.amount > 99 {
        response.reason = Some("Move between 1 and 99 materials at once.".to_owned());
        return;
    }
    if storing {
        if inventory_amount(&state.identities[identity_key].inventory, resource) < request.amount {
            response.reason = Some("You do not carry that many materials.".to_owned());
            return;
        }
        if state.foundation_properties[index]
            .storage
            .total_items()
            .saturating_add(request.amount)
            > state.foundation_properties[index].storage_capacity
        {
            response.reason = Some("That shelter chest does not have enough room.".to_owned());
            return;
        }
        *inventory_amount_mut(
            &mut state
                .identities
                .get_mut(identity_key)
                .expect("identity exists")
                .inventory,
            resource,
        ) -= request.amount;
        *inventory_amount_mut(&mut state.foundation_properties[index].storage, resource) +=
            request.amount;
    } else {
        if inventory_amount(&state.foundation_properties[index].storage, resource) < request.amount
        {
            response.reason = Some("The chest does not hold that many materials.".to_owned());
            return;
        }
        *inventory_amount_mut(&mut state.foundation_properties[index].storage, resource) -=
            request.amount;
        let carried = inventory_amount(&state.identities[identity_key].inventory, resource);
        *inventory_amount_mut(
            &mut state
                .identities
                .get_mut(identity_key)
                .expect("identity exists")
                .inventory,
            resource,
        ) = carried.saturating_add(request.amount);
    }
    state.foundation_properties[index].revision = state.foundation_properties[index]
        .revision
        .saturating_add(1);
    response.accepted = true;
}

fn maintain(
    state: &mut RepositoryState,
    identity_key: &str,
    request: &FoundationPropertyRequest,
    response: &mut FoundationPropertyResponse,
) {
    let Some(index) = selected_property_index(state, identity_key, request.property_id.as_deref())
    else {
        response.reason = Some("That personal shelter does not exist.".to_owned());
        return;
    };
    if state.foundation_properties[index].owner_account_id
        != state.identities[identity_key].account_id
    {
        response.reason = Some("Only the owner may maintain this shelter.".to_owned());
        return;
    }
    if !near_property(state, identity_key, &state.foundation_properties[index]) {
        response.reason = Some("Walk beside your shelter before maintaining it.".to_owned());
        return;
    }
    let now = super::super::models::unix_time_millis();
    let restore = FoundationPropertyContract::default()
        .upkeep
        .maintenance_restores_condition;
    let property = &mut state.foundation_properties[index];
    property.condition = effective_condition(property, now)
        .saturating_add(restore)
        .min(MAX_CONDITION);
    property.last_maintained_unix_millis = now;
    property.revision = property.revision.saturating_add(1);
    response.accepted = true;
}

fn finish(
    repository: &super::super::WorldRepository,
    state: &mut RepositoryState,
    identity_key: String,
    mut response: FoundationPropertyResponse,
    preview: Option<FoundationPropertyPlacementPreview>,
) -> Result<ApiResponse<FoundationPropertyResponse>, super::super::RepositoryError> {
    response.projection = projection(state, &identity_key, preview);
    response.player = super::super::player_projection(state, &identity_key);
    let account_id = response.player.account_id.clone();
    super::super::phase6::audit_command(
        state,
        &account_id,
        "foundation.property",
        response
            .projection
            .own_property
            .as_ref()
            .map_or("placement", |property| property.property_id.as_str()),
        response.accepted,
        "A proximity-checked personal property action was recorded.",
    );
    let request_id = response.request_id.clone();
    let cache = &mut state
        .identities
        .get_mut(&identity_key)
        .expect("identity exists")
        .foundation_property_results;
    cache.insert(request_id.clone(), response.clone());
    trim_replay_cache(cache);
    super::super::record_command_outcome(state, response.accepted);
    repository.persist(state)?;
    Ok(ApiResponse {
        meta: super::super::meta(state.tick, Some(request_id), Some(state.cursor)),
        data: response,
    })
}

fn projection(
    state: &RepositoryState,
    identity_key: &str,
    placement_preview: Option<FoundationPropertyPlacementPreview>,
) -> FoundationPropertyProjection {
    let now = super::super::models::unix_time_millis();
    let account_id = state.identities[identity_key].account_id.as_str();
    FoundationPropertyProjection {
        contract: FoundationPropertyContract::default(),
        properties: state
            .foundation_properties
            .iter()
            .map(|property| summary(property, now))
            .collect(),
        own_property: state
            .foundation_properties
            .iter()
            .find(|property| property.owner_account_id == account_id)
            .cloned()
            .map(|mut property| {
                property.condition = effective_condition(&property, now);
                property
            }),
        placement_preview,
    }
}

fn summary(property: &FoundationPropertyState, now: u64) -> FoundationPropertySummary {
    FoundationPropertySummary {
        property_id: property.property_id.clone(),
        owner_account_id: property.owner_account_id.clone(),
        owner_name: property.owner_name.clone(),
        stage: property.stage,
        anchor: property.anchor,
        entrance: property.entrance,
        access: property.access,
        revision: property.revision,
        condition: effective_condition(property, now),
        stored_units: property.storage.total_items(),
        storage_capacity: property.storage_capacity,
    }
}

fn effective_condition(property: &FoundationPropertyState, now: u64) -> u8 {
    let upkeep = FoundationPropertyContract::default().upkeep;
    let interval = u64::from(upkeep.interval_real_days)
        .saturating_mul(24 * 60 * 60 * 1_000)
        .max(1);
    let cycles = now.saturating_sub(property.last_maintained_unix_millis) / interval;
    let loss = u8::try_from(cycles)
        .unwrap_or(u8::MAX)
        .saturating_mul(upkeep.condition_loss_per_interval);
    property
        .condition
        .saturating_sub(loss)
        .max(upkeep.minimum_condition)
}

fn selected_property_index(
    state: &RepositoryState,
    identity_key: &str,
    requested_id: Option<&str>,
) -> Option<usize> {
    let own_account = state.identities[identity_key].account_id.as_str();
    state.foundation_properties.iter().position(|property| {
        requested_id.map_or(property.owner_account_id == own_account, |property_id| {
            property.property_id == property_id
        })
    })
}

fn near_property(
    state: &RepositoryState,
    identity_key: &str,
    property: &FoundationPropertyState,
) -> bool {
    let position = state.identities[identity_key].position;
    footprint_tiles(property.anchor, stage_definition(property.stage).footprint)
        .into_iter()
        .chain(std::iter::once(entrance_position(
            property.anchor,
            stage_definition(property.stage).footprint,
            property.entrance,
        )))
        .any(|tile| position.manhattan_distance(tile) <= 1)
}

fn inventory_amount(inventory: &Inventory, kind: FoundationResourceKind) -> u32 {
    match kind {
        FoundationResourceKind::Timber => inventory.timber,
        FoundationResourceKind::Stone => inventory.stone,
        FoundationResourceKind::IronOre => inventory.iron_ore,
    }
}

fn inventory_amount_mut(inventory: &mut Inventory, kind: FoundationResourceKind) -> &mut u32 {
    match kind {
        FoundationResourceKind::Timber => &mut inventory.timber,
        FoundationResourceKind::Stone => &mut inventory.stone,
        FoundationResourceKind::IronOre => &mut inventory.iron_ore,
    }
}

fn resource_label(kind: FoundationResourceKind) -> &'static str {
    match kind {
        FoundationResourceKind::Timber => "timber",
        FoundationResourceKind::Stone => "stone",
        FoundationResourceKind::IronOre => "iron ore",
    }
}

fn builder_gold_per_unit(kind: FoundationResourceKind) -> u32 {
    let policy = FoundationPropertyContract::default().builder;
    match kind {
        FoundationResourceKind::Timber => policy.timber_gold_per_unit,
        FoundationResourceKind::Stone => policy.stone_gold_per_unit,
        FoundationResourceKind::IronOre => 0,
    }
}
