//! Authoritative adventurer credentials derived from varied frontier work.

use super::models::RepositoryState;
use tarrowyn_protocol::{AdventurerRank, ExpeditionStatus};

pub(super) fn profile(state: &RepositoryState, key: &str) -> (AdventurerRank, Vec<String>) {
    let identity = state.identities.get(key).expect("identity exists");
    let completed_contracts = state
        .phase3
        .contracts
        .get(key)
        .map(|contract| contract.completion_count)
        .unwrap_or(0);
    let successful_expedition = state.phase3.expedition.as_ref().is_some_and(|expedition| {
        expedition.status == ExpeditionStatus::Succeeded
            && expedition
                .members
                .iter()
                .any(|member| member.account_id == identity.account_id)
    });
    let settled_standing = identity.reputation >= 3;
    let mut credentials = Vec::new();
    if completed_contracts > 0 {
        credentials.push("Brambleback watch report".to_owned());
    }
    if successful_expedition {
        credentials.push("Lantern Rest expedition".to_owned());
    }
    if settled_standing {
        credentials.push("Hearth standing".to_owned());
    }
    let rank = if completed_contracts >= 3 && successful_expedition && settled_standing {
        AdventurerRank::RoadWarden
    } else if completed_contracts > 0 && successful_expedition {
        AdventurerRank::Pathfinder
    } else if completed_contracts > 0 {
        AdventurerRank::Trailhand
    } else {
        AdventurerRank::Unproven
    };
    (rank, credentials)
}

#[cfg(test)]
mod tests;
