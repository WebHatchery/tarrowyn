use super::*;
use tarrowyn_protocol::{RegionSnapshot, RouteAction, RouteStatus, TravelStatus};

pub fn draw_account(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    if !ctx.account_open {
        return;
    }
    let panel = Rect::new(42.0, 126.0, 780.0, 438.0);
    draw_surface_with_title(
        panel,
        Some("Account and character"),
        &SurfaceStyle::new(Color::new(0.055, 0.075, 0.09, 0.99))
            .with_border(1.0, Color::new(0.50, 0.72, 0.62, 0.9))
            .with_header(42.0, Color::new(0.09, 0.14, 0.15, 1.0))
            .with_header_divider(1.0, Color::new(0.32, 0.48, 0.50, 0.75)),
        TextStyle::new(17.0, CREAM),
    );
    draw_text_block(
        ctx.account_summary,
        panel.x + 20.0,
        panel.y + 70.0,
        panel.w - 40.0,
        318.0,
        14.0,
        3.0,
        CREAM,
    );
    if virtual_button(
        Rect::new(panel.right() - 126.0, panel.bottom() - 42.0, 106.0, 28.0),
        "Close",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("account-close".to_owned()));
    }
}

pub fn draw_chronicle(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    if !ctx.chronicle_open {
        return;
    }
    let panel = Rect::new(42.0, 126.0, 780.0, 438.0);
    draw_surface_with_title(
        panel,
        Some("Chronicle archive"),
        &SurfaceStyle::new(Color::new(0.055, 0.075, 0.09, 0.99))
            .with_border(1.0, Color::new(0.50, 0.72, 0.62, 0.9))
            .with_header(42.0, Color::new(0.09, 0.14, 0.15, 1.0))
            .with_header_divider(1.0, Color::new(0.32, 0.48, 0.50, 0.75)),
        TextStyle::new(17.0, CREAM),
    );
    draw_surface(
        Rect::new(panel.x + 20.0, panel.y + 58.0, panel.w - 40.0, 30.0),
        &SurfaceStyle::new(Color::new(0.10, 0.14, 0.15, 1.0))
            .with_border(1.0, Color::new(0.32, 0.48, 0.50, 0.55)),
    );
    draw_ui_text_ex(
        &format!(
            "Query: {}",
            if ctx.chronicle_query.trim().is_empty() {
                "all records"
            } else {
                ctx.chronicle_query
            }
        ),
        panel.x + 28.0,
        panel.y + 78.0,
        TextStyle::new(12.0, CREAM).params(),
    );
    let panel_text = if ctx.chronicle_search_pending {
        format!(
            "Searching the durable chronicle for {}…",
            if ctx.chronicle_query.trim().is_empty() {
                "all records".to_owned()
            } else {
                format!("“{}”", ctx.chronicle_query)
            }
        )
    } else if let Some(query) = ctx.chronicle_search_query {
        chronicle_search_panel_text(query, ctx.chronicle_search, ctx.chronicle_search_summary)
    } else {
        chronicle_panel_text(ctx.chronicle, ctx.chronicle_summary)
    };
    draw_text_block(
        &panel_text,
        panel.x + 20.0,
        panel.y + 102.0,
        panel.w - 40.0,
        160.0,
        14.0,
        3.0,
        CREAM,
    );
    draw_chronicle_keyboard(panel, mouse, actions);
    if virtual_button(
        Rect::new(panel.x + 20.0, panel.bottom() - 42.0, 106.0, 28.0),
        if ctx.chronicle_search_pending {
            "Searching…"
        } else {
            "Search"
        },
        !ctx.chronicle_search_pending,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::Interact("chronicle-search".to_owned()));
    }
    let can_advance_search = chronicle_search_can_advance(
        ctx.chronicle_query,
        ctx.chronicle_search_query,
        ctx.chronicle_search_next_cursor,
    );
    if can_advance_search
        && virtual_button(
            Rect::new(panel.x + 134.0, panel.bottom() - 42.0, 106.0, 28.0),
            "Next",
            !ctx.chronicle_search_pending,
            ButtonTone::Secondary,
            mouse,
        )
    {
        actions.push(UiAction::Interact("chronicle-search-next".to_owned()));
    }
    if virtual_button(
        Rect::new(panel.right() - 126.0, panel.bottom() - 42.0, 106.0, 28.0),
        "Close",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("chronicle-close".to_owned()));
    }
}

