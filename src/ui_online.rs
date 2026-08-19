use super::*;

pub(super) fn draw_sidebar(
    ctx: &UiContext<'_>,
    content: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    draw_surface(
        Rect::new(content.x, content.y + 34.0, content.w, 58.0),
        &SurfaceStyle::new(Color::new(0.10, 0.14, 0.15, 1.0))
            .with_border(1.0, Color::new(0.32, 0.48, 0.50, 0.45)),
    );
    draw_text_block(
        &format!(
            "{}\n{}  •  {} visible companions",
            ctx.identity_name.unwrap_or("Guest identity"),
            ctx.connection.label(),
            ctx.remote_players
                .iter()
                .filter(|player| ctx.own_account_id != Some(player.account_id.as_str()))
                .count()
        ),
        content.x + 12.0,
        content.y + 50.0,
        content.w - 24.0,
        42.0,
        13.0,
        2.0,
        CREAM,
    );

    draw_ui_text_ex(
        "Walk the server-owned road",
        content.x,
        content.y + 116.0,
        TextStyle::new(16.0, CREAM).params(),
    );
    draw_move_pad(content.x + 77.0, content.y + 126.0, mouse, actions);

    draw_ui_text_ex(
        "Shared fields",
        content.x,
        content.y + 218.0,
        TextStyle::new(16.0, CREAM).params(),
    );
    for (index, (id, label)) in [
        ("plant", "Plant"),
        ("tend", "Tend"),
        ("harvest", "Harvest"),
        ("trade", "Trade seed"),
    ]
    .into_iter()
    .enumerate()
    {
        if virtual_button(
            Rect::new(
                content.x + index as f32 * 88.0,
                content.y + 226.0,
                82.0,
                27.0,
            ),
            label,
            ctx.connection == ConnectionState::Online && !ctx.offline,
            ButtonTone::Positive,
            mouse,
        ) {
            actions.push(UiAction::Interact(id.to_owned()));
        }
    }

    draw_ui_text_ex(
        if ctx.wilderness.is_some() {
            "Settlement chat • frontier signals"
        } else {
            "Settlement chat"
        },
        content.x,
        content.y + 267.0,
        TextStyle::new(16.0, CREAM).params(),
    );
    draw_surface(
        Rect::new(content.x, content.y + 278.0, content.w, 58.0),
        &SurfaceStyle::new(Color::new(0.075, 0.105, 0.115, 1.0))
            .with_border(1.0, Color::new(0.32, 0.48, 0.50, 0.45)),
    );
    for (index, message) in ctx.chat.iter().rev().take(2).enumerate() {
        draw_ui_text_ex(
            &format!("{}: {}", message.display_name, message.text),
            content.x + 8.0,
            content.y + 294.0 + index as f32 * 18.0,
            TextStyle::new(11.0, if index == 0 { CREAM } else { dark::TEXT_DIM }).params(),
        );
    }

    draw_surface(
        Rect::new(content.x, content.y + 341.0, content.w - 78.0, 30.0),
        &SurfaceStyle::new(Color::new(0.10, 0.14, 0.15, 1.0))
            .with_border(1.0, Color::new(0.32, 0.48, 0.50, 0.45)),
    );
    draw_ui_text_ex(
        if ctx.chat_draft.is_empty() {
            "Type a message or tap a quick phrase"
        } else {
            ctx.chat_draft
        },
        content.x + 9.0,
        content.y + 361.0,
        TextStyle::new(
            11.0,
            if ctx.chat_draft.is_empty() {
                dark::TEXT_DIM
            } else {
                CREAM
            },
        )
        .params(),
    );
    if virtual_button(
        Rect::new(content.right() - 70.0, content.y + 341.0, 70.0, 30.0),
        "Send",
        !ctx.chat_draft.trim().is_empty() && ctx.connection == ConnectionState::Online,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::SendChat);
    }

    for (index, (id, label)) in [
        ("contract", "Contract"),
        ("strike", "Strike"),
        ("recover", "Recover"),
        ("claim", "Claim"),
    ]
    .iter()
    .enumerate()
    {
        if virtual_button(
            Rect::new(
                content.x + index as f32 * 88.0,
                content.y + 379.0,
                82.0,
                24.0,
            ),
            label,
            ctx.connection == ConnectionState::Online && (!ctx.knocked_out || *id == "recover"),
            ButtonTone::Secondary,
            mouse,
        ) {
            actions.push(UiAction::Interact((*id).to_owned()));
        }
    }

    if virtual_button(
        Rect::new(content.x, content.y + 408.0, 122.0, 24.0),
        "Pioneer",
        ctx.connection == ConnectionState::Online && !ctx.knocked_out,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::Interact("expedition".to_owned()));
    }
    if virtual_button(
        Rect::new(content.x + 130.0, content.y + 408.0, 122.0, 24.0),
        "Chronicle",
        ctx.connection == ConnectionState::Online,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("chronicle".to_owned()));
    }
    if virtual_button(
        Rect::new(content.x + 260.0, content.y + 408.0, 122.0, 24.0),
        "Say hello",
        ctx.connection == ConnectionState::Online,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::QuickChat("Meet at the Hearth".to_owned()));
    }

    if virtual_button(
        Rect::new(content.x, content.y + 438.0, 122.0, 24.0),
        "Town hall",
        ctx.connection == ConnectionState::Online,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::Interact("town-hall".to_owned()));
    }
    if virtual_button(
        Rect::new(content.x + 130.0, content.y + 438.0, 122.0, 24.0),
        "Registry",
        ctx.connection == ConnectionState::Online,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("registry".to_owned()));
    }
    if virtual_button(
        Rect::new(content.x + 260.0, content.y + 438.0, 122.0, 24.0),
        "Order",
        ctx.connection == ConnectionState::Online,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("order".to_owned()));
    }
    if virtual_button(
        Rect::new(content.x, content.y + 467.0, 122.0, 24.0),
        "Knowledge",
        ctx.connection == ConnectionState::Online,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("knowledge".to_owned()));
    }
    if virtual_button(
        Rect::new(content.x + 130.0, content.y + 467.0, 122.0, 24.0),
        "Households",
        ctx.connection == ConnectionState::Online,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("households".to_owned()));
    }
    if virtual_button(
        Rect::new(content.x + 260.0, content.y + 467.0, 122.0, 24.0),
        "Local fight",
        ctx.connection == ConnectionState::Online && !ctx.knocked_out,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("local-fight".to_owned()));
    }

    for (index, (id, label)) in [
        ("travel", "Travel"),
        ("recover-travel", "Recover"),
        ("market-region", "Market"),
        ("region-event", "Event"),
    ]
    .iter()
    .enumerate()
    {
        if virtual_button(
            Rect::new(
                content.x + index as f32 * 88.0,
                content.y + 496.0,
                82.0,
                25.0,
            ),
            label,
            ctx.connection == ConnectionState::Online,
            ButtonTone::Primary,
            mouse,
        ) {
            actions.push(UiAction::Interact((*id).to_owned()));
        }
    }
    for (index, (id, label)) in [
        ("account", "Account"),
        ("logout", "Logout"),
        ("report", "Report"),
    ]
    .iter()
    .enumerate()
    {
        if virtual_button(
            Rect::new(
                content.x + index as f32 * 88.0,
                content.y + 525.0,
                82.0,
                25.0,
            ),
            label,
            ctx.connection == ConnectionState::Online,
            ButtonTone::Secondary,
            mouse,
        ) {
            actions.push(UiAction::Interact((*id).to_owned()));
        }
    }
    if virtual_button(
        Rect::new(content.x, content.y + 554.0, content.w, 25.0),
        "Reconnect",
        ctx.connection != ConnectionState::Online,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::Reconnect);
    }
    if virtual_button(
        Rect::new(content.x, content.y + 583.0, content.w, 25.0),
        "Use offline fixture",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::UseOffline);
    }
    draw_text_block(
        &format!(
            "{}\n{}\n{}\n{}",
            ctx.status_message,
            ctx.phase4_summary,
            ctx.phase5_summary,
            ctx.chronicle
                .last()
                .map(|entry| entry.title.as_str())
                .or_else(|| ctx
                    .opportunities
                    .first()
                    .map(|opportunity| opportunity.clue.as_str()))
                .unwrap_or("The frontier registry is listening.")
        ),
        content.x,
        content.y + 612.0,
        content.w,
        28.0,
        11.0,
        2.0,
        dark::TEXT_DIM,
    );
}
