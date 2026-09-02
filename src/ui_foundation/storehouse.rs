use super::*;
use tarrowyn_protocol::FoundationStorehouseState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StorehouseTouchAction {
    pub(super) label: String,
    pub(super) command: String,
    pub(super) enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NearbyStorehouseChoice {
    pub(super) detail: String,
    pub(super) material: Option<StorehouseTouchAction>,
    pub(super) gold: Option<StorehouseTouchAction>,
    pub(super) inspect_command: String,
    pub(super) operational: bool,
    pub(super) contribution_controls: bool,
}

pub(super) fn nearby_choice(
    context: Option<&FoundationContext<'_>>,
    project: &FoundationStorehouseState,
    inventory: Option<&Inventory>,
    gold: Option<u32>,
) -> Option<NearbyStorehouseChoice> {
    let context = context?;
    let contribution_controls = matches!(
        context.interaction_action,
        "speak_or_request_construction" | "inspect_or_contribute"
    );
    if !contribution_controls && context.interaction_action != "read_needs" {
        return None;
    }
    let credited = |kind| {
        project
            .contributions
            .iter()
            .filter(|contribution| contribution.credited_kind == kind)
            .fold(0_u32, |total, contribution| {
                total.saturating_add(contribution.credited_units)
            })
    };
    let requirement = |kind| {
        project
            .requirements
            .iter()
            .find(|requirement| requirement.kind == kind)
    };
    let timber_required =
        requirement(FoundationResourceKind::Timber).map_or(0, |r| r.units_required);
    let stone_required = requirement(FoundationResourceKind::Stone).map_or(0, |r| r.units_required);
    let timber_credited = credited(FoundationResourceKind::Timber);
    let stone_credited = credited(FoundationResourceKind::Stone);
    let remaining = |kind| match kind {
        FoundationResourceKind::Timber => timber_required.saturating_sub(timber_credited),
        FoundationResourceKind::Stone => stone_required.saturating_sub(stone_credited),
        FoundationResourceKind::IronOre => 0,
    };
    let operational = project.completion.is_some();
    let inventory = inventory.cloned().unwrap_or_default();
    let material_kind = [
        FoundationResourceKind::Timber,
        FoundationResourceKind::Stone,
    ]
    .into_iter()
    .find(|kind| remaining(*kind) > 0 && inventory_material(&inventory, *kind) > 0);
    let material = material_kind.map(|kind| StorehouseTouchAction {
        label: format!("Give 1 {}", resource_name(kind)),
        command: format!(
            "foundation-storehouse:{}:material:{}:1",
            context.landmark.id,
            resource_command_name(kind)
        ),
        enabled: true,
    });
    let gold_kind = [
        FoundationResourceKind::Timber,
        FoundationResourceKind::Stone,
    ]
    .into_iter()
    .filter(|kind| remaining(*kind) > 0)
    .max_by_key(|kind| u8::from(Some(*kind) != material_kind));
    let gold_action = gold_kind.and_then(|kind| {
        let rate = requirement(kind)?.gold_per_unit;
        Some(StorehouseTouchAction {
            label: format!("Fund {} {rate}g", resource_name(kind)),
            command: format!(
                "foundation-storehouse:{}:gold:{}:{rate}",
                context.landmark.id,
                resource_command_name(kind)
            ),
            enabled: gold.unwrap_or(0) >= rate,
        })
    });
    let stage = project
        .stages
        .iter()
        .find(|gate| gate.stage == project.current_stage)
        .map(|gate| gate.visible_label.as_str())
        .unwrap_or("Marked storehouse site");
    let mut contributors = Vec::new();
    for contribution in &project.contributions {
        if !contributors.contains(&contribution.account_id) {
            contributors.push(contribution.account_id.clone());
        }
    }
    let recovery = if operational {
        "Tap Use storehouse to inspect the permanent public structure."
    } else if material.is_none() && gold_action.as_ref().is_none_or(|action| !action.enabled) {
        "Gather timber at the woodland, mine stone, or earn the exact gold substitute."
    } else {
        "Give a carried unit or fund one exact substitute; both advance the same ledger."
    };
    let guidance = if context.interaction_action == "speak_or_request_construction" {
        "Mara: "
    } else {
        ""
    };
    Some(NearbyStorehouseChoice {
        detail: format!(
            "{guidance}{stage}. Timber {timber_credited}/{timber_required}; stone {stone_credited}/{stone_required}; {} contributors. {recovery}",
            contributors.len()
        ),
        material,
        gold: gold_action,
        inspect_command: format!("foundation-storehouse:{}:inspect", context.landmark.id),
        operational,
        contribution_controls,
    })
}

pub(super) fn draw_controls(
    ctx: &UiContext<'_>,
    dock: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
    choice: &NearbyStorehouseChoice,
) {
    let ready = ctx.connection == ConnectionState::Online
        && ctx.player_position_authoritative
        && !ctx.foundation_interaction_pending;
    if choice.operational {
        if super::super::virtual_button(
            Rect::new(930.0, dock.y + 18.0, 190.0, 32.0),
            "Use storehouse",
            ready,
            ButtonTone::Positive,
            mouse,
        ) {
            actions.push(UiAction::Interact(choice.inspect_command.clone()));
        }
        return;
    }
    let material_label = choice
        .material
        .as_ref()
        .map(|action| action.label.as_str())
        .unwrap_or("No needed goods");
    let material_enabled = ready
        && choice
            .material
            .as_ref()
            .is_some_and(|action| action.enabled);
    if super::super::virtual_button(
        Rect::new(930.0, dock.y + 18.0, 92.0, 32.0),
        material_label,
        material_enabled,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::Interact(
            choice.material.as_ref().unwrap().command.clone(),
        ));
    }
    let gold_label = choice
        .gold
        .as_ref()
        .map(|action| action.label.as_str())
        .unwrap_or("Funding done");
    let gold_enabled = ready && choice.gold.as_ref().is_some_and(|action| action.enabled);
    if super::super::virtual_button(
        Rect::new(1028.0, dock.y + 18.0, 92.0, 32.0),
        gold_label,
        gold_enabled,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::Interact(
            choice.gold.as_ref().unwrap().command.clone(),
        ));
    }
}

fn inventory_material(inventory: &Inventory, kind: FoundationResourceKind) -> u32 {
    match kind {
        FoundationResourceKind::Timber => inventory.timber,
        FoundationResourceKind::Stone => inventory.stone,
        FoundationResourceKind::IronOre => inventory.iron_ore,
    }
}

fn resource_name(kind: FoundationResourceKind) -> &'static str {
    match kind {
        FoundationResourceKind::Timber => "timber",
        FoundationResourceKind::Stone => "stone",
        FoundationResourceKind::IronOre => "iron ore",
    }
}

fn resource_command_name(kind: FoundationResourceKind) -> &'static str {
    match kind {
        FoundationResourceKind::Timber => "timber",
        FoundationResourceKind::Stone => "stone",
        FoundationResourceKind::IronOre => "iron-ore",
    }
}
