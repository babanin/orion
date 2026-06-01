use crate::config::{TFT_H_RES, TFT_V_RES};
use crate::theme;
use crate::ui_widgets::draw_option_row;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawCommand {
    Fill {
        rect: Rect,
        color: u16,
    },
    Bitmap {
        x: i16,
        y: i16,
        w: u16,
        h: u16,
        offset: u32,
    },
    Flush,
}

pub trait DisplaySink {
    fn push(&mut self, command: DrawCommand);
}

pub fn fill_rect(display: &mut impl DisplaySink, x: i16, y: i16, w: i16, h: i16, color: u16) {
    if w <= 0 || h <= 0 {
        return;
    }
    display.push(DrawCommand::Fill {
        rect: Rect { x, y, w, h },
        color,
    });
}

pub fn clear(display: &mut impl DisplaySink, color: u16) {
    fill_rect(display, 0, 0, TFT_H_RES, TFT_V_RES, color);
}

pub fn draw_bitmap(display: &mut impl DisplaySink, x: i16, y: i16, w: u16, h: u16, offset: u32) {
    display.push(DrawCommand::Bitmap { x, y, w, h, offset });
}

pub fn flush(display: &mut impl DisplaySink) {
    display.push(DrawCommand::Flush);
}

#[derive(Debug, Clone, Default)]
pub struct RecordingDisplay {
    commands: Vec<DrawCommand>,
}

impl RecordingDisplay {
    pub const fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

impl DisplaySink for RecordingDisplay {
    fn push(&mut self, command: DrawCommand) {
        self.commands.push(command);
    }
}

pub fn render_launcher<const N: usize>(
    display: &mut impl DisplaySink,
    titles: [&'static str; N],
    selected: usize,
) {
    clear(display, theme::BG);
    crate::font::draw_text(display, 116, 28, "ORION", theme::TEXT, 3);
    crate::font::draw_text(display, 70, 70, "SELECT APP", theme::MUTED, 1);

    let start_y = if N > 3 { 82 } else { 102 };
    let row_gap = if N > 3 { 32 } else { 40 };
    for (index, title) in titles.into_iter().enumerate() {
        draw_option_row(
            display,
            42,
            start_y + index as i16 * row_gap,
            "APP",
            title,
            selected == index,
        );
    }
    crate::font::draw_text(display, 70, 204, "UD OR KNOB SELECT", theme::MUTED, 1);
    crate::font::draw_text(display, 70, 222, "PRESS SW TO OPEN", theme::TEXT, 1);
    flush(display);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_renderer_records_full_screen_redraw() {
        let mut display = RecordingDisplay::new();
        render_launcher(&mut display, ["Flags", "Snake"], 1);
        assert!(matches!(display.commands()[0], DrawCommand::Fill { .. }));
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
        assert!(display.commands().len() > 4);
    }
}
