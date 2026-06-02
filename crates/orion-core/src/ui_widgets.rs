use crate::font::draw_text;
use crate::render::{fill_rect, DisplaySink};
use crate::theme;

pub fn draw_option_row(
    display: &mut impl DisplaySink,
    x: i16,
    y: i16,
    width: i16,
    label: &str,
    value: &str,
    selected: bool,
) {
    fill_rect(
        display,
        x,
        y,
        width,
        30,
        if selected { theme::OVERLAY } else { theme::HUD },
    );
    if selected {
        fill_rect(display, x + 2, y + 7, 4, 16, theme::ACCENT);
    }

    if label.is_empty() {
        draw_text(display, x + 26, y + 9, value, theme::TEXT, 1);
    } else {
        draw_text(
            display,
            x + 12,
            y + 9,
            label,
            if selected { theme::TEXT } else { theme::MUTED },
            1,
        );
        draw_text(display, x + 86, y + 9, value, theme::TEXT, 1);
    }
}

pub fn draw_menu_button(display: &mut impl DisplaySink, x: i16, y: i16, selected: bool) {
    fill_rect(
        display,
        x,
        y,
        104,
        28,
        if selected { theme::ACCENT } else { theme::HUD },
    );
    fill_rect(display, x + 12, y + 8, 12, 2, theme::TEXT);
    fill_rect(display, x + 12, y + 13, 12, 2, theme::TEXT);
    fill_rect(display, x + 12, y + 18, 12, 2, theme::TEXT);
    draw_text(
        display,
        x + 36,
        y + 10,
        "MENU",
        if selected { theme::BG } else { theme::TEXT },
        1,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{DrawCommand, RecordingDisplay};

    #[test]
    fn selected_row_draws_overlay_and_accent() {
        let mut display = RecordingDisplay::new();
        draw_option_row(&mut display, 42, 88, 94, "MODE", "PRACTICE", true);
        assert!(matches!(
            display.commands()[0],
            DrawCommand::Fill {
                rect: crate::render::Rect {
                    x: 42,
                    y: 88,
                    w: 94,
                    h: 30
                },
                color: theme::OVERLAY
            }
        ));
        assert!(display.commands().iter().any(|command| matches!(
            command,
            DrawCommand::Fill {
                rect: crate::render::Rect {
                    x: 44,
                    y: 95,
                    w: 4,
                    h: 16
                },
                color: theme::ACCENT
            }
        )));
    }

    #[test]
    fn labeled_row_draws_value_text() {
        let mut label_only = RecordingDisplay::new();
        draw_option_row(&mut label_only, 42, 88, 236, "MODE", "", false);

        let mut with_value = RecordingDisplay::new();
        draw_option_row(&mut with_value, 42, 88, 236, "MODE", "PRACTICE", false);

        assert!(with_value.commands().len() > label_only.commands().len());
    }

    #[test]
    fn menu_button_draws_button_and_icon() {
        let mut display = RecordingDisplay::new();
        draw_menu_button(&mut display, 108, 204, true);
        assert!(matches!(
            display.commands()[0],
            DrawCommand::Fill {
                rect: crate::render::Rect {
                    x: 108,
                    y: 204,
                    w: 104,
                    h: 28
                },
                color: theme::ACCENT
            }
        ));
    }
}
