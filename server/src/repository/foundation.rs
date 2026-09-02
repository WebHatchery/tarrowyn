//! Persistent foundational gathering state and deterministic renewal.

use super::models::RepositoryState;
use tarrowyn_protocol::{
    FoundationActivityState, FoundationCrudeToolKind, FoundationResourceDeposit,
    FoundationResourceKind, FoundationResourceNode, FoundationToolAccess,
};

const RESOURCE_RECOVERY_INTERVAL_TICKS: u64 = 6;

pub(super) fn fresh() -> FoundationActivityState {
    FoundationActivityState {
        resource_nodes: vec![
            FoundationResourceNode {
                node_id: "whisperwood-edge-node".to_owned(),
                landmark_id: "whisperwood-edge".to_owned(),
                deposits: vec![deposit(FoundationResourceKind::Timber, 12)],
                recovery_interval_ticks: RESOURCE_RECOVERY_INTERVAL_TICKS,
                last_recovered_tick: 0,
            },
            FoundationResourceNode {
                node_id: "shallow-stone-seam-node".to_owned(),
                landmark_id: "first-beacon-mine".to_owned(),
                deposits: vec![
                    deposit(FoundationResourceKind::Stone, 10),
                    deposit(FoundationResourceKind::IronOre, 4),
                ],
                recovery_interval_ticks: RESOURCE_RECOVERY_INTERVAL_TICKS,
                last_recovered_tick: 0,
            },
        ],
        crude_tool_access: vec![FoundationToolAccess {
            landmark_id: "first-beacon-tool-rack".to_owned(),
            tools: vec![
                FoundationCrudeToolKind::HandAxe,
                FoundationCrudeToolKind::StonePick,
            ],
            available_to_all: true,
        }],
    }
}

fn deposit(kind: FoundationResourceKind, capacity: u32) -> FoundationResourceDeposit {
    FoundationResourceDeposit {
        kind,
        remaining: capacity,
        capacity,
    }
}

pub(super) fn recover_resource_nodes(state: &mut RepositoryState) {
    for node in &mut state.foundation_activity.resource_nodes {
        let interval = node.recovery_interval_ticks.max(1);
        let elapsed = state.tick.saturating_sub(node.last_recovered_tick);
        let cycles = elapsed / interval;
        if cycles == 0 {
            continue;
        }
        let recovered = u32::try_from(cycles).unwrap_or(u32::MAX);
        for deposit in &mut node.deposits {
            deposit.remaining = deposit
                .remaining
                .saturating_add(recovered)
                .min(deposit.capacity);
        }
        node.last_recovered_tick = node
            .last_recovered_tick
            .saturating_add(cycles.saturating_mul(interval));
    }
}

#[cfg(test)]
mod tests;
