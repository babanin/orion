use core::fmt::Write;

use crate::config::{MENU_COLS, TFT_H_RES, TFT_V_RES};
use crate::launcher::{HomeSelection, HomeSnapshot, LauncherView};
use crate::theme;
use crate::ui_widgets::{draw_home_button, draw_option_row};

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
        data_len: u32,
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

pub fn draw_bitmap(
    display: &mut impl DisplaySink,
    x: i16,
    y: i16,
    w: u16,
    h: u16,
    offset: u32,
    data_len: u32,
) {
    display.push(DrawCommand::Bitmap {
        x,
        y,
        w,
        h,
        offset,
        data_len,
    });
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

pub fn render_launcher<const G: usize, const A: usize>(
    display: &mut impl DisplaySink,
    game_titles: [&'static str; G],
    app_titles: [&'static str; A],
    view: LauncherView,
    selected: usize,
    home_selection: HomeSelection,
    home: HomeSnapshot,
) {
    match view {
        LauncherView::Home => render_home(display, home, home_selection),
        LauncherView::GameMenu => render_game_menu(display, game_titles, selected),
        LauncherView::AppMenu => render_app_menu(display, app_titles, selected),
    }
}

pub fn render_home(display: &mut impl DisplaySink, home: HomeSnapshot, selected: HomeSelection) {
    clear(display, theme::BG);
    crate::font::draw_centered_text(
        display,
        0,
        20,
        TFT_H_RES,
        "Glebchinskiy Games",
        theme::TEXT,
        3,
    );
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
    draw_home_button(display, 48, 204, "GAMES", selected == HomeSelection::Games);
    draw_home_button(display, 168, 204, "APPS", selected == HomeSelection::Apps);
    flush(display);
}

pub fn render_game_menu<const N: usize>(
    display: &mut impl DisplaySink,
    titles: [&'static str; N],
    selected: usize,
) {
    render_menu(display, "GAMES", titles, selected, false);
}

pub fn render_app_menu<const N: usize>(
    display: &mut impl DisplaySink,
    titles: [&'static str; N],
    selected: usize,
) {
    render_menu(display, "APPS", titles, selected, true);
}

fn render_menu<const N: usize>(
    display: &mut impl DisplaySink,
    heading: &str,
    titles: [&'static str; N],
    selected: usize,
    app_icons: bool,
) {
    const COL_W: i16 = 94;
    const COL_GAP: i16 = 6;
    const ROW_H: i16 = 30;
    const ROW_GAP: i16 = 6;
    const TOTAL_W: i16 = COL_W * MENU_COLS as i16 + COL_GAP * (MENU_COLS as i16 - 1);
    const MARGIN_X: i16 = (TFT_H_RES - TOTAL_W) / 2;

    clear(display, theme::BG);
    crate::font::draw_centered_text(display, 0, 18, TFT_H_RES, heading, theme::TEXT, 2);
    let rows = (N + MENU_COLS - 1) / MENU_COLS;
    let content_h = rows as i16 * ROW_H + (rows as i16 - 1) * ROW_GAP;
    let start_y = 58 + (168 - content_h) / 2;
    for (index, title) in titles.into_iter().enumerate() {
        let col = index % MENU_COLS;
        let row = index / MENU_COLS;
        let x = MARGIN_X + col as i16 * (COL_W + COL_GAP);
        let y = start_y + row as i16 * (ROW_H + ROW_GAP);
        draw_option_row(display, x, y, COL_W, "", title, selected == index);
        if app_icons {
            draw_app_icon(display, x + 8, y + 7, 16, index, title, selected == index);
        } else {
            draw_game_icon(display, x + 8, y + 7, 16, index, title, selected == index);
        }
    }
    crate::font::draw_text(display, 70, 218, "UDLR OR KNOB SELECT", theme::MUTED, 1);
    crate::font::draw_text(
        display,
        70,
        230,
        "PRESS TO OPEN  HOLD TO HOME",
        theme::TEXT,
        1,
    );
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

fn draw_game_icon(
    display: &mut impl DisplaySink,
    x: i16,
    y: i16,
    size: i16,
    index: usize,
    title: &str,
    selected: bool,
) {
    let fg = if selected { theme::TEXT } else { theme::MUTED };
    let s = size;
    fill_rect(display, x, y, s, s, theme::GRID);
    if title == "HOME" {
        draw_home_icon(display, x, y, s, fg);
        return;
    }
    match index {
        0 => {
            fill_rect(display, x + s / 5, y + s / 5, s / 10, 3 * s / 5, fg);
            fill_rect(
                display,
                x + 3 * s / 10,
                y + s / 5,
                s / 2,
                s / 5,
                theme::ACCENT,
            );
            fill_rect(
                display,
                x + 3 * s / 10,
                y + 2 * s / 5,
                s / 2,
                s / 5,
                theme::TEXT,
            );
            fill_rect(
                display,
                x + 3 * s / 10,
                y + 3 * s / 5,
                s / 2,
                s / 5,
                theme::BAD,
            );
        }
        1 => {
            fill_rect(display, x + s / 5, y + s / 2, s / 5, s / 5, theme::SNAKE);
            fill_rect(
                display,
                x + 2 * s / 5,
                y + s / 2,
                s / 5,
                s / 5,
                theme::SNAKE,
            );
            fill_rect(
                display,
                x + 3 * s / 5,
                y + 3 * s / 10,
                s / 4,
                2 * s / 5,
                theme::HEAD,
            );
            fill_rect(display, x + s / 4, y + s / 4, s / 6, s / 6, theme::APPLE);
        }
        2 => {
            fill_rect(
                display,
                x + s / 5,
                y + s / 5,
                3 * s / 5,
                3 * s / 5,
                theme::ACCENT,
            );
            crate::font::draw_text(display, x + s / 4, y + 7 * s / 20, "2", theme::BG, 1);
        }
        3 => {
            fill_rect(
                display,
                x + 3 * s / 10,
                y + s / 5,
                s / 5,
                s / 5,
                theme::GOOD,
            );
            fill_rect(
                display,
                x + 3 * s / 10,
                y + 2 * s / 5,
                s / 5,
                s / 5,
                theme::GOOD,
            );
            fill_rect(
                display,
                x + 3 * s / 10,
                y + 3 * s / 5,
                s / 5,
                s / 5,
                theme::GOOD,
            );
            fill_rect(display, x + s / 2, y + 3 * s / 5, s / 5, s / 5, theme::GOOD);
        }
        4 => {
            fill_rect(
                display,
                x + s / 4,
                y + s / 5,
                s / 2,
                s / 10,
                theme::HEAD_MARK,
            );
            fill_rect(
                display,
                x + s / 5,
                y + 3 * s / 10,
                3 * s / 5,
                s / 2,
                theme::HEAD,
            );
            fill_rect(
                display,
                x + 3 * s / 10,
                y + 2 * s / 5,
                s / 6,
                s / 6,
                theme::EYE,
            );
            fill_rect(
                display,
                x + 3 * s / 5,
                y + 2 * s / 5,
                s / 6,
                s / 6,
                theme::EYE,
            );
            fill_rect(
                display,
                x + 2 * s / 5,
                y + 3 * s / 5,
                s / 4,
                s / 8,
                theme::BAD,
            );
        }
        _ => draw_home_icon(display, x, y, s, fg),
    }
}

fn draw_app_icon(
    display: &mut impl DisplaySink,
    x: i16,
    y: i16,
    size: i16,
    _index: usize,
    title: &str,
    selected: bool,
) {
    let fg = if selected { theme::TEXT } else { theme::MUTED };
    let s = size;
    fill_rect(display, x, y, s, s, theme::GRID);
    if title == "HOME" {
        draw_home_icon(display, x, y, s, fg);
        return;
    }
    fill_rect(display, x + 3, y + 5, s - 6, s - 5, theme::APPLE);
    fill_rect(display, x + 6, y + 3, s - 12, 3, theme::LEAF);
    fill_rect(display, x + 5, y + 9, 2, 2, theme::EYE);
    fill_rect(display, x + s - 7, y + 9, 2, 2, theme::EYE);
    fill_rect(display, x + 6, y + 12, s - 12, 2, fg);
}

fn draw_home_icon(display: &mut impl DisplaySink, x: i16, y: i16, s: i16, fg: u16) {
    fill_rect(
        display,
        x + 3 * s / 20,
        y + 7 * s / 20,
        7 * s / 10,
        2 * s / 5,
        fg,
    );
    fill_rect(display, x + s / 4, y + 11 * s / 20, s / 5, s / 5, theme::BG);
    fill_rect(
        display,
        x + 3 * s / 20,
        y + s / 4,
        7 * s / 20,
        3 * s / 20,
        fg,
    );
    fill_rect(display, x + s / 2, y + s / 4, 7 * s / 20, 3 * s / 20, fg);
    fill_rect(display, x + s / 2, y + 3 * s / 10, s / 5, s / 10, fg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_renderer_records_full_screen_redraw() {
        let mut display = RecordingDisplay::new();
        render_home(&mut display, HomeSnapshot::default(), HomeSelection::Games);
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

    #[test]
    fn render_launcher_dispatches_to_home() {
        let mut display = RecordingDisplay::new();
        let home = HomeSnapshot::default();
        render_launcher(
            &mut display,
            ["Flags", "Snake"],
            ["POMODORO", "HOME"],
            LauncherView::Home,
            0,
            HomeSelection::Games,
            home,
        );
        assert!(matches!(display.commands()[0], DrawCommand::Fill { .. }));
    }

    #[test]
    fn render_launcher_dispatches_to_game_menu() {
        let mut display = RecordingDisplay::new();
        let home = HomeSnapshot::default();
        render_launcher(
            &mut display,
            ["Flags", "Snake"],
            ["POMODORO", "HOME"],
            LauncherView::GameMenu,
            0,
            HomeSelection::Games,
            home,
        );
        assert!(matches!(display.commands()[0], DrawCommand::Fill { .. }));
    }

    #[test]
    fn render_launcher_dispatches_to_app_menu() {
        let mut display = RecordingDisplay::new();
        let home = HomeSnapshot::default();
        render_launcher(
            &mut display,
            ["Flags", "Snake"],
            ["POMODORO", "HOME"],
            LauncherView::AppMenu,
            0,
            HomeSelection::Apps,
            home,
        );
        assert!(matches!(display.commands()[0], DrawCommand::Fill { .. }));
    }

    #[test]
    fn render_home_with_time() {
        let mut display = RecordingDisplay::new();
        let home = HomeSnapshot {
            time: Some(crate::launcher::ClockTime::new(14, 30)),
            date: Some(crate::launcher::CalendarDate::new(2025, 6, 15)),
            temperature_tenths_c: Some(225),
            status: crate::launcher::HomeStatus::Ready,
        };
        render_home(&mut display, home, HomeSelection::Games);
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_home_with_no_time() {
        let mut display = RecordingDisplay::new();
        let home = HomeSnapshot {
            time: None,
            date: None,
            temperature_tenths_c: None,
            status: crate::launcher::HomeStatus::Wifi,
        };
        render_home(&mut display, home, HomeSelection::Apps);
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_home_with_negative_temperature() {
        let mut display = RecordingDisplay::new();
        let home = HomeSnapshot {
            time: Some(crate::launcher::ClockTime::new(9, 5)),
            date: Some(crate::launcher::CalendarDate::new(2025, 1, 3)),
            temperature_tenths_c: Some(-55),
            status: crate::launcher::HomeStatus::Ready,
        };
        render_home(&mut display, home, HomeSelection::Games);
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_game_menu_with_many_items() {
        let mut display = RecordingDisplay::new();
        render_game_menu(
            &mut display,
            ["FLAGS", "SNAKE", "2048", "TETRIS", "OM NOM", "HOME"],
            0,
        );
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_app_menu_with_pomodoro() {
        let mut display = RecordingDisplay::new();
        render_app_menu(&mut display, ["POMODORO", "HOME"], 0);
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
        assert!(display.commands().iter().any(|command| matches!(
            command,
            DrawCommand::Fill {
                color: theme::APPLE,
                ..
            }
        )));
    }

    #[test]
    fn game_menu_icons_cover_all_indices() {
        for i in 0..6 {
            let mut display = RecordingDisplay::new();
            draw_game_icon(&mut display, 10, 10, 16, i, "APP", true);
            assert!(!display.commands().is_empty());
        }
        let mut display = RecordingDisplay::new();
        draw_game_icon(&mut display, 10, 10, 16, 99, "HOME", true);
        assert!(!display.commands().is_empty());
    }

    #[test]
    fn game_menu_icon_unselected_color() {
        let mut display = RecordingDisplay::new();
        draw_game_icon(&mut display, 10, 10, 16, 0, "FLAGS", false);
        assert!(!display.commands().is_empty());
    }

    #[test]
    fn fill_rect_skips_zero_or_negative_dimensions() {
        let mut display = RecordingDisplay::new();
        fill_rect(&mut display, 0, 0, 0, 10, theme::BG);
        assert!(display.commands().is_empty());
        fill_rect(&mut display, 0, 0, 10, -1, theme::BG);
        assert!(display.commands().is_empty());
    }

    #[test]
    fn fill_rect_draws_for_positive_dimensions() {
        let mut display = RecordingDisplay::new();
        fill_rect(&mut display, 0, 0, 10, 20, theme::BG);
        assert_eq!(display.commands().len(), 1);
    }

    #[test]
    fn draw_bitmap_pushes_bitmap_command() {
        let mut display = RecordingDisplay::new();
        draw_bitmap(&mut display, 10, 20, 30, 40, 100, 200);
        assert_eq!(display.commands().len(), 1);
        assert!(matches!(
            display.commands()[0],
            DrawCommand::Bitmap {
                x: 10,
                y: 20,
                w: 30,
                h: 40,
                offset: 100,
                data_len: 200
            }
        ));
    }

    #[test]
    fn flush_pushes_flush_command() {
        let mut display = RecordingDisplay::new();
        flush(&mut display);
        assert_eq!(display.commands().len(), 1);
        assert!(matches!(display.commands()[0], DrawCommand::Flush));
    }

    #[test]
    fn clear_fills_entire_screen() {
        let mut display = RecordingDisplay::new();
        clear(&mut display, theme::BG);
        assert_eq!(display.commands().len(), 1);
        assert!(matches!(
            display.commands()[0],
            DrawCommand::Fill {
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: TFT_H_RES,
                    h: TFT_V_RES
                },
                color: theme::BG
            }
        ));
    }

    #[test]
    fn recording_display_clear_works() {
        let mut display = RecordingDisplay::new();
        fill_rect(&mut display, 0, 0, 10, 10, theme::BG);
        assert_eq!(display.commands().len(), 1);
        display.clear();
        assert!(display.commands().is_empty());
    }
}
