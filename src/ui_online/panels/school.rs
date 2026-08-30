use super::*;

pub fn draw_school_selection(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    if !ctx.school_selection_open {
        return;
    }
    let panel = Rect::new(42.0, 126.0, 780.0, 438.0);
    draw_surface_with_title(
        panel,
        Some("Open a school lesson"),
        &SurfaceStyle::new(Color::new(0.055, 0.075, 0.09, 0.99))
            .with_border(1.0, Color::new(0.50, 0.72, 0.62, 0.9))
            .with_header(42.0, Color::new(0.09, 0.14, 0.15, 1.0))
            .with_header_divider(1.0, Color::new(0.32, 0.48, 0.50, 0.75)),
        TextStyle::new(17.0, CREAM),
    );
    draw_text_block(
        "Tap a mastered discipline to demonstrate it nearby. A discovered advanced art appears here only when its own requirements are ready.",
        panel.x + 20.0,
        panel.y + 70.0,
        panel.w - 40.0,
        38.0,
        13.0,
        2.0,
        CREAM,
    );
    if ctx.skill_pending {
        draw_ui_text_ex(
            "The school ledger is settling; wait for its response before opening another lesson.",
            panel.x + 20.0,
            panel.y + 124.0,
            TextStyle::new(11.0, dark::TEXT_DIM).params(),
        );
    }
    let choices: Vec<_> = ctx
        .skills
        .iter()
        .filter(|skill| school_teaching_choice(skill))
        .collect();
    if choices.is_empty() {
        draw_ui_text_ex(
            if ctx.skills.is_empty() {
                "The skill ledger is loading; tap Close and try again shortly."
            } else {
                "No mastered or ready advanced discipline is available for a lesson yet."
            },
            panel.x + 20.0,
            panel.y + 145.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
    } else {
        draw_ui_text_ex(
            &format!(
                "{} lesson subject{} ready",
                choices.len(),
                if choices.len() == 1 { "" } else { "s" }
            ),
            panel.x + 20.0,
            panel.y + 135.0,
            TextStyle::new(11.0, MINT).params(),
        );
        draw_school_choices(panel, &choices, ctx.skill_pending, mouse, actions);
    }
    if virtual_button(
        Rect::new(panel.right() - 126.0, panel.bottom() - 42.0, 106.0, 28.0),
        "Close",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::Interact("school-close".to_owned()));
    }
}

fn draw_school_choices(
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
                panel.y + 150.0 + row as f32 * (height + gap),
                width,
                height,
            ),
            &skill.name,
            !skill_pending,
            ButtonTone::Primary,
            mouse,
        ) {
            actions.push(UiAction::Teach(skill.skill_id.clone()));
        }
    }
}

pub fn school_teaching_choice(skill: &tarrowyn_protocol::SkillView) -> bool {
    (skill.depth == 1 && skill.mastery >= 5 && skill.skill_id != "teaching")
        || (skill.depth > 1
            && skill.status == tarrowyn_protocol::SkillStatus::Discovered
            && skill.usable)
}