fn draw_chronicle_keyboard(panel: Rect, mouse: Vec2, actions: &mut Vec<UiAction>) {
    draw_ui_text_ex(
        "Tap letters to refine the archive query.",
        panel.x + 20.0,
        panel.y + 281.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );
    let rows = ["ABCDEFGHIJ", "KLMNOPQRST", "UVWXYZ"];
    let gap = 4.0;
    let columns = 10.0;
    let width = (panel.w - 40.0 - gap * (columns - 1.0)) / columns;
    let height = 23.0;
    for (row, keys) in rows.iter().enumerate() {
        for (column, key) in keys.chars().enumerate() {
            if virtual_button(
                Rect::new(
                    panel.x + 20.0 + column as f32 * (width + gap),
                    panel.y + 284.0 + row as f32 * (height + gap),
                    width,
                    height,
                ),
                &key.to_string(),
                true,
                ButtonTone::Secondary,
                mouse,
            ) {
                actions.push(UiAction::Interact(format!("chronicle-key-{key}")));
            }
        }
    }
    if virtual_button(
        Rect::new(panel.x + 20.0, panel.y + 365.0, 220.0, height),
        "Space",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("chronicle-key-space".to_owned()));
    }
    if virtual_button(
        Rect::new(panel.x + 244.0, panel.y + 365.0, 106.0, height),
        "Delete",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("chronicle-key-delete".to_owned()));
    }
    if virtual_button(
        Rect::new(panel.x + 354.0, panel.y + 365.0, 106.0, height),
        "Clear",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("chronicle-key-clear".to_owned()));
    }
}

pub fn chronicle_panel_text(
    entries: &[tarrowyn_protocol::ChronicleEntry],
    summary: Option<&tarrowyn_protocol::ChronicleSummary>,
) -> String {
    let mut lines = Vec::new();
    if let Some(summary) = summary {
        lines.push(format!(
            "Archive: {} records across beats {}–{}.",
            summary.entry_count, summary.from_tick, summary.to_tick
        ));
        if let Some(highlight) = summary.highlights.last() {
            lines.push(format!("Last highlight: {highlight}"));
        }
    }
    lines.push("Recent community records:".to_owned());
    if entries.is_empty() {
        lines.push("The chronicle is quiet; new shared-road moments will appear here.".to_owned());
    } else {
        for entry in entries.iter().rev().take(6) {
            lines.push(format!("• {} — {}", entry.title, entry.text));
        }
    }
    lines.join("\n")
}

pub fn chronicle_search_panel_text(
    query: &str,
    entries: &[tarrowyn_protocol::ChronicleEntry],
    summary: Option<&tarrowyn_protocol::ChronicleSummary>,
) -> String {
    let label = if query.trim().is_empty() {
        "all records".to_owned()
    } else {
        format!("“{query}”")
    };
    let mut lines = vec![format!("Search results for {label}:")];
    if let Some(summary) = summary {
        lines.push(format!(
            "Archive range: beats {}–{} • {} matching records.",
            summary.from_tick, summary.to_tick, summary.entry_count
        ));
    }
    if entries.is_empty() {
        lines.push("No matching records were found in the durable chronicle.".to_owned());
    } else {
        for entry in entries.iter().rev().take(6) {
            lines.push(format!("• {} — {}", entry.title, entry.text));
        }
    }
    lines.join("\n")
}

pub fn chronicle_search_can_advance(
    query: &str,
    search_query: Option<&str>,
    next_cursor: Option<u64>,
) -> bool {
    next_cursor.is_some_and(|_| search_query.is_some_and(|search| search.trim() == query.trim()))
}

pub fn combat_side_control(
    combat: Option<&tarrowyn_protocol::LocalCombatState>,
    frontier_threat_active: bool,
) -> (&'static str, &'static str) {
    if combat.is_some_and(|combat| combat.status == tarrowyn_protocol::LocalCombatStatus::Engaged) {
        ("retreat", "Retreat")
    } else if frontier_threat_active {
        ("frontier-retreat", "Retreat")
    } else {
        ("contract", "Contract")
    }
}

pub fn local_combat_action_enabled(
    combat: Option<&tarrowyn_protocol::LocalCombatState>,
    server_tick: u64,
) -> bool {
    combat.is_some_and(|combat| {
        combat.status == tarrowyn_protocol::LocalCombatStatus::Engaged
            && combat.action_available_at_tick <= server_tick
    })
}

