use super::*;
use macroquad_toolkit::ui::draw_ui_text_ex;
use tarrowyn_protocol::{
    FarmingAction, FieldWeather, FoundationActivityState, FoundationBaseline,
    FoundationCacheAction, FoundationFieldToolKind, FoundationForgeAction,
    FoundationJourneyFutureGoalState, FoundationJourneyProjection, FoundationLandmark,
    FoundationResourceAction, FoundationResourceKind, Inventory, TradeOffer, TradeStatus,
};

#[path = "ui_foundation/property.rs"]
mod property;
#[path = "ui_foundation/storehouse.rs"]
mod storehouse;
use property::{draw_controls as draw_property_controls, nearby_choice as nearby_property_choice};
use storehouse::{
    draw_controls as draw_storehouse_controls, nearby_choice as nearby_storehouse_choice,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundationContext<'a> {
    pub landmark: &'a FoundationLandmark,
    pub interaction_id: &'a str,
    pub interaction_action: &'a str,
    pub action_label: String,
    pub resource_node_id: Option<&'a str>,
    pub resource_action: Option<FoundationResourceAction>,
    pub cache_action: Option<FoundationCacheAction>,
    pub cache_resource: Option<FoundationResourceKind>,
}

pub(crate) fn nearby_context<'a>(
    baseline: &'a FoundationBaseline,
    activity: &'a FoundationActivityState,
    player: TilePos,
    inventory: Option<&Inventory>,
) -> Option<FoundationContext<'a>> {
    baseline
        .landmarks
        .iter()
        .enumerate()
        .filter_map(|(index, landmark)| {
            let distance =
                player.manhattan_distance(&TilePos::new(landmark.position.x, landmark.position.y));
            let interaction = baseline
                .interactions
                .iter()
                .find(|interaction| interaction.landmark_id == landmark.id)?;
            (landmark.visible && distance <= 1).then_some((distance, index, landmark, interaction))
        })
        .min_by_key(|(distance, index, _, _)| (*distance, *index))
        .map(|(_, _, landmark, interaction)| {
            let resource_action = match interaction.action.as_str() {
                "log" => Some(FoundationResourceAction::Log),
                "mine" => Some(FoundationResourceAction::Mine),
                _ => None,
            };
            let resource_node_id = resource_action.and_then(|_| {
                activity
                    .resource_nodes
                    .iter()
                    .find(|node| node.landmark_id == landmark.id)
                    .map(|node| node.node_id.as_str())
            });
            let cache_choice = (interaction.action == "deposit_or_collect")
                .then(|| shared_cache_choice(activity, inventory))
                .flatten();
            FoundationContext {
                landmark,
                interaction_id: interaction.id.as_str(),
                interaction_action: interaction.action.as_str(),
                action_label: cache_choice.as_ref().map_or_else(
                    || action_label(&interaction.action).to_owned(),
                    |choice| choice.label.to_owned(),
                ),
                resource_node_id,
                resource_action,
                cache_action: cache_choice.as_ref().map(|choice| choice.action),
                cache_resource: cache_choice.and_then(|choice| choice.resource),
            }
        })
}

