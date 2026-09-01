use super::*;

pub(crate) fn travel_control_enabled(
    available: bool,
    knocked_out: bool,
    travel_pending: bool,
) -> bool {
    available && !knocked_out && !travel_pending
}

pub(crate) fn recovery_control_enabled(knocked_out: bool, recovery_pending: bool) -> bool {
    knocked_out && !recovery_pending
}

pub(crate) fn market_control_enabled(market_pending: bool) -> bool {
    !market_pending
}

pub(crate) fn trade_control_enabled(available: bool, trade_pending: bool) -> bool {
    available && !trade_pending
}

pub(crate) fn farming_control_enabled(available: bool, farming_pending: bool) -> bool {
    available && !farming_pending
}

pub(crate) fn cancel_market_control_enabled(
    has_open_market_order: bool,
    market_pending: bool,
) -> bool {
    has_open_market_order && !market_pending
}

pub(crate) fn event_control_enabled(event_pending: bool) -> bool {
    !event_pending
}

pub(crate) fn identity_control_enabled(identity_pending: bool) -> bool {
    !identity_pending
}

pub(crate) fn report_control_enabled(report_pending: bool) -> bool {
    !report_pending
}

pub(crate) fn claim_control_enabled(claim_available: bool, claim_pending: bool) -> bool {
    claim_available && !claim_pending
}

pub(crate) fn route_control_enabled(route_available: bool, route_pending: bool) -> bool {
    route_available && !route_pending
}

pub(crate) fn governance_control_enabled(governance_pending: bool) -> bool {
    !governance_pending
}

pub(crate) fn skill_control_enabled(available: bool, skill_pending: bool) -> bool {
    available && !skill_pending
}

pub(crate) fn knowledge_control_enabled(knowledge_pending: bool) -> bool {
    !knowledge_pending
}

pub(crate) fn order_control_enabled(available: bool, order_pending: bool) -> bool {
    available && !order_pending
}

pub(crate) fn combat_control_enabled(available: bool, combat_pending: bool) -> bool {
    available && !combat_pending
}

pub(crate) fn contract_control_enabled(available: bool, contract_pending: bool) -> bool {
    available && !contract_pending
}

pub(crate) fn expedition_control_enabled(available: bool, expedition_pending: bool) -> bool {
    available && !expedition_pending
}

pub(crate) fn frontier_combat_control_enabled(reachable: bool, combat_pending: bool) -> bool {
    reachable && !combat_pending
}

pub(crate) fn movement_enabled(ctx: &UiContext<'_>) -> bool {
    !ctx.knocked_out
        && walking_connection_enabled(ctx.connection)
        && walking_projection_enabled(ctx.player_position_authoritative)
        && !super::panels::regional_travel_blocks_movement(ctx.regional_region)
}

pub(crate) fn movement_tooltip(ctx: &UiContext<'_>) -> &'static str {
    movement_tooltip_for(
        ctx.connection,
        ctx.knocked_out,
        ctx.player_position_authoritative,
        super::panels::regional_travel_blocks_movement(ctx.regional_region),
    )
}

pub(crate) fn walking_connection_enabled(connection: ConnectionState) -> bool {
    connection == ConnectionState::Online
}

pub(crate) fn walking_projection_enabled(player_position_authoritative: bool) -> bool {
    player_position_authoritative
}

pub(crate) fn movement_tooltip_for(
    connection: ConnectionState,
    knocked_out: bool,
    player_position_authoritative: bool,
    regional_travel_blocked: bool,
) -> &'static str {
    if connection != ConnectionState::Online {
        "The shared road is reconnecting; tap Reconnect when it is available."
    } else if knocked_out {
        "Choose a recovery prompt before walking."
    } else if !player_position_authoritative {
        "Your position is still loading; wait for the shared road snapshot."
    } else if regional_travel_blocked {
        "Your regional journey is underway; use the visible Travel or Recover control."
    } else {
        "Press and hold the map to move freely; arrow keys also work."
    }
}

#[cfg(test)]
pub(crate) fn visible_companion_count(
    players: &[RemotePlayer],
    own_account_id: Option<&str>,
    server_tick: u64,
) -> usize {
    players
        .iter()
        .filter(|player| {
            own_account_id != Some(player.account_id.as_str()) && !player.stale(server_tick)
        })
        .count()
}

#[cfg(test)]
pub(crate) fn visible_player_count(players: &[RemotePlayer], server_tick: u64) -> usize {
    players
        .iter()
        .filter(|player| !player.stale(server_tick))
        .count()
}

#[cfg(test)]
pub(crate) fn pioneer_status_line(
    expedition: &tarrowyn_protocol::Expedition,
    requirements: tarrowyn_protocol::ExpeditionRequirements,
) -> String {
    let status = match expedition.status {
        tarrowyn_protocol::ExpeditionStatus::Planning => "planning",
        tarrowyn_protocol::ExpeditionStatus::Launched => "on the road",
        tarrowyn_protocol::ExpeditionStatus::Succeeded => "founded",
        tarrowyn_protocol::ExpeditionStatus::Retreated => "retreated",
    };
    format!(
        "Pioneer {status} • {} companions • F{}/{} T{}/{} M{}/{} S{}/{} • {}",
        expedition.members.len(),
        expedition.food,
        requirements.food,
        expedition.tools,
        requirements.tools,
        expedition.materials,
        requirements.materials,
        expedition.safety,
        requirements.safety,
        expedition.outpost_name,
    )
}
