use super::{RepositoryState, ServerConfig};
use std::collections::{HashSet, VecDeque};
use tarrowyn_protocol::{
    FoundationPropertyAction, FoundationPropertyContract, FoundationPropertyDirection,
    FoundationPropertyFootprint, FoundationPropertyPlacementPreview,
    FoundationPropertyPlacementRule, FoundationPropertyRequest, FoundationPropertyStage,
    FoundationPropertyStageDefinition, Position, TileKind,
};

pub(super) fn preview_placement(
    state: &RepositoryState,
    identity_key: &str,
    request: &FoundationPropertyRequest,
    config: &ServerConfig,
    ignored_property_id: Option<&str>,
) -> FoundationPropertyPlacementPreview {
    preview_for_footprint(
        state,
        identity_key,
        request,
        config,
        FoundationPropertyContract::default().stages[0].footprint,
        ignored_property_id,
    )
}

pub(super) fn preview_for_footprint(
    state: &RepositoryState,
    identity_key: &str,
    request: &FoundationPropertyRequest,
    config: &ServerConfig,
    footprint: FoundationPropertyFootprint,
    ignored_property_id: Option<&str>,
) -> FoundationPropertyPlacementPreview {
    let anchor = request
        .anchor
        .unwrap_or(state.identities[identity_key].position);
    let entrance = request
        .entrance
        .unwrap_or(FoundationPropertyDirection::South);
    let tiles = footprint_tiles(anchor, footprint);
    let entrance_tile = entrance_position(anchor, footprint, entrance);
    let mut rejected = Vec::new();
    if tiles.iter().any(|position| {
        super::super::super::world::position_in_world(
            *position,
            config.world_width,
            config.world_height,
        )
        .is_none()
    }) {
        rejected.push(FoundationPropertyPlacementRule::InsideWorld);
    }
    if tiles.iter().any(|position| {
        super::super::super::world::tile_at(*position, config.world_width, config.world_height)
            != TileKind::Meadow
    }) {
        rejected.push(FoundationPropertyPlacementRule::ClearTerrain);
    }
    if tiles
        .iter()
        .any(|position| occupied(state, *position, ignored_property_id, true))
    {
        rejected.push(FoundationPropertyPlacementRule::NoStructureOverlap);
    }
    let beacon = Position { x: 8, y: 6 };
    let radius = u32::from(
        FoundationPropertyContract::default()
            .placement
            .beacon_commons_radius,
    );
    if tiles
        .iter()
        .chain(std::iter::once(&entrance_tile))
        .any(|position| position.manhattan_distance(beacon) <= radius)
    {
        rejected.push(FoundationPropertyPlacementRule::OutsideBeaconCommons);
    }
    if tiles.iter().any(|position| {
        super::super::super::world::tile_at(*position, config.world_width, config.world_height)
            == TileKind::Path
    }) {
        rejected.push(FoundationPropertyPlacementRule::OutsideProtectedRoute);
    }
    if super::super::super::world::position_in_world(
        entrance_tile,
        config.world_width,
        config.world_height,
    )
    .is_none()
        || !super::super::super::world::tile_at(
            entrance_tile,
            config.world_width,
            config.world_height,
        )
        .is_walkable()
        || occupied(state, entrance_tile, ignored_property_id, false)
    {
        rejected.push(FoundationPropertyPlacementRule::EntranceClear);
    }
    if tiles.contains(&state.identities[identity_key].position)
        || !escape_path_exists(state, entrance_tile, &tiles, ignored_property_id, config)
        || state.identities.values().any(|identity| {
            !tiles.contains(&identity.position)
                && open_neighbour_count(
                    state,
                    identity.position,
                    &tiles,
                    ignored_property_id,
                    config,
                ) == 0
        })
    {
        rejected.push(FoundationPropertyPlacementRule::EscapePathOpen);
    }
    if state.identities[identity_key]
        .position
        .manhattan_distance(anchor)
        > 1
        && request.action == FoundationPropertyAction::PlaceTent
    {
        rejected.push(FoundationPropertyPlacementRule::EntranceClear);
    }
    rejected.sort_by_key(|rule| *rule as u8);
    rejected.dedup();
    let accepted = rejected.is_empty();
    FoundationPropertyPlacementPreview {
        anchor,
        entrance,
        footprint,
        accepted,
        rejected_rules: rejected.clone(),
        message: if accepted {
            "This clear ground keeps the commons, road, entrance, and escape paths open.".to_owned()
        } else {
            format!(
                "Placement is blocked by {}.",
                placement_rule_labels(&rejected)
            )
        },
    }
}

pub(super) fn footprint_tiles(
    anchor: Position,
    footprint: FoundationPropertyFootprint,
) -> Vec<Position> {
    (0..footprint.height)
        .flat_map(|dy| {
            (0..footprint.width).map(move |dx| Position {
                x: anchor.x.saturating_add(i32::from(dx)),
                y: anchor.y.saturating_add(i32::from(dy)),
            })
        })
        .collect()
}