pub fn regional_travel_blocks_movement(region: Option<&RegionSnapshot>) -> bool {
    region
        .and_then(|region| region.travel.as_ref())
        .is_some_and(|travel| {
            matches!(
                travel.status,
                TravelStatus::Travelling | TravelStatus::Interrupted | TravelStatus::Recovering
            )
        })
}

pub fn frontier_threat_is_reachable(
    player_position: TilePos,
    wilderness: Option<&tarrowyn_protocol::WildernessZone>,
) -> bool {
    wilderness.is_some_and(|zone| {
        zone.threat_active
            && player_position.manhattan_distance(&TilePos::new(zone.position.x, zone.position.y))
                <= 2
    })
}

pub fn draw_regional_inspection(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let Some(details) = ctx.regional_inspection else {
        return;
    };
    let panel = Rect::new(42.0, 126.0, 780.0, 438.0);
    draw_surface_with_title(
        panel,
        Some("Regional ledger inspection"),
        &SurfaceStyle::new(Color::new(0.055, 0.075, 0.09, 0.99))
            .with_border(1.0, Color::new(0.50, 0.72, 0.62, 0.9))
            .with_header(42.0, Color::new(0.09, 0.14, 0.15, 1.0))
            .with_header_divider(1.0, Color::new(0.32, 0.48, 0.50, 0.75)),
        TextStyle::new(17.0, CREAM),
    );
    let detail_height = if ctx.regional_event_choices.is_empty() {
        318.0
    } else {
        238.0
    };
    draw_text_block(
        details,
        panel.x + 20.0,
        panel.y + 70.0,
        panel.w - 40.0,
        detail_height,
        14.0,
        3.0,
        CREAM,
    );
    if !ctx.regional_event_choices.is_empty() {
        draw_event_choices(
            panel,
            ctx.regional_event_choices,
            ctx.event_pending,
            mouse,
            actions,
        );
    }
    draw_button_row(
        Rect::new(panel.x + 20.0, panel.bottom() - 82.0, panel.w - 40.0, 28.0),
        panel.bottom() - 82.0,
        28.0,
        mouse,
        &[
            (
                "route-escort",
                "Escort road",
                route_control_enabled(
                    has_local_route(ctx.regional_region, RouteAction::Escort),
                    ctx.route_pending,
                ),
                ButtonTone::Positive,
            ),
            (
                "route-improve",
                "Improve road",
                route_control_enabled(
                    has_local_route(ctx.regional_region, RouteAction::Improve),
                    ctx.route_pending,
                ),
                ButtonTone::Primary,
            ),
        ],
        ctx,
        actions,
    );
    if virtual_button(
        Rect::new(panel.right() - 126.0, panel.bottom() - 42.0, 106.0, 28.0),
        "Close",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("region-details".to_owned()));
    }
}

pub(super) fn has_local_route(region: Option<&RegionSnapshot>, action: RouteAction) -> bool {
    region.is_some_and(|region| {
        region.routes.iter().any(|route| {
            (route.origin_location_id == region.player_location_id
                || route.destination_location_id == region.player_location_id)
                && match action {
                    RouteAction::Repair => route.status != RouteStatus::Operational,
                    RouteAction::Escort => true,
                    RouteAction::Improve => route.status != RouteStatus::Closed,
                }
        })
    })
}

fn draw_event_choices(
    panel: Rect,
    choices: &[String],
    event_pending: bool,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    let gap = 4.0;
    let visible = choices.iter().take(3).collect::<Vec<_>>();
    let width = (panel.w - 40.0 - gap * (visible.len().saturating_sub(1) as f32))
        / visible.len().max(1) as f32;
    for (index, choice) in visible.iter().enumerate() {
        if virtual_button(
            Rect::new(
                panel.x + 20.0 + index as f32 * (width + gap),
                panel.bottom() - 118.0,
                width,
                28.0,
            ),
            choice,
            !event_pending,
            ButtonTone::Primary,
            mouse,
        ) {
            actions.push(UiAction::RegionalEvent((*choice).clone()));
        }
    }
}

fn route_control_enabled(route_available: bool, route_pending: bool) -> bool {
    route_available && !route_pending
}

