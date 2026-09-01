use super::*;

pub(super) fn draw_online_tools(
    panel: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
    ctx: &UiContext<'_>,
) {
    let has_pending_trade = ctx.trades.iter().any(|trade| {
        trade.status == tarrowyn_protocol::TradeStatus::Pending
            && ctx.own_account_id.is_some_and(|account_id| {
                trade.creator_account_id == account_id || trade.recipient_account_id == account_id
            })
    });
    let has_incoming_trade = ctx.trades.iter().any(|trade| {
        trade.status == tarrowyn_protocol::TradeStatus::Pending
            && ctx
                .own_account_id
                .is_some_and(|account_id| trade.recipient_account_id == account_id)
    });
    let frontier_threat_reachable =
        super::super::ui_online::frontier_threat_is_reachable(ctx.player_position, ctx.wilderness);
    let (combat_side_id, combat_side_label) =
        super::super::ui_online::combat_side_control(ctx.combat, frontier_threat_reachable);
    let local_combat_action_ready = super::super::ui_online::combat_control_enabled(
        super::super::ui_online::local_combat_action_enabled(ctx.combat, ctx.server_tick),
        ctx.combat_pending,
    );
    let combat_side_enabled = match combat_side_id {
        "contract" => super::super::ui_online::contract_control_enabled(true, ctx.contract_pending),
        "frontier-retreat" => super::super::ui_online::frontier_combat_control_enabled(
            frontier_threat_reachable,
            ctx.frontier_combat_pending,
        ),
        "retreat" => local_combat_action_ready,
        _ => false,
    };
    let local_fight_enabled = !ctx.knocked_out
        && !ctx.combat.is_some_and(|combat| {
            combat.status == tarrowyn_protocol::LocalCombatStatus::Engaged
                && combat.action_available_at_tick > ctx.server_tick
        })
        && !ctx.combat_pending;
    let repair_available = ctx.regional_region.is_some_and(|region| {
        region.routes.iter().any(|route| {
            (route.origin_location_id == region.player_location_id
                || route.destination_location_id == region.player_location_id)
                && route.status != tarrowyn_protocol::RouteStatus::Operational
        })
    });
    let encounter_entries = if ctx.knocked_out {
        vec![
            (
                "recover-self",
                "Self",
                super::super::ui_online::recovery_control_enabled(
                    ctx.knocked_out,
                    ctx.recovery_pending,
                ),
                ButtonTone::Secondary,
            ),
            (
                "recover",
                "Rescuer",
                super::super::ui_online::recovery_control_enabled(
                    ctx.knocked_out,
                    ctx.recovery_pending,
                ),
                ButtonTone::Secondary,
            ),
            (
                "recover-healer",
                "Healer",
                super::super::ui_online::recovery_control_enabled(
                    ctx.knocked_out,
                    ctx.recovery_pending,
                ),
                ButtonTone::Secondary,
            ),
        ]
    } else {
        vec![
            (
                combat_side_id,
                combat_side_label,
                combat_side_enabled,
                ButtonTone::Secondary,
            ),
            (
                "strike",
                "Strike",
                local_combat_action_ready,
                ButtonTone::Secondary,
            ),
            (
                "technique",
                "Technique",
                local_combat_action_ready,
                ButtonTone::Secondary,
            ),
            (
                "claim",
                "Claim",
                super::super::ui_online::claim_control_enabled(true, ctx.frontier_claim_pending),
                ButtonTone::Secondary,
            ),
            (
                "item",
                "Bandage",
                local_combat_action_ready,
                ButtonTone::Secondary,
            ),
        ]
    };
    let x = panel.x + 28.0;
    let width = panel.w - 56.0;
    let rows = [
        (
            "FIELD",
            vec![
                (
                    "plant",
                    "Plant",
                    super::super::ui_online::farming_control_enabled(
                        !ctx.knocked_out,
                        ctx.farming_pending,
                    ),
                    ButtonTone::Positive,
                ),
                (
                    "tend",
                    "Tend",
                    super::super::ui_online::farming_control_enabled(
                        !ctx.knocked_out,
                        ctx.farming_pending,
                    ),
                    ButtonTone::Positive,
                ),
                (
                    "harvest",
                    "Harvest",
                    super::super::ui_online::farming_control_enabled(
                        !ctx.knocked_out,
                        ctx.farming_pending,
                    ),
                    ButtonTone::Positive,
                ),
                (
                    "animal",
                    "Care",
                    super::super::ui_online::farming_control_enabled(
                        !ctx.knocked_out,
                        ctx.farming_pending,
                    ),
                    ButtonTone::Positive,
                ),
                (
                    "trade",
                    "Trade",
                    super::super::ui_online::trade_control_enabled(true, ctx.trade_pending),
                    ButtonTone::Primary,
                ),
                (
                    "accept-trade",
                    "Accept",
                    super::super::ui_online::trade_control_enabled(
                        has_incoming_trade,
                        ctx.trade_pending,
                    ),
                    ButtonTone::Positive,
                ),
                (
                    "cancel-trade",
                    "Cancel",
                    super::super::ui_online::trade_control_enabled(
                        has_pending_trade,
                        ctx.trade_pending,
                    ),
                    ButtonTone::Secondary,
                ),
            ],
        ),
        ("ENCOUNTER", encounter_entries),
        (
            "DISCOVERY",
            vec![
                (
                    "spell",
                    if ctx.storm_magic_unlocked {
                        "Storm"
                    } else {
                        "Spell"
                    },
                    local_combat_action_ready,
                    ButtonTone::Secondary,
                ),
                (
                    "expedition",
                    "Pioneer",
                    super::super::ui_online::expedition_control_enabled(
                        !ctx.knocked_out,
                        ctx.expedition_pending,
                    ),
                    ButtonTone::Primary,
                ),
                ("chronicle", "Chronicle", true, ButtonTone::Secondary),
                ("say-hello", "Meet", true, ButtonTone::Secondary),
                (
                    "school",
                    "School",
                    super::super::ui_online::skill_control_enabled(
                        !ctx.knocked_out,
                        ctx.skill_pending,
                    ),
                    ButtonTone::Primary,
                ),
                (
                    "practice",
                    "Practice",
                    super::super::ui_online::skill_control_enabled(
                        !ctx.knocked_out,
                        ctx.skill_pending,
                    ),
                    ButtonTone::Positive,
                ),
            ],
        ),
        (
            "SETTLEMENT",
            vec![
                (
                    "town-hall",
                    "Town hall",
                    super::super::ui_online::governance_control_enabled(ctx.governance_pending),
                    ButtonTone::Primary,
                ),
                (
                    "registry",
                    "Registry",
                    super::super::ui_online::claim_control_enabled(true, ctx.claim_pending),
                    ButtonTone::Secondary,
                ),
                (
                    "abandon-claim",
                    "Abandon",
                    super::super::ui_online::claim_control_enabled(
                        ctx.can_abandon_claim,
                        ctx.claim_pending,
                    ),
                    ButtonTone::Secondary,
                ),
                (
                    "transfer-claim",
                    "Transfer",
                    super::super::ui_online::claim_control_enabled(
                        ctx.can_transfer_claim,
                        ctx.claim_pending,
                    ),
                    ButtonTone::Secondary,
                ),
                (
                    "order",
                    "Order",
                    super::super::ui_online::order_control_enabled(
                        ctx.crafting.is_none(),
                        ctx.order_pending,
                    ),
                    ButtonTone::Secondary,
                ),
                (
                    "tax-rate",
                    "Tax",
                    super::super::ui_online::governance_control_enabled(ctx.governance_pending),
                    ButtonTone::Secondary,
                ),
            ],
        ),
        (
            "COMPANIONS",
            vec![
                (
                    "knowledge",
                    ctx.knowledge_label,
                    super::super::ui_online::knowledge_control_enabled(ctx.knowledge_pending),
                    ButtonTone::Secondary,
                ),
                ("households", "Households", true, ButtonTone::Secondary),
                (
                    "local-fight",
                    "Local fight",
                    local_fight_enabled,
                    ButtonTone::Secondary,
                ),
                (
                    "guard",
                    "Guard",
                    local_combat_action_ready,
                    ButtonTone::Secondary,
                ),
                (
                    "reposition",
                    "Reposition",
                    local_combat_action_ready,
                    ButtonTone::Secondary,
                ),
            ],
        ),
        (
            "REGIONAL",
            vec![
                (
                    "travel",
                    ctx.travel_label,
                    super::super::ui_online::travel_control_enabled(
                        ctx.can_travel,
                        ctx.knocked_out,
                        ctx.travel_pending,
                    ),
                    ButtonTone::Primary,
                ),
                (
                    "recover-travel",
                    "Recover",
                    super::super::ui_online::travel_control_enabled(
                        ctx.can_recover_travel,
                        ctx.knocked_out,
                        ctx.travel_pending,
                    ),
                    ButtonTone::Primary,
                ),
                (
                    "route-repair",
                    "Repair",
                    super::super::ui_online::route_control_enabled(
                        repair_available,
                        ctx.route_pending,
                    ),
                    ButtonTone::Positive,
                ),
                (
                    "market-region",
                    "Market",
                    super::super::ui_online::market_control_enabled(ctx.market_pending),
                    ButtonTone::Primary,
                ),
                (
                    "region-event",
                    "Event",
                    super::super::ui_online::event_control_enabled(ctx.event_pending),
                    ButtonTone::Primary,
                ),
                ("region-details", "Inspect", true, ButtonTone::Secondary),
                (
                    "cancel-market",
                    "Cancel",
                    super::super::ui_online::cancel_market_control_enabled(
                        ctx.has_open_market_order,
                        ctx.market_pending,
                    ),
                    ButtonTone::Secondary,
                ),
            ],
        ),
        (
            "ACCOUNT",
            vec![
                (
                    if ctx.account_link_available {
                        "account"
                    } else {
                        "account-details"
                    },
                    "Account",
                    true,
                    ButtonTone::Secondary,
                ),
                (
                    "logout",
                    "Logout",
                    super::super::ui_online::identity_control_enabled(ctx.identity_pending),
                    ButtonTone::Secondary,
                ),
                (
                    "report",
                    "Report",
                    super::super::ui_online::report_control_enabled(ctx.report_pending),
                    ButtonTone::Secondary,
                ),
                (
                    "delete-account",
                    if ctx.identity_pending {
                        "Pending"
                    } else if ctx.account_deletion_armed {
                        "Tap again"
                    } else {
                        "Delete"
                    },
                    ctx.account_deletion_available
                        && super::super::ui_online::identity_control_enabled(ctx.identity_pending),
                    ButtonTone::Danger,
                ),
            ],
        ),
        (
            "SESSION",
            vec![
                ("reconnect", "Reconnect", true, ButtonTone::Primary),
                (
                    "offline",
                    "Offline first evening",
                    true,
                    ButtonTone::Secondary,
                ),
            ],
        ),
    ];
    for (index, (label, entries)) in rows.iter().enumerate() {
        let y = panel.y + 88.0 + index as f32 * 51.0;
        draw_ui_text_ex(
            label,
            x,
            y - 7.0,
            TextStyle::new(8.0, if index == 0 { MINT } else { GOLD }).params(),
        );
        super::super::ui_online::draw_button_row(
            Rect::new(x, y, width, 30.0),
            y,
            30.0,
            mouse,
            entries,
            ctx,
            actions,
        );
    }
}