pub(crate) fn property_touch_command(
    projection: &tarrowyn_protocol::FoundationPropertyProjection,
    player: TilePos,
    tile: TilePos,
) -> Option<String> {
    property::touch_command(projection, player, tile)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NearbyFarmChoice {
    action: FarmingAction,
    label: &'static str,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NearbyForgeChoice {
    action: FoundationForgeAction,
    label: &'static str,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NearbyCooperationChoice {
    label: &'static str,
    detail: String,
    command: String,
}

fn nearby_cooperation_choice(
    activity: &FoundationActivityState,
    inventory: Option<&Inventory>,
    own_account_id: Option<&str>,
    players: &[crate::network::RemotePlayer],
    trades: &[TradeOffer],
    server_tick: u64,
) -> Option<NearbyCooperationChoice> {
    let own_account_id = own_account_id?;
    let inventory = inventory?;
    let goal = &activity.cooperation.goal;
    let required_ore = goal
        .required_inputs
        .iter()
        .find(|material| material.kind == tarrowyn_protocol::FoundationForgeMaterialKind::IronOre)
        .map_or(0, |material| material.amount);
    let required_timber = goal
        .required_inputs
        .iter()
        .find(|material| material.kind == tarrowyn_protocol::FoundationForgeMaterialKind::Timber)
        .map_or(0, |material| material.amount);
    let credited = |account_id: &str,
                    kind: tarrowyn_protocol::FoundationCooperationWorkKind,
                    material_kind: tarrowyn_protocol::FoundationForgeMaterialKind,
                    amount: u32| {
        activity.cooperation.recent_work.iter().any(|credit| {
            credit.account_id == account_id
                && credit.kind == kind
                && credit
                    .materials
                    .iter()
                    .any(|material| material.kind == material_kind && material.amount >= amount)
        })
    };
    let timber_credited = credited(
        own_account_id,
        tarrowyn_protocol::FoundationCooperationWorkKind::Log,
        tarrowyn_protocol::FoundationForgeMaterialKind::Timber,
        required_timber,
    );
    if inventory.timber >= required_timber && timber_credited {
        if let Some(trade) = trades.iter().find(|trade| {
            trade.status == TradeStatus::Pending
                && trade.recipient_account_id == own_account_id
                && trade.offer.iron_ore >= required_ore
                && credited(
                    &trade.creator_account_id,
                    tarrowyn_protocol::FoundationCooperationWorkKind::Mine,
                    tarrowyn_protocol::FoundationForgeMaterialKind::IronOre,
                    required_ore,
                )
        }) {
            return Some(NearbyCooperationChoice {
                label: "Accept 2 ore",
                detail: format!(
                    "Accept {}'s ore, then make charcoal, a handle, and the tool: {} actions together vs {} solo.",
                    trade.creator_name,
                    goal.cooperative_target_work_actions,
                    goal.solo_work_actions
                ),
                command: format!("cooperation-accept-ore:{}", trade.trade_id),
            });
        }
    }
    if let Some(trade) = trades.iter().find(|trade| {
        trade.status == TradeStatus::Pending
            && trade.creator_account_id == own_account_id
            && trade.offer.iron_ore >= required_ore
    }) {
        return Some(NearbyCooperationChoice {
            label: "Review ore offer",
            detail: format!(
                "{} has your {}-ore offer. The measured target is {} actions together vs {} solo.",
                trade.recipient_name,
                required_ore,
                goal.cooperative_target_work_actions,
                goal.solo_work_actions
            ),
            command: format!("cooperation-review-ore:{}", trade.trade_id),
        });
    }
    if inventory.iron_ore < required_ore
        || required_ore == 0
        || !credited(
            own_account_id,
            tarrowyn_protocol::FoundationCooperationWorkKind::Mine,
            tarrowyn_protocol::FoundationForgeMaterialKind::IronOre,
            required_ore,
        )
    {
        return None;
    }
    let target = players
        .iter()
        .find(|player| player.account_id != own_account_id && !player.stale(server_tick))?;
    Some(NearbyCooperationChoice {
        label: "Offer 2 ore",
        detail: format!(
            "Send {} ore to {} for the First Beacon tool: {} actions together vs {} solo.",
            required_ore,
            target.display_name,
            goal.cooperative_target_work_actions,
            goal.solo_work_actions
        ),
        command: format!("cooperation-offer-ore:{}", target.account_id),
    })
}

fn cooperation_detail(activity: &FoundationActivityState, own_account_id: Option<&str>) -> String {
    let cooperation = &activity.cooperation;
    if let Some(result) = cooperation.latest_result.as_ref() {
        return format!(
            "First Beacon tool: {}/{} accepted actions; {} saved through barter.",
            result.work_actions, cooperation.goal.solo_work_actions, result.saved_work_actions
        );
    }
    if let Some(attempt) = own_account_id.and_then(|account_id| {
        cooperation
            .active_attempts
            .iter()
            .find(|attempt| attempt.coordinator_account_id == account_id)
    }) {
        return format!(
            "Ore received: {}/{} actions. Make charcoal, handle, then iron tool; solo needs {}.",
            attempt.work_actions,
            cooperation.goal.cooperative_target_work_actions,
            cooperation.goal.solo_work_actions
        );
    }
    format!(
        "Goal: 2 timber + 2 ore -> iron tool. Solo {} actions; practiced-miner partnership {}. Solo fallback open.",
        cooperation.goal.solo_work_actions, cooperation.goal.cooperative_target_work_actions
    )
}

fn nearby_forge_choice(
    inventory: Option<&Inventory>,
    field_tool_kind: Option<FoundationFieldToolKind>,
    field_tool_condition: Option<u8>,
) -> NearbyForgeChoice {
    let inventory = inventory.cloned().unwrap_or_default();
    let kind = field_tool_kind.unwrap_or_default();
    let condition = field_tool_condition.unwrap_or(0);
    let ready = inventory.iron_ore >= 2
        && inventory.charcoal >= 1
        && inventory.tool_handles >= 1
        && (kind != FoundationFieldToolKind::Iron
            || condition < FoundationFieldToolKind::Iron.max_condition());
    let (action, label) = if ready {
        (
            FoundationForgeAction::ForgeFieldTool,
            "Forge iron field tool",
        )
    } else if inventory.charcoal == 0 && inventory.timber > 0 {
        (FoundationForgeAction::BurnCharcoal, "Burn charcoal")
    } else if inventory.tool_handles == 0 && inventory.timber > 0 {
        (FoundationForgeAction::ShapeHandle, "Shape tool handle")
    } else {
        (FoundationForgeAction::Inspect, "Inspect forge")
    };
    let need = if kind == FoundationFieldToolKind::Iron
        && condition == FoundationFieldToolKind::Iron.max_condition()
    {
        "Iron tool is ready for 6 field actions.".to_owned()
    } else {
        format!(
            "Iron tool needs 2 ore + 1 charcoal + 1 handle; missing {} ore, {} charcoal, {} handle.",
            2_u32.saturating_sub(inventory.iron_ore),
            1_u32.saturating_sub(inventory.charcoal),
            1_u32.saturating_sub(inventory.tool_handles)
        )
    };
    NearbyForgeChoice {
        action,
        label,
        detail: format!(
            "Materials: {} timber, {} ore, {} charcoal, {} handles. {} {}/{}. {need}",
            inventory.timber,
            inventory.iron_ore,
            inventory.charcoal,
            inventory.tool_handles,
            kind.label(),
            condition,
            kind.max_condition()
        ),
    }
}

fn nearby_farm_choice(
    world: &crate::state::WorldState,
    player: TilePos,
    field_tool_condition: Option<u8>,
    field_weather: Option<FieldWeather>,
    field_pest_pressure: Option<u8>,
) -> Option<NearbyFarmChoice> {
    let (_, _, _, _, crop) = world
        .tiles
        .iter_with_pos()
        .filter_map(|(position, tile)| {
            let distance = position.manhattan_distance(&player);
            if *tile != crate::state::TileKind::Field || distance > 1 {
                return None;
            }
            let crop = world.crops.get(position).copied().flatten();
            let priority = match crop {
                Some(crop) if crop.mature() => 0,
                Some(_) => 1,
                None => 2,
            };
            Some((distance, priority, position.y, position.x, crop))
        })
        .min_by_key(|(distance, priority, y, x, _)| (*distance, *priority, *y, *x))?;
    let conditions = format!(
        "Tool {}/3; {}; pests {}/2.",
        field_tool_condition.unwrap_or(0),
        field_weather.unwrap_or_default().label(),
        field_pest_pressure.unwrap_or(0)
    );
    Some(match crop {
        Some(crop) if crop.mature() => NearbyFarmChoice {
            action: FarmingAction::Harvest,
            label: "Harvest crop",
            detail: format!(
                "{} is ready to harvest (stage 3/3). {conditions}",
                crop_name(crop.kind)
            ),
        },
        Some(crop) => NearbyFarmChoice {
            action: FarmingAction::Tend,
            label: "Tend / water",
            detail: format!(
                "{} stage {}/3. Tend/water improves yield (optional). {conditions}",
                crop_name(crop.kind),
                crop.stage
            ),
        },
        None => NearbyFarmChoice {
            action: FarmingAction::Plant,
            label: "Plant crop",
            detail: format!("Empty shared plot; planting uses one seed. {conditions}"),
        },
    })
}

fn crop_name(kind: crate::state::CropKind) -> &'static str {
    match kind {
        crate::state::CropKind::Wheat => "Wheat",
        crate::state::CropKind::Turnip => "Turnip",
        crate::state::CropKind::Moonberry => "Moonberry",
    }
}

