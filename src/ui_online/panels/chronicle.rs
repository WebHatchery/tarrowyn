use super::*;

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
