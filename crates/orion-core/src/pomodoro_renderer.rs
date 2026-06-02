use core::fmt::Write;

use crate::config::TFT_H_RES;
use crate::font::{draw_centered_text, TextBuffer};
use crate::pomodoro::{PomodoroField, PomodoroMode, PomodoroPauseAction, PomodoroTimer};
use crate::render::{clear, fill_rect, flush, DisplaySink};
use crate::theme;
use crate::ui_widgets::draw_option_row;

const RUNNING_TIME_X: i16 = 12;
const RUNNING_TIME_Y: i16 = 60;
const RUNNING_TIME_W: i16 = 296;
const RUNNING_TIME_H: i16 = 96;
const SEGMENT_DIGIT_W: i16 = 50;
const SEGMENT_DIGIT_H: i16 = 78;
const SEGMENT_THICKNESS: i16 = 8;
const SEGMENT_GAP: i16 = 5;
const COLON_W: i16 = 16;

pub fn render(display: &mut impl DisplaySink, timer: &PomodoroTimer) {
    match timer.mode() {
        PomodoroMode::Setup => render_setup(display, timer),
        PomodoroMode::Running => render_running(display, timer),
        PomodoroMode::Paused => render_paused(display, timer),
        PomodoroMode::Finished => render_finished(display, timer),
    }
}

pub fn render_setup(display: &mut impl DisplaySink, timer: &PomodoroTimer) {
    clear(display, theme::BG);
    draw_centered_text(display, 0, 14, TFT_H_RES, "POMODORO", theme::TEXT, 2);
    draw_tomato_logo(display, 116, 44, 4);
    draw_time_editor(display, timer, 116);
    draw_option_row(display, 108, 168, 104, "", "START", timer.can_start());
    draw_centered_text(display, 0, 218, TFT_H_RES, "HOLD TO APPS", theme::MUTED, 1);
    flush(display);
}

pub fn render_running(display: &mut impl DisplaySink, timer: &PomodoroTimer) {
    clear(display, theme::BG);
    draw_centered_text(display, 0, 22, TFT_H_RES, "POMODORO", theme::MUTED, 1);
    fill_rect(
        display,
        RUNNING_TIME_X,
        RUNNING_TIME_Y,
        RUNNING_TIME_W,
        RUNNING_TIME_H,
        theme::HUD,
    );
    draw_large_remaining(display, timer.remaining_seconds());
    draw_centered_text(display, 0, 174, TFT_H_RES, "RUNNING", theme::GOOD, 1);
    draw_centered_text(
        display,
        0,
        218,
        TFT_H_RES,
        "PRESS PAUSE  HOLD APPS",
        theme::MUTED,
        1,
    );
    flush(display);
}

pub fn render_running_time_delta(display: &mut impl DisplaySink, timer: &PomodoroTimer) {
    fill_rect(
        display,
        RUNNING_TIME_X,
        RUNNING_TIME_Y,
        RUNNING_TIME_W,
        RUNNING_TIME_H,
        theme::HUD,
    );
    draw_large_remaining(display, timer.remaining_seconds());
    flush(display);
}

pub fn render_paused(display: &mut impl DisplaySink, timer: &PomodoroTimer) {
    clear(display, theme::BG);
    draw_centered_text(display, 0, 20, TFT_H_RES, "PAUSED", theme::TEXT, 2);
    draw_remaining(
        display,
        0,
        72,
        TFT_H_RES,
        timer.remaining_seconds(),
        theme::MUTED,
        2,
    );
    draw_option_row(
        display,
        92,
        134,
        136,
        "",
        "CONTINUE",
        timer.pause_action() == PomodoroPauseAction::Continue,
    );
    draw_option_row(
        display,
        92,
        172,
        136,
        "",
        "EXIT",
        timer.pause_action() == PomodoroPauseAction::Exit,
    );
    draw_centered_text(display, 0, 218, TFT_H_RES, "HOLD TO APPS", theme::MUTED, 1);
    flush(display);
}

pub fn render_finished(display: &mut impl DisplaySink, _timer: &PomodoroTimer) {
    clear(display, theme::BG);
    draw_tomato_logo(display, 116, 36, 4);
    draw_centered_text(display, 0, 130, TFT_H_RES, "DONE", theme::GOOD, 2);
    draw_centered_text(display, 0, 166, TFT_H_RES, "TIME IS UP", theme::TEXT, 1);
    draw_centered_text(
        display,
        0,
        218,
        TFT_H_RES,
        "PRESS TO RESET",
        theme::MUTED,
        1,
    );
    flush(display);
}

