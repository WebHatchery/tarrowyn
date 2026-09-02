use super::*;
use macroquad_toolkit::ui::draw_ui_text_ex;
use tarrowyn_protocol::{
    FoundationActivityState, FoundationBaseline, FoundationLandmark, FoundationResourceAction,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundationContext<'a> {
    pub landmark: &'a FoundationLandmark,
    pub interaction_id: &'a str,
    pub action_label: &'static str,
    pub resource_node_id: Option<&'a str>,
    pub resource_action: Option<FoundationResourceAction>,
}

pub(crate) fn nearby_context<'a>(
    baseline: &'a FoundationBaseline,
    activity: &'a FoundationActivityState,
    player: TilePos,
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
            FoundationContext {
                landmark,
                interaction_id: interaction.id.as_str(),
                action_label: action_label(&interaction.action),
                resource_node_id,
                resource_action,
            }
        })
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
    let context = nearby_context(ctx.foundation, ctx.foundation_activity, ctx.player_position);
    draw_ui_text_ex(
        "NEARBY",
        18.0,
        dock.y + 14.0,
        TextStyle::new(8.0, MINT).params(),
    );

    let (name, detail) = context.as_ref().map_or(
        (
            "First Beacon camp",
            "Tap the road to walk. Find MARA or the NOTICEBOARD.",
        ),
        |context| {
            (
                context.landmark.name.as_str(),
                context.landmark.note.as_str(),
            )
        },
    );
    draw_ui_text_ex(
        &format!(
            "{}  •  {}",
            name.to_ascii_uppercase(),
            ellipsize(detail, 74)
        ),
        18.0,
        dock.y + 43.0,
        TextStyle::new(10.0, CREAM).params(),
    );

    if let Some(context) = context {
        let enabled = ctx.connection == ConnectionState::Online
            && ctx.player_position_authoritative
            && !ctx.foundation_interaction_pending;
        if super::virtual_button(
            Rect::new(930.0, dock.y + 18.0, 190.0, 32.0),
            context.action_label,
            enabled,
            ButtonTone::Positive,
            mouse,
        ) {
            let command = match (context.resource_node_id, context.resource_action) {
                (Some(node_id), Some(FoundationResourceAction::Log)) => {
                    format!("foundation-resource:{node_id}:log")
                }
                (Some(node_id), Some(FoundationResourceAction::Mine)) => {
                    format!("foundation-resource:{node_id}:mine")
                }
                _ => format!("foundation:{}", context.interaction_id),
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