pub fn draw_skill_selection(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    if !ctx.skill_selection_open {
        return;
    }
    let panel = Rect::new(42.0, 126.0, 780.0, 438.0);
    draw_surface_with_title(
        panel,
        Some("Choose a discipline"),
        &SurfaceStyle::new(Color::new(0.055, 0.075, 0.09, 0.99))
            .with_border(1.0, Color::new(0.50, 0.72, 0.62, 0.9))
            .with_header(42.0, Color::new(0.09, 0.14, 0.15, 1.0))
            .with_header_divider(1.0, Color::new(0.32, 0.48, 0.50, 0.75)),
        TextStyle::new(17.0, CREAM),
    );
    draw_text_block(
        "Tap a depth-one practice to begin or continue it. Advanced arts emerge from play and are not direct choices here.",
        panel.x + 20.0,
        panel.y + 70.0,
        panel.w - 40.0,
        34.0,
        13.0,
        2.0,
        CREAM,
    );
    draw_ui_text_ex(
        &advanced_skill_line(ctx.skills),
        panel.x + 20.0,
        panel.y + 131.0,
        TextStyle::new(10.0, MINT).params(),
    );
    if ctx.skill_pending {
        draw_ui_text_ex(
            "The skill ledger is settling; wait for its response before choosing another discipline.",
            panel.x + 20.0,
            panel.y + 145.0,
            TextStyle::new(11.0, dark::TEXT_DIM).params(),
        );
    }
    let choices: Vec<_> = ctx
        .skills
        .iter()
        .filter(|skill| skill_practice_choice(skill))
        .collect();
    if choices.is_empty() {
        draw_ui_text_ex(
            if ctx.skills.is_empty() {
                "The skill ledger is loading; tap Close and try again shortly."
            } else {
                "Every depth-one practice is mastered; advanced discoveries remain in the ledger."
            },
            panel.x + 20.0,
            panel.y + 138.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
    } else {
        draw_ui_text_ex(
            &format!("{} depth-one practices available", choices.len()),
            panel.x + 20.0,
            panel.y + 133.0,
            TextStyle::new(11.0, MINT).params(),
        );
        draw_skill_choices(panel, &choices, ctx.skill_pending, mouse, actions);
    }
    if virtual_button(
        Rect::new(panel.right() - 126.0, panel.bottom() - 42.0, 106.0, 28.0),
        "Close",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("skill-close".to_owned()));
    }
}

fn draw_skill_choices(
    panel: Rect,
    choices: &[&tarrowyn_protocol::SkillView],
    skill_pending: bool,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    let columns = 4;
    let gap = 4.0;
    let width = (panel.w - 40.0 - gap * (columns - 1) as f32) / columns as f32;
    let height = 25.0;
    for (index, skill) in choices.iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        if virtual_button(
            Rect::new(
                panel.x + 20.0 + column as f32 * (width + gap),
                panel.y + 148.0 + row as f32 * (height + gap),
                width,
                height,
            ),
            &skill.name,
            !skill_pending,
            ButtonTone::Primary,
            mouse,
        ) {
            actions.push(UiAction::Practice(skill.skill_id.clone()));
        }
    }
}

pub fn skill_practice_choice(skill: &tarrowyn_protocol::SkillView) -> bool {
    skill.depth == 1
        && matches!(
            skill.status,
            tarrowyn_protocol::SkillStatus::Available | tarrowyn_protocol::SkillStatus::Practising
        )
}

pub fn advanced_skill_line(skills: &[tarrowyn_protocol::SkillView]) -> String {
    let visible = skills
        .iter()
        .filter(|skill| {
            skill.depth > 1
                && matches!(
                    skill.status,
                    tarrowyn_protocol::SkillStatus::Resonating
                        | tarrowyn_protocol::SkillStatus::Discovered
                )
        })
        .map(|skill| {
            let status = match skill.status {
                tarrowyn_protocol::SkillStatus::Resonating => "resonating",
                tarrowyn_protocol::SkillStatus::Discovered => "discovered",
                _ => "hidden",
            };
            format!("{} ({status})", skill.name)
        })
        .collect::<Vec<_>>();
    if visible.is_empty() {
        "Advanced arts remain quiet; keep practising to find a pattern.".to_owned()
    } else {
        format!("Advanced arts in your ledger: {}", visible.join(" • "))
    }
}