pub fn draw_tomato_logo(display: &mut impl DisplaySink, x: i16, y: i16, scale: i16) {
    const PIXELS: [&str; 16] = [
        "......LLSS......",
        ".....LLLLSS.....",
        "....LLGLLSS.....",
        ".....GGGGG......",
        "...RRRRRRRRRR...",
        "..RRRRRRRRRRRR..",
        ".RRRRRRRRRRRRRR.",
        ".RRWWRRRRRRWWRR.",
        "RRRWWRRRRRRWWRRR",
        "RRRRRRRRRRRRRRRR",
        "RRRRKKRRRRKKRRRR",
        "RRRRKKRRRRKKRRRR",
        "RRRRRRRRRRRRRRRR",
        ".RRRKKKKKKKKRRR.",
        "..RRRKKKKKKRRR..",
        "...RRRRRRRRRR...",
    ];

    for (row, line) in PIXELS.iter().enumerate() {
        for (col, pixel) in line.as_bytes().iter().enumerate() {
            let color = match *pixel {
                b'R' => theme::APPLE,
                b'W' => theme::APPLE_HIGHLIGHT,
                b'K' => theme::EYE,
                b'L' => theme::LEAF,
                b'G' => theme::GOOD,
                b'S' => theme::STEM,
                _ => continue,
            };
            fill_rect(
                display,
                x + col as i16 * scale,
                y + row as i16 * scale,
                scale,
                scale,
                color,
            );
        }
    }
}

fn draw_time_editor(display: &mut impl DisplaySink, timer: &PomodoroTimer, y: i16) {
    fill_rect(display, 56, y - 14, 208, 44, theme::HUD);
    if timer.active_field() == PomodoroField::Minutes {
        fill_rect(display, 82, y + 22, 62, 4, theme::ACCENT);
    } else {
        fill_rect(display, 176, y + 22, 62, 4, theme::ACCENT);
    }
    let mut text = TextBuffer::<8>::new();
    let _ = write!(text, "{:02}:{:02}", timer.minutes(), timer.seconds());
    draw_centered_text(display, 0, y, TFT_H_RES, text.as_str(), theme::TEXT, 2);
}

fn draw_remaining(
    display: &mut impl DisplaySink,
    x: i16,
    y: i16,
    w: i16,
    remaining_seconds: u16,
    color: u16,
    scale: i16,
) {
    let mut text = TextBuffer::<8>::new();
    let _ = write!(
        text,
        "{:02}:{:02}",
        remaining_seconds / 60,
        remaining_seconds % 60
    );
    draw_centered_text(display, x, y, w, text.as_str(), color, scale);
}

fn draw_large_remaining(display: &mut impl DisplaySink, remaining_seconds: u16) {
    let minutes = remaining_seconds / 60;
    let seconds = remaining_seconds % 60;
    let digits = [
        (minutes / 10) as u8,
        (minutes % 10) as u8,
        (seconds / 10) as u8,
        (seconds % 10) as u8,
    ];
    let total_w = SEGMENT_DIGIT_W * 4 + SEGMENT_GAP * 4 + COLON_W;
    let mut x = RUNNING_TIME_X + (RUNNING_TIME_W - total_w) / 2;
    let y = RUNNING_TIME_Y + (RUNNING_TIME_H - SEGMENT_DIGIT_H) / 2;

    draw_segment_digit(display, x, y, digits[0], theme::TEXT);
    x += SEGMENT_DIGIT_W + SEGMENT_GAP;
    draw_segment_digit(display, x, y, digits[1], theme::TEXT);
    x += SEGMENT_DIGIT_W + SEGMENT_GAP;
    draw_large_colon(display, x, y, theme::ACCENT);
    x += COLON_W + SEGMENT_GAP;
    draw_segment_digit(display, x, y, digits[2], theme::TEXT);
    x += SEGMENT_DIGIT_W + SEGMENT_GAP;
    draw_segment_digit(display, x, y, digits[3], theme::TEXT);
}

fn draw_segment_digit(display: &mut impl DisplaySink, x: i16, y: i16, digit: u8, color: u16) {
    let segments = match digit {
        0 => 0b011_1111,
        1 => 0b000_0110,
        2 => 0b101_1011,
        3 => 0b100_1111,
        4 => 0b110_0110,
        5 => 0b110_1101,
        6 => 0b111_1101,
        7 => 0b000_0111,
        8 => 0b111_1111,
        9 => 0b110_1111,
        _ => 0,
    };
    if segments & 0b000_0001 != 0 {
        draw_horizontal_segment(display, x, y, color);
    }
    if segments & 0b000_0010 != 0 {
        draw_vertical_segment(display, x + SEGMENT_DIGIT_W - SEGMENT_THICKNESS, y, color);
    }
    if segments & 0b000_0100 != 0 {
        draw_vertical_segment(
            display,
            x + SEGMENT_DIGIT_W - SEGMENT_THICKNESS,
            y + SEGMENT_DIGIT_H / 2,
            color,
        );
    }
    if segments & 0b000_1000 != 0 {
        draw_horizontal_segment(display, x, y + SEGMENT_DIGIT_H - SEGMENT_THICKNESS, color);
    }
    if segments & 0b001_0000 != 0 {
        draw_vertical_segment(display, x, y + SEGMENT_DIGIT_H / 2, color);
    }
    if segments & 0b010_0000 != 0 {
        draw_vertical_segment(display, x, y, color);
    }
    if segments & 0b100_0000 != 0 {
        draw_horizontal_segment(
            display,
            x,
            y + (SEGMENT_DIGIT_H - SEGMENT_THICKNESS) / 2,
            color,
        );
    }
}