struct SharedCacheChoice {
    action: FoundationCacheAction,
    resource: Option<FoundationResourceKind>,
    label: &'static str,
}

fn shared_cache_choice(
    activity: &FoundationActivityState,
    inventory: Option<&Inventory>,
) -> Option<SharedCacheChoice> {
    let cache = &activity.shared_cache;
    let has_room = cache.inventory.total_items() < cache.capacity;
    if has_room {
        if let Some(resource) = inventory.and_then(first_material) {
            return Some(SharedCacheChoice {
                action: FoundationCacheAction::Deposit,
                resource: Some(resource),
                label: match resource {
                    FoundationResourceKind::Timber => "Store timber",
                    FoundationResourceKind::Stone => "Store stone",
                    FoundationResourceKind::IronOre => "Store iron ore",
                },
            });
        }
    }
    if let Some(resource) = first_material(&cache.inventory) {
        return Some(SharedCacheChoice {
            action: FoundationCacheAction::Withdraw,
            resource: Some(resource),
            label: match resource {
                FoundationResourceKind::Timber => "Collect timber",
                FoundationResourceKind::Stone => "Collect stone",
                FoundationResourceKind::IronOre => "Collect iron ore",
            },
        });
    }
    Some(SharedCacheChoice {
        action: FoundationCacheAction::Inspect,
        resource: None,
        label: "Inspect cache",
    })
}

