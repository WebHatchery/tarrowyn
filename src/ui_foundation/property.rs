use super::*;
use tarrowyn_protocol::{
    FoundationPropertyAccess, FoundationPropertyProjection, FoundationPropertyStage,
    FoundationResourceKind, Inventory,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PropertyTouchAction {
    pub(super) label: String,
    pub(super) command: String,
    pub(super) enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NearbyPropertyChoice {
    pub(super) name: String,
    pub(super) detail: String,
    pub(super) actions: Vec<PropertyTouchAction>,
}

pub(super) fn touch_command(
    projection: &FoundationPropertyProjection,
    player: TilePos,
    tile: TilePos,
) -> Option<String> {
    projection.properties.iter().find_map(|property| {
        let footprint = projection
            .contract
            .stages
            .iter()
            .find(|stage| stage.stage == property.stage)?
            .footprint;
        let contains = tile.x >= property.anchor.x
            && tile.y >= property.anchor.y
            && tile.x < property.anchor.x.saturating_add(i32::from(footprint.width))
            && tile.y
                < property
                    .anchor
                    .y
                    .saturating_add(i32::from(footprint.height));
        (contains && distance(player, tile.x, tile.y) <= 1)
            .then(|| format!("foundation-property:inspect:{}", property.property_id))
    })
}

pub(super) fn nearby_choice(
    projection: &FoundationPropertyProjection,
    baseline: &FoundationBaseline,
    player: TilePos,
    own_account_id: Option<&str>,
    inventory: Option<&Inventory>,
    gold: Option<u32>,
    allow_placement: bool,
) -> Option<NearbyPropertyChoice> {
    let inventory = inventory.copied().unwrap_or_default();
    if let Some(own) = projection.own_property.as_ref() {
        if own.stage != FoundationPropertyStage::House && near_builder(baseline, player) {
            let next = next_stage(projection, own.stage)?;
            let missing_gold = next.material_costs.iter().fold(0_u32, |sum, cost| {
                let missing = cost.amount.saturating_sub(amount(&inventory, cost.kind));
                sum.saturating_add(missing.saturating_mul(match cost.kind {
                    FoundationResourceKind::Timber => {
                        projection.contract.builder.timber_gold_per_unit
                    }
                    FoundationResourceKind::Stone => {
                        projection.contract.builder.stone_gold_per_unit
                    }
                    FoundationResourceKind::IronOre => 0,
                }))
            });
            return Some(NearbyPropertyChoice {
                name: "Mara's shelter work".to_owned(),
                detail: format!(
                    "{} can become {}. Mara supplies only missing materials for {missing_gold}g.",
                    stage_name(own.stage),
                    next.title
                ),
                actions: vec![action(
                    &format!("Build {missing_gold}g"),
                    &format!("foundation-property:builder:{}", own.property_id),
                    gold.unwrap_or(0) >= missing_gold,
                )],
            });
        }
    }
    let nearby = projection.properties.iter().find(|property| {
        let Some(footprint) = projection
            .contract
            .stages
            .iter()
            .find(|stage| stage.stage == property.stage)
            .map(|stage| stage.footprint)
        else {
            return false;
        };
        (0..footprint.height).any(|dy| {
            (0..footprint.width).any(|dx| {
                distance(
                    player,
                    property.anchor.x + i32::from(dx),
                    property.anchor.y + i32::from(dy),
                ) <= 1
            })
        })
    });
    if let Some(property) = nearby {
        let owner = own_account_id == Some(property.owner_account_id.as_str());
        let mut actions = vec![action(
            "Inspect",
            &format!("foundation-property:inspect:{}", property.property_id),
            true,
        )];
        if owner && property.condition < 100 {
            actions.push(action(
                "Maintain",
                &format!("foundation-property:maintain:{}", property.property_id),
                true,
            ));
        }
        if owner && property.stage != FoundationPropertyStage::House {
            let next = next_stage(projection, property.stage)?;
            let ready = next
                .material_costs
                .iter()
                .all(|cost| amount(&inventory, cost.kind) >= cost.amount);
            actions.push(action(
                "Improve",
                &format!("foundation-property:upgrade:{}", property.property_id),
                ready,
            ));
        }
        if owner {
            let access = if property.access == FoundationPropertyAccess::OwnerOnly {
                "guests"
            } else {
                "owner"
            };
            actions.push(action(
                "Access",
                &format!(
                    "foundation-property:access:{}:{access}",
                    property.property_id
                ),
                true,
            ));
        }
        if owner || property.access == FoundationPropertyAccess::GuestsAllowed {
            if let Some(kind) = first_material(&inventory) {
                actions.push(action(
                    "Store 1",
                    &format!(
                        "foundation-property:store:{}:{}:1",
                        property.property_id,
                        resource_command(kind)
                    ),
                    property.stored_units < property.storage_capacity,
                ));
            }
        }
        if owner {
            if let Some(own) = projection.own_property.as_ref() {
                if let Some(kind) = first_material(&own.storage) {
                    actions.push(action(
                        "Collect 1",
                        &format!(
                            "foundation-property:collect:{}:{}:1",
                            property.property_id,
                            resource_command(kind)
                        ),
                        true,
                    ));
                }
            }
        }
        actions.truncate(5);
        return Some(NearbyPropertyChoice {
            name: if owner {
                format!("Your {}", stage_name(property.stage))
            } else {
                format!("{}'s {}", property.owner_name, stage_name(property.stage))
            },
            detail: format!(
                "Condition {}%. Chest {}/{}. {}",
                property.condition,
                property.stored_units,
                property.storage_capacity,
                if owner {
                    "Owner controls are touchable here."
                } else {
                    "Guests may inspect and deposit only when opened."
                }
            ),
            actions,
        });
    }
    if projection.own_property.is_none() && allow_placement {
        let anchor_x = player.x.saturating_add(1);
        let anchor_y = player.y;
        let preview_ready = projection
            .placement_preview
            .as_ref()
            .is_some_and(|preview| {
                preview.anchor.x == anchor_x && preview.anchor.y == anchor_y && preview.accepted
            });
        let verb = if preview_ready { "place" } else { "preview" };
        return Some(NearbyPropertyChoice {
            name: "Personal tent site".to_owned(),
            detail: projection
                .placement_preview
                .as_ref()
                .map_or(
                    "Check the ground east of you before pitching a tent.",
                    |preview| preview.message.as_str(),
                )
                .to_owned(),
            actions: vec![action(
                if preview_ready {
                    "Pitch tent"
                } else {
                    "Check site"
                },
                &format!("foundation-property:{verb}:{anchor_x}:{anchor_y}:south"),
                true,
            )],
        });
    }
    None
}

pub(super) fn draw_controls(
    ctx: &UiContext<'_>,
    dock: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
    choice: &NearbyPropertyChoice,
) {
    let ready = ctx.connection == ConnectionState::Online
        && ctx.player_position_authoritative
        && !ctx.foundation_property_pending
        && !ctx.foundation_interaction_pending;
    let width = (350.0 / choice.actions.len().max(1) as f32).min(92.0);
    for (index, action) in choice.actions.iter().enumerate() {
        if super::super::virtual_button(
            Rect::new(
                760.0 + index as f32 * (width + 4.0),
                dock.y + 18.0,
                width,
                32.0,
            ),
            &action.label,
            ready && action.enabled,
            ButtonTone::Positive,
            mouse,
        ) {
            actions.push(UiAction::Interact(action.command.clone()));
        }
    }
}

fn action(label: &str, command: &str, enabled: bool) -> PropertyTouchAction {
    PropertyTouchAction {
        label: label.to_owned(),
        command: command.to_owned(),
        enabled,
    }
}

fn near_builder(baseline: &FoundationBaseline, player: TilePos) -> bool {
    baseline
        .landmarks
        .iter()
        .find(|landmark| landmark.id == "builder-mara")
        .is_some_and(|mara| distance(player, mara.position.x, mara.position.y) <= 1)
}

fn distance(player: TilePos, x: i32, y: i32) -> u32 {
    player.x.abs_diff(x).saturating_add(player.y.abs_diff(y))
}

fn next_stage(
    projection: &FoundationPropertyProjection,
    stage: FoundationPropertyStage,
) -> Option<&tarrowyn_protocol::FoundationPropertyStageDefinition> {
    let next = match stage {
        FoundationPropertyStage::Tent => FoundationPropertyStage::Camp,
        FoundationPropertyStage::Camp => FoundationPropertyStage::House,
        FoundationPropertyStage::House => return None,
    };
    projection
        .contract
        .stages
        .iter()
        .find(|definition| definition.stage == next)
}

fn stage_name(stage: FoundationPropertyStage) -> &'static str {
    match stage {
        FoundationPropertyStage::Tent => "tent",
        FoundationPropertyStage::Camp => "camp",
        FoundationPropertyStage::House => "house",
    }
}

fn amount(inventory: &Inventory, kind: FoundationResourceKind) -> u32 {
    match kind {
        FoundationResourceKind::Timber => inventory.timber,
        FoundationResourceKind::Stone => inventory.stone,
        FoundationResourceKind::IronOre => inventory.iron_ore,
    }
}

fn first_material(inventory: &Inventory) -> Option<FoundationResourceKind> {
    [
        FoundationResourceKind::Timber,
        FoundationResourceKind::Stone,
        FoundationResourceKind::IronOre,
    ]
    .into_iter()
    .find(|kind| amount(inventory, *kind) > 0)
}

fn resource_command(kind: FoundationResourceKind) -> &'static str {
    match kind {
        FoundationResourceKind::Timber => "timber",
        FoundationResourceKind::Stone => "stone",
        FoundationResourceKind::IronOre => "iron-ore",
    }
}