fn draw_horizontal_segment(display: &mut impl DisplaySink, x: i16, y: i16, color: u16) {
    fill_rect(
        display,
        x + SEGMENT_THICKNESS / 2,
        y,
        SEGMENT_DIGIT_W - SEGMENT_THICKNESS,
        SEGMENT_THICKNESS,
        color,
    );
    fill_rect(
        display,
        x,
        y + 2,
        SEGMENT_THICKNESS / 2,
        SEGMENT_THICKNESS - 4,
        color,
    );
    fill_rect(
        display,
        x + SEGMENT_DIGIT_W - SEGMENT_THICKNESS / 2,
        y + 2,
        SEGMENT_THICKNESS / 2,
        SEGMENT_THICKNESS - 4,
        color,
    );
}

fn draw_vertical_segment(display: &mut impl DisplaySink, x: i16, y: i16, color: u16) {
    fill_rect(
        display,
        x,
        y + SEGMENT_THICKNESS / 2,
        SEGMENT_THICKNESS,
        SEGMENT_DIGIT_H / 2 - SEGMENT_THICKNESS,
        color,
    );
    fill_rect(
        display,
        x + 2,
        y,
        SEGMENT_THICKNESS - 4,
        SEGMENT_THICKNESS / 2,
        color,
    );
    fill_rect(
        display,
        x + 2,
        y + SEGMENT_DIGIT_H / 2 - SEGMENT_THICKNESS / 2,
        SEGMENT_THICKNESS - 4,
        SEGMENT_THICKNESS / 2,
        color,
    );
}

fn draw_large_colon(display: &mut impl DisplaySink, x: i16, y: i16, color: u16) {
    fill_rect(display, x + 4, y + 21, 8, 8, color);
    fill_rect(display, x + 4, y + 49, 8, 8, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{DrawCommand, RecordingDisplay};

    #[test]
    fn setup_renderer_draws_tomato_and_flushes() {
        let mut display = RecordingDisplay::new();
        render_setup(&mut display, &PomodoroTimer::new());
        assert!(display.commands().iter().any(|command| matches!(
            command,
            DrawCommand::Fill {
                color: theme::APPLE,
                ..
            }
        )));
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn running_paused_and_finished_render() {
        let mut timer = PomodoroTimer::new();
        let mut display = RecordingDisplay::new();
        timer.start(0);
        render_running(&mut display, &timer);
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));

        display.clear();
        timer.pause(1_000_000);
        render_paused(&mut display, &timer);
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));

        display.clear();
        timer.resume(2_000_000);
        timer.update_running(1_502_000_000);
        render_finished(&mut display, &timer);
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn running_time_delta_does_not_clear_full_screen() {
        let mut timer = PomodoroTimer::new();
        let mut display = RecordingDisplay::new();
        timer.start(0);
        timer.update_running(1_000_000);

        render_running_time_delta(&mut display, &timer);

        assert!(!matches!(
            display.commands().first(),
            Some(DrawCommand::Fill {
                rect: crate::render::Rect {
                    x: 0,
                    y: 0,
                    w: crate::config::TFT_H_RES,
                    h: crate::config::TFT_V_RES
                },
                ..
            })
        ));
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn running_render_uses_large_segment_digits() {
        let mut timer = PomodoroTimer::new();
        let mut display = RecordingDisplay::new();
        timer.start(0);

        render_running(&mut display, &timer);

        assert!(display.commands().iter().any(|command| matches!(
            command,
            DrawCommand::Fill {
                rect,
                color: theme::TEXT
            } if rect.w == SEGMENT_DIGIT_W - SEGMENT_THICKNESS && rect.h == SEGMENT_THICKNESS
        )));
        assert!(display.commands().iter().any(|command| matches!(
            command,
            DrawCommand::Fill {
                rect: crate::render::Rect {
                    w: SEGMENT_THICKNESS,
                    h: 31,
                    ..
                },
                color: theme::TEXT
            }
        )));
    }

    #[test]
    fn logo_draws_pixel_art_commands() {
        let mut display = RecordingDisplay::new();
        draw_tomato_logo(&mut display, 10, 20, 2);
        assert!(display.commands().len() > 80);
    }
}
