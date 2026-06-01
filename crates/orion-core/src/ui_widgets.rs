use crate::font::draw_text;
use crate::render::{fill_rect, DisplaySink};
use crate::theme;

pub fn draw_option_row(
    display: &mut impl DisplaySink,
    x: i16,
    y: i16,
    label: &str,
    value: &str,
    selected: bool,
) {
    fill_rect(
        display,
        x,
        y,
        236,
        30,
        if selected { theme::OVERLAY } else { theme::HUD },
    );
    if selected {
        fill_rect(display, x + 8, y + 7, 6, 16, theme::ACCENT);
    }

    draw_text(
        display,
        x + 24,
        y + 9,
        label,
        if selected { theme::TEXT } else { theme::MUTED },
        1,
    );
    draw_text(display, x + 120, y + 9, value, theme::TEXT, 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{DrawCommand, RecordingDisplay};

    #[test]
    fn selected_row_draws_overlay_and_accent() {
        let mut display = RecordingDisplay::new();
        draw_option_row(&mut display, 42, 88, "MODE", "PRACTICE", true);
        assert!(matches!(
            display.commands()[0],
            DrawCommand::Fill {
                rect: crate::render::Rect {
                    x: 42,
                    y: 88,
                    w: 236,
                    h: 30
                },
                color: theme::OVERLAY
            }
        ));
        assert!(display.commands().iter().any(|command| matches!(
            command,
            DrawCommand::Fill {
                rect: crate::render::Rect {
                    x: 50,
                    y: 95,
                    w: 6,
                    h: 16
                },
                color: theme::ACCENT
            }
        )));
    }
}
