use core::fmt::Write;

use crate::font::{draw_centered_text, draw_text, TextBuffer};
use crate::render::{fill_rect, flush, DisplaySink, DrawCommand, Rect};
use crate::tetris::{
    tetromino_cells, TetrisGame, TetrisMode, TetrisPauseAction, TetrisPiece, Tetromino,
    TETRIS_CELLS, TETRIS_COLS, TETRIS_ROWS,
};
use crate::theme;
use crate::ui_widgets::draw_option_row;

const PORTRAIT_W: i16 = 240;
const PORTRAIT_H: i16 = 320;
const WELL_CELL: i16 = 13;
const WELL_X: i16 = 43;
const WELL_Y: i16 = 38;
const WELL_W: i16 = TETRIS_COLS as i16 * WELL_CELL;
const WELL_H: i16 = TETRIS_ROWS as i16 * WELL_CELL;
const MINI_CELL: i16 = 8;

const WELL_BG: u16 = theme::rgb565(7, 10, 14);
const WELL_BORDER: u16 = theme::rgb565(62, 72, 86);
const I_COLOR: u16 = theme::rgb565(58, 211, 230);
const O_COLOR: u16 = theme::rgb565(248, 219, 75);
const T_COLOR: u16 = theme::rgb565(166, 103, 230);
const S_COLOR: u16 = theme::rgb565(65, 195, 96);
const Z_COLOR: u16 = theme::rgb565(231, 76, 83);
const J_COLOR: u16 = theme::rgb565(76, 129, 232);
const L_COLOR: u16 = theme::rgb565(241, 151, 61);
const CELL_INSET: i16 = 1;

pub fn render_choosing(display: &mut impl DisplaySink) {
    let mut display = RotatedClockwise::new(display);
    clear_portrait(&mut display, theme::BG);
    draw_centered_text(&mut display, 0, 38, PORTRAIT_W, "TETRIS", theme::TEXT, 3);
    draw_centered_text(
        &mut display,
        0,
        88,
        PORTRAIT_W,
        "10X20 PORTRAIT WELL",
        theme::ACCENT,
        1,
    );
    draw_centered_text(
        &mut display,
        0,
        140,
        PORTRAIT_W,
        "LR MOVE  UP ROTATE",
        theme::MUTED,
        1,
    );
    draw_centered_text(
        &mut display,
        0,
        158,
        PORTRAIT_W,
        "DOWN DROPS",
        theme::MUTED,
        1,
    );
    draw_centered_text(
        &mut display,
        0,
        220,
        PORTRAIT_W,
        "PRESS SW TO START",
        theme::TEXT,
        1,
    );
    flush(&mut display);
}

pub fn render(display: &mut impl DisplaySink, game: &TetrisGame) {
    let mut display = RotatedClockwise::new(display);
    clear_portrait(&mut display, theme::BG);
    render_hud(&mut display, game);
    render_well(&mut display, game);
    if game.mode() == TetrisMode::GameOver {
        render_game_over_overlay(&mut display, game);
    }
    flush(&mut display);
}

pub fn render_play_delta(
    display: &mut impl DisplaySink,
    game: &TetrisGame,
    previous_board: &[u8; TETRIS_CELLS],
    previous_active: TetrisPiece,
    previous_score: u32,
    previous_lines: u32,
    previous_next: Tetromino,
) {
    let mut display = RotatedClockwise::new(display);
    render_hud_delta(
        &mut display,
        game,
        previous_score,
        previous_lines,
        previous_next,
    );

    for row in 0..TETRIS_ROWS {
        for col in 0..TETRIS_COLS {
            let before = occupied_cell(previous_board, previous_active, row, col);
            let after = game.occupied_cell(row, col);
            if before != after {
                draw_board_position(&mut display, row, col, after);
            }
        }
    }

    flush(&mut display);
}

pub fn render_pause_menu(display: &mut impl DisplaySink, game: &TetrisGame) {
    let mut display = RotatedClockwise::new(display);
    clear_portrait(&mut display, theme::BG);
    render_hud(&mut display, game);
    render_well(&mut display, game);
    fill_rect(&mut display, 28, 104, 184, 92, theme::OVERLAY);
    draw_centered_text(&mut display, 28, 114, 184, "PAUSED", theme::TEXT, 2);
    draw_option_row(
        &mut display,
        50,
        142,
        "DO",
        "CONTINUE",
        game.pause_action() == TetrisPauseAction::Continue,
    );
    draw_option_row(
        &mut display,
        50,
        172,
        "DO",
        "EXIT",
        game.pause_action() == TetrisPauseAction::Exit,
    );
    flush(&mut display);
}