pub fn draw_combat_status(ctx: &UiContext<'_>, content: Rect, top: f32) {
    let Some(combat) = ctx.combat else {
        return;
    };
    let available_in = combat
        .action_available_at_tick
        .saturating_sub(ctx.server_tick);
    let timing = if available_in == 0 {
        "Action ready".to_owned()
    } else {
        format!(
            "Action opens in {available_in} beat{}",
            if available_in == 1 { "" } else { "s" }
        )
    };
    let status = match combat.status {
        tarrowyn_protocol::LocalCombatStatus::Ready => "ready",
        tarrowyn_protocol::LocalCombatStatus::Engaged => "engaged",
        tarrowyn_protocol::LocalCombatStatus::Victorious => "victorious",
        tarrowyn_protocol::LocalCombatStatus::KnockedOut => "knocked out",
        tarrowyn_protocol::LocalCombatStatus::Retreated => "retreated",
    };
    draw_surface(
        Rect::new(
            content.x,
            top + 101.0,
            content.w,
            if combat.status == tarrowyn_protocol::LocalCombatStatus::KnockedOut {
                55.0
            } else {
                43.0
            },
        ),
        &SurfaceStyle::new(Color::new(0.075, 0.105, 0.115, 1.0))
            .with_border(1.0, Color::new(0.62, 0.42, 0.22, 0.7)),
    );
    draw_ui_text_ex(
        &format!(
            "Encounter {status}  •  enemy {}  •  you {}",
            combat.enemy_health, combat.player_health
        ),
        content.x + 8.0,
        top + 115.0,
        TextStyle::new(10.0, GOLD).params(),
    );
    draw_ui_text_ex(
        &combat_weapon_line(combat.weapon, &timing),
        content.x + 8.0,
        top + 129.0,
        TextStyle::new(8.5, CREAM).params(),
    );
    if combat.status == tarrowyn_protocol::LocalCombatStatus::KnockedOut {
        draw_ui_text_ex(
            &format!(
                "Risk: {}  •  Healer: {} gold  •  stored property safe",
                recovery_risk_label(&combat.carried_risk),
                combat.recovery_cost,
            ),
            content.x + 8.0,
            top + 143.0,
            TextStyle::new(8.0, CREAM).params(),
        );
    }
}

pub fn combat_weapon_line(weapon: tarrowyn_protocol::WeaponKind, timing: &str) -> String {
    format!("Weapon: {}  •  {timing}", weapon.label())
}

pub fn recovery_risk_label(carried_risk: &str) -> &'static str {
    if carried_risk.to_ascii_lowercase().contains("seed") {
        "1 carried seed"
    } else {
        "carried item"
    }
}

pub fn reconnect_control_enabled(connection: ConnectionState) -> bool {
    matches!(
        connection,
        ConnectionState::Degraded | ConnectionState::Offline
    )
}

pub fn draw_button_row(
    content: Rect,
    y: f32,
    height: f32,
    mouse: Vec2,
    entries: &[(&str, &str, bool, ButtonTone)],
    ctx: &UiContext<'_>,
    actions: &mut Vec<UiAction>,
) {
    let gap = 4.0;
    let width =
        (content.w - gap * (entries.len().saturating_sub(1) as f32)) / entries.len().max(1) as f32;
    for (index, (id, label, active, tone)) in entries.iter().enumerate() {
        let enabled = sidebar_button_enabled(
            id,
            *active,
            ctx.connection,
            ctx.player_position_authoritative,
        );
        if virtual_button(
            Rect::new(content.x + index as f32 * (width + gap), y, width, height),
            label,
            enabled,
            *tone,
            mouse,
        ) {
            if *id == "reconnect" {
                actions.push(UiAction::Reconnect);
            } else if *id == "offline" {
                actions.push(UiAction::UseOffline);
            } else if *id == "say-hello" {
                actions.push(UiAction::QuickChat("Meet at the Hearth".to_owned()));
            } else {
                actions.push(UiAction::Interact((*id).to_owned()));
            }
        }
    }
}

pub(super) fn sidebar_button_enabled(
    id: &str,
    active: bool,
    connection: ConnectionState,
    player_position_authoritative: bool,
) -> bool {
    match id {
        "reconnect" => reconnect_control_enabled(connection),
        "offline" => true,
        "account" | "account-details" | "logout" | "report" | "delete-account" => {
            account_control_enabled(active, connection)
        }
        _ => button_enabled(active, connection, player_position_authoritative),
    }
}

pub fn button_enabled(
    active: bool,
    connection: ConnectionState,
    player_position_authoritative: bool,
) -> bool {
    active && connection == ConnectionState::Online && player_position_authoritative
}

pub fn account_control_enabled(active: bool, connection: ConnectionState) -> bool {
    active && connection == ConnectionState::Online
}