fn first_material(inventory: &Inventory) -> Option<FoundationResourceKind> {
    if inventory.timber > 0 {
        Some(FoundationResourceKind::Timber)
    } else if inventory.stone > 0 {
        Some(FoundationResourceKind::Stone)
    } else if inventory.iron_ore > 0 {
        Some(FoundationResourceKind::IronOre)
    } else {
        None
    }
}

fn action_label(action: &str) -> &'static str {
    match action {
        "arrive_or_travel" => "Inspect beacon",
        "inspect_shelter" => "Inspect tents",
        "gather" => "Warm by fire",
        "speak_or_request_construction" => "Talk to Mara",
        "read_needs" => "Read local need",
        "deposit_or_collect" => "Inspect cache",
        "borrow_crude_tool" => "Use crude tools",
        "farm" => "Inspect fields",
        "log" => "Gather timber",
        "mine" => "Mine stone",
        "smith" => "Inspect forge",
        "inspect_or_contribute" => "Inspect site",
        _ => "Inspect",
    }
}

pub(super) fn draw_context_deck(
    ctx: &UiContext<'_>,
    dock: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    let context = nearby_context(
        ctx.foundation,
        ctx.foundation_activity,
        ctx.player_position,
        ctx.player_inventory,
    );
    let farm_choice = match context.as_ref() {
        Some(context) if context.interaction_action != "farm" => None,
        _ => nearby_farm_choice(
            ctx.world,
            ctx.player_position,
            ctx.field_tool_condition,
            ctx.field_weather,
            ctx.field_pest_pressure,
        ),
    };
    let forge_choice = context
        .as_ref()
        .filter(|context| context.interaction_action == "smith")
        .map(|_| {
            nearby_forge_choice(
                ctx.player_inventory,
                ctx.field_tool_kind,
                ctx.field_tool_condition,
            )
        });
    let storehouse_choice = nearby_storehouse_choice(
        context.as_ref(),
        &ctx.foundation_activity.storehouse,
        ctx.player_inventory,
        ctx.player_gold,
    );
    let property_choice = nearby_property_choice(
        ctx.property,
        ctx.foundation,
        ctx.player_position,
        ctx.own_account_id,
        ctx.player_inventory,
        ctx.player_gold,
        context.is_none() && farm_choice.is_none(),
    );
    let cooperation_choice = storehouse_choice
        .is_none()
        .then(|| {
            nearby_cooperation_choice(
                ctx.foundation_activity,
                ctx.player_inventory,
                ctx.own_account_id,
                ctx.remote_players,
                ctx.trades,
                ctx.server_tick,
            )
        })
        .flatten();
    draw_ui_text_ex(
        "NEARBY",
        18.0,
        dock.y + 14.0,
        TextStyle::new(8.0, MINT).params(),
    );

    let name = match (
        property_choice.as_ref(),
        storehouse_choice.as_ref(),
        cooperation_choice.as_ref(),
        context.as_ref(),
        farm_choice.as_ref(),
    ) {
        (Some(choice), _, _, _, _) => choice.name.as_str(),
        (_, Some(_), _, Some(context), _) => context.landmark.name.as_str(),
        (_, _, Some(_), _, _) => ctx.foundation_activity.cooperation.goal.title.as_str(),
        (_, _, None, Some(context), _) => context.landmark.name.as_str(),
        (_, _, None, None, Some(_)) => "Shared fields",
        (_, _, None, None, None) => "First Beacon camp",
    };
    let cooperation_relevant = cooperation_choice.is_some()
        || context.as_ref().is_some_and(|context| {
            matches!(
                context.interaction_action,
                "arrive_or_travel" | "borrow_crude_tool" | "log" | "mine" | "smith"
            )
        });
    let cooperation_status = cooperation_relevant
        .then(|| cooperation_detail(ctx.foundation_activity, ctx.own_account_id));
    let detail = property_choice
        .as_ref()
        .map(|choice| choice.detail.as_str())
        .or_else(|| {
            storehouse_choice
                .as_ref()
                .map(|choice| choice.detail.as_str())
        })
        .or_else(|| {
            cooperation_choice
                .as_ref()
                .map(|choice| choice.detail.as_str())
        })
        .or(cooperation_status.as_deref())
        .or_else(|| farm_choice.as_ref().map(|choice| choice.detail.as_str()))
        .or_else(|| forge_choice.as_ref().map(|choice| choice.detail.as_str()))
        .unwrap_or_else(|| {
            context.as_ref().map_or(
                "Tap the road to walk. Find MARA or the NOTICEBOARD.",
                |context| context.landmark.note.as_str(),
            )
        });
    let guidance = property_choice
        .is_none()
        .then(|| journey_guidance(ctx.journey))
        .flatten();
    draw_ui_text_ex(
        &format!(
            "{}  •  {}",
            name.to_ascii_uppercase(),
            ellipsize(guidance.as_deref().unwrap_or(detail), 92)
        ),
        18.0,
        dock.y + 43.0,
        TextStyle::new(10.0, CREAM).params(),
    );

    if let Some(choice) = property_choice.as_ref() {
        draw_property_controls(ctx, dock, mouse, actions, choice);
    } else if let Some(choice) = storehouse_choice
        .as_ref()
        .filter(|choice| choice.contribution_controls)
    {
        draw_storehouse_controls(ctx, dock, mouse, actions, choice);
    } else if cooperation_choice.is_some() || context.is_some() || farm_choice.is_some() {
        let label = cooperation_choice
            .as_ref()
            .map(|choice| choice.label)
            .or_else(|| farm_choice.as_ref().map(|choice| choice.label))
            .or_else(|| forge_choice.as_ref().map(|choice| choice.label))
            .unwrap_or_else(|| {
                context
                    .as_ref()
                    .expect("context exists")
                    .action_label
                    .as_str()
            });
        let enabled = ctx.connection == ConnectionState::Online
            && ctx.player_position_authoritative
            && if cooperation_choice.is_some() {
                !ctx.trade_pending
            } else if farm_choice.is_some() {
                !ctx.farming_pending
            } else {
                !ctx.foundation_interaction_pending
            };
        if super::virtual_button(
            Rect::new(930.0, dock.y + 18.0, 190.0, 32.0),
            label,
            enabled,
            ButtonTone::Positive,
            mouse,
        ) {
            let command = if let Some(choice) = cooperation_choice.as_ref() {
                choice.command.clone()
            } else if let Some(choice) = farm_choice.as_ref() {
                match choice.action {
                    FarmingAction::Plant => "plant".to_owned(),
                    FarmingAction::Tend => "tend".to_owned(),
                    FarmingAction::Harvest => "harvest".to_owned(),
                    FarmingAction::TendAnimal => {
                        unreachable!("field choice does not target animals")
                    }
                }
            } else if let Some(choice) = forge_choice.as_ref() {
                foundation_forge_command(choice.action)
            } else {
                let context = context.as_ref().expect("context exists");
                match (
                    context.resource_node_id,
                    context.resource_action,
                    context.cache_action,
                ) {
                    (Some(node_id), Some(FoundationResourceAction::Log), _) => {
                        format!("foundation-resource:{node_id}:log")
                    }
                    (Some(node_id), Some(FoundationResourceAction::Mine), _) => {
                        format!("foundation-resource:{node_id}:mine")
                    }
                    (_, _, Some(action)) => {
                        foundation_cache_command(action, context.cache_resource)
                    }
                    _ => format!("foundation:{}", context.interaction_id),
                }
            };
            actions.push(UiAction::Interact(command));
        }
    }
    if super::virtual_button(
        Rect::new(1132.0, dock.y + 18.0, 130.0, 32.0),
        "All tools",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("menu-toggle".to_owned()));
    }
}