fn render_hud(display: &mut impl DisplaySink, game: &TetrisGame) {
    draw_text(display, 8, 8, "TETRIS", theme::TEXT, 2);
    draw_score(display, game.score());
    draw_lines(display, game.lines(), game.level());
    draw_next(display, game.next());
}

fn render_hud_delta(
    display: &mut impl DisplaySink,
    game: &TetrisGame,
    previous_score: u32,
    previous_lines: u32,
    previous_next: Tetromino,
) {
    if game.score() != previous_score {
        fill_rect(display, 8, 25, 68, 10, theme::BG);
        draw_score(display, game.score());
    }
    if game.lines() != previous_lines {
        fill_rect(display, 86, 8, 74, 24, theme::BG);
        draw_lines(display, game.lines(), game.level());
    }
    if game.next() != previous_next {
        fill_rect(display, 180, 39, 56, 43, theme::BG);
        draw_next(display, game.next());
    }
}

fn draw_score(display: &mut impl DisplaySink, score: u32) {
    let mut buf = TextBuffer::<24>::new();
    let _ = write!(buf, "S:{}", score);
    draw_text(display, 8, 25, buf.as_str(), theme::TEXT, 1);
}

fn draw_lines(display: &mut impl DisplaySink, lines: u32, level: u32) {
    let mut line_buf = TextBuffer::<24>::new();
    let _ = write!(line_buf, "L:{}", lines);
    draw_text(display, 86, 10, line_buf.as_str(), theme::ACCENT, 1);

    let mut level_buf = TextBuffer::<24>::new();
    let _ = write!(level_buf, "LV:{}", level);
    draw_text(display, 86, 24, level_buf.as_str(), theme::MUTED, 1);
}

fn draw_next(display: &mut impl DisplaySink, kind: Tetromino) {
    draw_text(display, 184, 10, "NEXT", theme::MUTED, 1);
    draw_next_piece(display, kind, 186, 42);
}

fn render_well(display: &mut impl DisplaySink, game: &TetrisGame) {
    fill_rect(
        display,
        WELL_X - 2,
        WELL_Y - 2,
        WELL_W + 4,
        WELL_H + 4,
        WELL_BORDER,
    );
    fill_rect(display, WELL_X, WELL_Y, WELL_W, WELL_H, WELL_BG);

    for row in 0..TETRIS_ROWS {
        for col in 0..TETRIS_COLS {
            draw_board_position(display, row, col, game.occupied_cell(row, col));
        }
    }
}

fn draw_board_position(display: &mut impl DisplaySink, row: usize, col: usize, value: u8) {
    if value == 0 {
        draw_empty_cell(display, row, col);
    } else {
        draw_cell(display, row, col, piece_color(value));
    }
}

fn draw_empty_cell(display: &mut impl DisplaySink, row: usize, col: usize) {
    let x = WELL_X + col as i16 * WELL_CELL;
    let y = WELL_Y + row as i16 * WELL_CELL;
    fill_rect(display, x, y, WELL_CELL, WELL_CELL, WELL_BG);
    fill_rect(display, x, y, 1, WELL_CELL, theme::GRID);
    fill_rect(display, x, y, WELL_CELL, 1, theme::GRID);
}

fn draw_cell(display: &mut impl DisplaySink, row: usize, col: usize, color: u16) {
    let x = WELL_X + col as i16 * WELL_CELL;
    let y = WELL_Y + row as i16 * WELL_CELL;
    fill_rect(display, x, y, WELL_CELL, WELL_CELL, theme::GRID);
    fill_rect(
        display,
        x + CELL_INSET,
        y + CELL_INSET,
        WELL_CELL - CELL_INSET * 2,
        WELL_CELL - CELL_INSET * 2,
        color,
    );
    fill_rect(display, x + 2, y + 2, WELL_CELL - 4, 1, theme::TEXT);
}

fn draw_next_piece(display: &mut impl DisplaySink, kind: Tetromino, x: i16, y: i16) {
    fill_rect(display, x - 4, y - 4, 50, 42, theme::HUD);
    for block in tetromino_cells(kind, 0) {
        let px = x + block.x as i16 * MINI_CELL;
        let py = y + block.y as i16 * MINI_CELL;
        fill_rect(
            display,
            px,
            py,
            MINI_CELL - 1,
            MINI_CELL - 1,
            piece_color(kind.cell_value()),
        );
    }
}