pub(super) fn entrance_position(
    anchor: Position,
    footprint: FoundationPropertyFootprint,
    direction: FoundationPropertyDirection,
) -> Position {
    match direction {
        FoundationPropertyDirection::North => Position {
            x: anchor.x.saturating_add(i32::from(footprint.width / 2)),
            y: anchor.y.saturating_sub(1),
        },
        FoundationPropertyDirection::East => Position {
            x: anchor.x.saturating_add(i32::from(footprint.width)),
            y: anchor.y.saturating_add(i32::from(footprint.height / 2)),
        },
        FoundationPropertyDirection::South => Position {
            x: anchor.x.saturating_add(i32::from(footprint.width / 2)),
            y: anchor.y.saturating_add(i32::from(footprint.height)),
        },
        FoundationPropertyDirection::West => Position {
            x: anchor.x.saturating_sub(1),
            y: anchor.y.saturating_add(i32::from(footprint.height / 2)),
        },
    }
}

fn occupied(
    state: &RepositoryState,
    position: Position,
    ignored_property_id: Option<&str>,
    include_players: bool,
) -> bool {
    crate::content::foundation_baseline()
        .landmarks
        .iter()
        .any(|landmark| landmark.position == position)
        || state.plots.iter().any(|plot| plot.position == position)
        || state
            .phase4
            .infrastructure
            .iter()
            .any(|record| record.position == position)
        || state.foundation_properties.iter().any(|property| {
            Some(property.property_id.as_str()) != ignored_property_id
                && footprint_tiles(property.anchor, stage_definition(property.stage).footprint)
                    .contains(&position)
        })
        || (include_players
            && state
                .identities
                .values()
                .any(|identity| identity.position == position))
}

fn escape_path_exists(
    state: &RepositoryState,
    start: Position,
    proposed: &[Position],
    ignored_property_id: Option<&str>,
    config: &ServerConfig,
) -> bool {
    let mut queue = VecDeque::from([start]);
    let mut visited = HashSet::from([(start.x, start.y)]);
    while let Some(position) = queue.pop_front() {
        if super::super::super::world::tile_at(position, config.world_width, config.world_height)
            == TileKind::Path
        {
            return true;
        }
        for next in neighbours(position) {
            if visited.insert((next.x, next.y))
                && !proposed.contains(&next)
                && super::super::super::world::position_in_world(
                    next,
                    config.world_width,
                    config.world_height,
                )
                .is_some()
                && super::super::super::world::tile_at(
                    next,
                    config.world_width,
                    config.world_height,
                )
                .is_walkable()
                && !occupied(state, next, ignored_property_id, false)
            {
                queue.push_back(next);
            }
        }
    }
    false
}

fn open_neighbour_count(
    state: &RepositoryState,
    position: Position,
    proposed: &[Position],
    ignored_property_id: Option<&str>,
    config: &ServerConfig,
) -> usize {
    neighbours(position)
        .into_iter()
        .filter(|next| {
            !proposed.contains(next)
                && super::super::super::world::position_in_world(
                    *next,
                    config.world_width,
                    config.world_height,
                )
                .is_some()
                && super::super::super::world::tile_at(
                    *next,
                    config.world_width,
                    config.world_height,
                )
                .is_walkable()
                && !occupied(state, *next, ignored_property_id, false)
        })
        .count()
}

fn neighbours(position: Position) -> [Position; 4] {
    [
        Position {
            x: position.x.saturating_add(1),
            y: position.y,
        },
        Position {
            x: position.x.saturating_sub(1),
            y: position.y,
        },
        Position {
            x: position.x,
            y: position.y.saturating_add(1),
        },
        Position {
            x: position.x,
            y: position.y.saturating_sub(1),
        },
    ]
}

pub(super) fn stage_definition(
    stage: FoundationPropertyStage,
) -> FoundationPropertyStageDefinition {
    FoundationPropertyContract::default()
        .stages
        .into_iter()
        .find(|definition| definition.stage == stage)
        .expect("complete property contract")
}

fn placement_rule_labels(rules: &[FoundationPropertyPlacementRule]) -> String {
    rules
        .iter()
        .map(|rule| match rule {
            FoundationPropertyPlacementRule::InsideWorld => "the settlement edge",
            FoundationPropertyPlacementRule::ClearTerrain => "unclear terrain",
            FoundationPropertyPlacementRule::NoStructureOverlap => "an occupied space",
            FoundationPropertyPlacementRule::OutsideBeaconCommons => "the public Beacon commons",
            FoundationPropertyPlacementRule::OutsideProtectedRoute => "a protected road",
            FoundationPropertyPlacementRule::EntranceClear => "a blocked entrance",
            FoundationPropertyPlacementRule::EscapePathOpen => "an unsafe escape path",
        })
        .collect::<Vec<_>>()
        .join(", ")
}
