use core::fmt::Write;

use crate::config::{TFT_H_RES, TFT_V_RES};
use crate::launcher::{HomeSnapshot, LauncherView};
use crate::theme;
use crate::ui_widgets::{draw_menu_button, draw_option_row};

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
    view: LauncherView,
    selected: usize,
    home: HomeSnapshot,
) {
    match view {
        LauncherView::Home => render_home(display, home),
        LauncherView::GameMenu => render_game_menu(display, titles, selected),
    }
}

pub fn render_home(display: &mut impl DisplaySink, home: HomeSnapshot) {
    clear(display, theme::BG);
    crate::font::draw_centered_text(display, 0, 20, TFT_H_RES, "ORION", theme::TEXT, 3);
    crate::font::draw_centered_text(
        display,
        0,
        54,
        TFT_H_RES,
        "SAINT PETERSBURG",
        theme::MUTED,
        1,
    );

    fill_rect(display, 24, 78, 272, 84, theme::HUD);

    let mut time = crate::font::TextBuffer::<8>::new();
    if let Some(value) = home.time {
        let _ = write!(time, "{:02}:{:02}", value.hour, value.minute);
    } else {
        let _ = write!(time, "--:--");
    }
    crate::font::draw_centered_text(display, 24, 94, 272, time.as_str(), theme::TEXT, 3);

    let mut date = crate::font::TextBuffer::<16>::new();
    if let Some(value) = home.date {
        let _ = write!(
            date,
            "{:04}-{:02}-{:02}",
            value.year, value.month, value.day
        );
    } else {
        let _ = write!(date, "---- -- --");
    }
    crate::font::draw_centered_text(display, 24, 132, 272, date.as_str(), theme::MUTED, 1);

    draw_temperature(display, 40, 176, home);
    draw_status(display, 198, 184, home.status.label());
    draw_menu_button(display, 108, 204, true);
    flush(display);
}

pub fn render_game_menu<const N: usize>(
    display: &mut impl DisplaySink,
    titles: [&'static str; N],
    selected: usize,
) {
    clear(display, theme::BG);
    crate::font::draw_centered_text(display, 0, 18, TFT_H_RES, "GAMES", theme::TEXT, 2);
    crate::font::draw_centered_text(
        display,
        0,
        42,
        TFT_H_RES,
        "PRESS HOLD FOR HOME",
        theme::MUTED,
        1,
    );
    let start_y = if N > 3 { 82 } else { 102 };
    let row_gap = if N > 3 { 32 } else { 40 };
    for (index, title) in titles.into_iter().enumerate() {
        let y = start_y + index as i16 * row_gap;
        draw_option_row(display, 42, y, "", title, selected == index);
        draw_game_icon(display, 56, y + 5, index, selected == index);
    }
    crate::font::draw_text(display, 70, 218, "UD OR KNOB SELECT", theme::MUTED, 1);
    crate::font::draw_text(display, 82, 230, "PRESS SW TO OPEN", theme::TEXT, 1);
    flush(display);
}

fn draw_temperature(display: &mut impl DisplaySink, x: i16, y: i16, home: HomeSnapshot) {
    fill_rect(display, x, y, 128, 20, theme::HUD);
    let mut text = crate::font::TextBuffer::<12>::new();
    if let Some(tenths) = home.temperature_tenths_c {
        let sign = if tenths < 0 { "-" } else { "" };
        let value = tenths.abs();
        let _ = write!(text, "{}{}.{:01}C", sign, value / 10, value % 10);
    } else {
        let _ = write!(text, "--.-C");
    }
    crate::font::draw_text(display, x + 12, y + 6, text.as_str(), theme::TEXT, 1);
}

fn draw_status(display: &mut impl DisplaySink, x: i16, y: i16, status: &str) {
    let color = if status == "ONLINE" {
        theme::GOOD
    } else {
        theme::ACCENT
    };
    fill_rect(display, x, y + 3, 8, 8, color);
    crate::font::draw_text(display, x + 16, y + 4, status, theme::MUTED, 1);
}

fn draw_game_icon(display: &mut impl DisplaySink, x: i16, y: i16, index: usize, selected: bool) {
    let fg = if selected { theme::TEXT } else { theme::MUTED };
    fill_rect(display, x, y, 20, 20, theme::GRID);
    match index {
        0 => {
            fill_rect(display, x + 4, y + 4, 2, 12, fg);
            fill_rect(display, x + 6, y + 4, 10, 4, theme::ACCENT);
            fill_rect(display, x + 6, y + 8, 10, 4, theme::TEXT);
            fill_rect(display, x + 6, y + 12, 10, 4, theme::BAD);
        }
        1 => {
            fill_rect(display, x + 4, y + 10, 4, 4, theme::SNAKE);
            fill_rect(display, x + 8, y + 10, 4, 4, theme::SNAKE);
            fill_rect(display, x + 12, y + 6, 4, 8, theme::HEAD);
            fill_rect(display, x + 5, y + 5, 3, 3, theme::APPLE);
        }
        2 => {
            fill_rect(display, x + 4, y + 4, 12, 12, theme::ACCENT);
            crate::font::draw_text(display, x + 5, y + 7, "2", theme::BG, 1);
        }
        _ => {
            fill_rect(display, x + 6, y + 4, 4, 4, theme::GOOD);
            fill_rect(display, x + 6, y + 8, 4, 4, theme::GOOD);
            fill_rect(display, x + 6, y + 12, 4, 4, theme::GOOD);
            fill_rect(display, x + 10, y + 12, 4, 4, theme::GOOD);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_renderer_records_full_screen_redraw() {
        let mut display = RecordingDisplay::new();
        render_home(&mut display, HomeSnapshot::default());
        assert!(matches!(display.commands()[0], DrawCommand::Fill { .. }));
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
        assert!(display.commands().len() > 8);
    }

    #[test]
    fn game_menu_renderer_records_full_screen_redraw() {
        let mut display = RecordingDisplay::new();
        render_game_menu(&mut display, ["Flags", "Snake"], 1);
        assert!(matches!(display.commands()[0], DrawCommand::Fill { .. }));
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
        assert!(display.commands().len() > 4);
    }
}