fn render_game_over_overlay(display: &mut impl DisplaySink, game: &TetrisGame) {
    fill_rect(
        display,
        WELL_X + 8,
        WELL_Y + 98,
        WELL_W - 16,
        70,
        theme::OVERLAY,
    );
    draw_centered_text(
        display,
        WELL_X + 8,
        WELL_Y + 112,
        WELL_W - 16,
        "GAME OVER",
        theme::BAD,
        1,
    );
    let mut buf = TextBuffer::<24>::new();
    let _ = write!(buf, "S:{}", game.score());
    draw_centered_text(
        display,
        WELL_X + 8,
        WELL_Y + 138,
        WELL_W - 16,
        buf.as_str(),
        theme::TEXT,
        1,
    );
}

fn occupied_cell(board: &[u8; TETRIS_CELLS], active: TetrisPiece, row: usize, col: usize) -> u8 {
    let x = col as i8;
    let y = row as i8;
    for block in active.cells() {
        if block.x == x && block.y == y {
            return active.kind.cell_value();
        }
    }
    board[row * TETRIS_COLS + col]
}

fn piece_color(value: u8) -> u16 {
    match value {
        1 => I_COLOR,
        2 => O_COLOR,
        3 => T_COLOR,
        4 => S_COLOR,
        5 => Z_COLOR,
        6 => J_COLOR,
        7 => L_COLOR,
        _ => WELL_BG,
    }
}

fn clear_portrait(display: &mut impl DisplaySink, color: u16) {
    fill_rect(display, 0, 0, PORTRAIT_W, PORTRAIT_H, color);
}

struct RotatedClockwise<'a, D: DisplaySink> {
    display: &'a mut D,
}

impl<'a, D: DisplaySink> RotatedClockwise<'a, D> {
    fn new(display: &'a mut D) -> Self {
        Self { display }
    }
}

impl<D: DisplaySink> DisplaySink for RotatedClockwise<'_, D> {
    fn push(&mut self, command: DrawCommand) {
        match command {
            DrawCommand::Fill { rect, color } => {
                self.display.push(DrawCommand::Fill {
                    rect: rotate_rect_clockwise(rect),
                    color,
                });
            }
            DrawCommand::Flush => self.display.push(DrawCommand::Flush),
            DrawCommand::Bitmap { .. } => {}
        }
    }
}

fn rotate_rect_clockwise(rect: Rect) -> Rect {
    Rect {
        x: PORTRAIT_H - rect.y - rect.h,
        y: rect.x,
        w: rect.h,
        h: rect.w,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{DrawCommand, RecordingDisplay};
    use crate::rng::ScriptedRng;

    #[test]
    fn render_choosing_draws_rotated_start_screen() {
        let mut display = RecordingDisplay::new();

        render_choosing(&mut display);

        assert!(matches!(
            display.commands()[0],
            DrawCommand::Fill {
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 320,
                    h: 240
                },
                color: theme::BG
            }
        ));
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_playing_draws_rotated_portrait_well() {
        let mut display = RecordingDisplay::new();
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([0, 1]);
        game.press_switch(&mut rng, 0);

        render(&mut display, &game);

        let expected = rotate_rect_clockwise(Rect {
            x: WELL_X - 2,
            y: WELL_Y - 2,
            w: WELL_W + 4,
            h: WELL_H + 4,
        });
        assert!(display.commands().iter().any(|command| matches!(
            command,
            DrawCommand::Fill {
                rect,
                ..
            } if *rect == expected
        )));
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_play_delta_skips_full_screen_clear() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([0, 1]);
        game.press_switch(&mut rng, 0);
        let previous_board = *game.board();
        let previous_active = game.active();
        let previous_score = game.score();
        let previous_lines = game.lines();
        let previous_next = game.next();
        game.move_active(crate::config::Direction::Down);

        let mut display = RecordingDisplay::new();
        render_play_delta(
            &mut display,
            &game,
            &previous_board,
            previous_active,
            previous_score,
            previous_lines,
            previous_next,
        );

        assert!(!display.commands().iter().any(|command| matches!(
            command,
            DrawCommand::Fill {
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 320,
                    h: 240
                },
                color: theme::BG
            }
        )));
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_pause_menu_draws_options() {
        let mut display = RecordingDisplay::new();
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([0, 1]);
        game.press_switch(&mut rng, 0);
        game.press_switch(&mut rng, 1);

        render_pause_menu(&mut display, &game);

        assert!(display.commands().len() > 20);
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }
}
