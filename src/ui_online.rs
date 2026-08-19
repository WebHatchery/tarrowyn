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
        "Settlement chat",
        content.x,
        content.y + 235.0,
        TextStyle::new(16.0, CREAM).params(),
    );
    draw_surface(
        Rect::new(content.x, content.y + 246.0, content.w, 72.0),
        &SurfaceStyle::new(Color::new(0.075, 0.105, 0.115, 1.0))
            .with_border(1.0, Color::new(0.32, 0.48, 0.50, 0.45)),
    );
    for (index, message) in ctx.chat.iter().rev().take(3).enumerate() {
        draw_ui_text_ex(
            &format!("{}: {}", message.display_name, message.text),
            content.x + 8.0,
            content.y + 262.0 + index as f32 * 19.0,
            TextStyle::new(11.0, if index == 0 { CREAM } else { dark::TEXT_DIM }).params(),
        );
    }

    draw_surface(
        Rect::new(content.x, content.y + 328.0, content.w - 78.0, 30.0),
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
        content.y + 348.0,
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
        Rect::new(content.right() - 70.0, content.y + 328.0, 70.0, 30.0),
        "Send",
        !ctx.chat_draft.trim().is_empty() && ctx.connection == ConnectionState::Online,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::SendChat);
    }

    for (index, phrase) in [
        "Hello from the road",
        "Meet at the Hearth",
        "I found the path",
    ]
    .iter()
    .enumerate()
    {
        if virtual_button(
            Rect::new(
                content.x,
                content.y + 366.0 + index as f32 * 29.0,
                content.w,
                24.0,
            ),
            phrase,
            ctx.connection == ConnectionState::Online,
            ButtonTone::Secondary,
            mouse,
        ) {
            actions.push(UiAction::QuickChat((*phrase).to_owned()));
        }
    }

    if virtual_button(
        Rect::new(content.x, content.y + 430.0, content.w, 25.0),
        "Reconnect",
        ctx.connection != ConnectionState::Online,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::Reconnect);
    }
    if virtual_button(
        Rect::new(content.x, content.y + 459.0, content.w, 25.0),
        "Use offline fixture",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::UseOffline);
    }
    draw_text_block(
        ctx.status_message,
        content.x,
        content.y + 487.0,
        content.w,
        28.0,
        11.0,
        2.0,
        dark::TEXT_DIM,
    );
}