pub(crate) fn journey_guidance(journey: Option<&FoundationJourneyProjection>) -> Option<String> {
    let journey = journey?;
    if let Some(next) = &journey.next_milestone {
        return Some(format!(
            "NEXT {}/{}: {} — {}",
            journey.completed_milestones.saturating_add(1),
            journey.total_milestones,
            next.title,
            next.direction
        ));
    }
    match journey.progress.future_goal_state {
        FoundationJourneyFutureGoalState::Active => Some(format!(
            "RETURN GOAL: {} — {}",
            journey.contract.future_goal.title, journey.next_action
        )),
        FoundationJourneyFutureGoalState::Complete => Some(format!(
            "JOURNEY {}/{}: Return goal complete",
            journey.completed_milestones, journey.total_milestones
        )),
        FoundationJourneyFutureGoalState::Locked => Some(format!(
            "JOURNEY {}/{}: {}",
            journey.completed_milestones, journey.total_milestones, journey.next_action
        )),
    }
}

fn foundation_forge_command(action: FoundationForgeAction) -> String {
    let action = match action {
        FoundationForgeAction::Inspect => "inspect",
        FoundationForgeAction::BurnCharcoal => "burn-charcoal",
        FoundationForgeAction::ShapeHandle => "shape-handle",
        FoundationForgeAction::ForgeFieldTool => "forge-field-tool",
    };
    format!("foundation-forge:{action}")
}

fn foundation_cache_command(
    action: FoundationCacheAction,
    resource: Option<FoundationResourceKind>,
) -> String {
    let action = match action {
        FoundationCacheAction::Inspect => "inspect",
        FoundationCacheAction::Deposit => "deposit",
        FoundationCacheAction::Withdraw => "withdraw",
    };
    let resource = match resource {
        Some(FoundationResourceKind::Timber) => "timber",
        Some(FoundationResourceKind::Stone) => "stone",
        Some(FoundationResourceKind::IronOre) => "iron-ore",
        None => "none",
    };
    format!("foundation-cache:{action}:{resource}")
}

fn ellipsize(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let compact: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{compact}…")
    } else {
        compact
    }
}

#[cfg(test)]
#[path = "ui_foundation/tests.rs"]
mod tests;
