use crate::config::TFT_H_RES;
use crate::font::{draw_centered_text, draw_text, TextBuffer};
use crate::game2048::{Game2048, Game2048Mode, GridSize, PauseAction, MAX_CELLS, MAX_GRID_SIZE};
use crate::render::{clear, fill_rect, flush, DisplaySink};
use crate::theme;
use core::fmt::Write;

const GRID_MARGIN: i16 = 4;
const SCORE_AREA_HEIGHT: i16 = 20;
const SCORE_TEXT_X: i16 = 4;
const SCORE_TEXT_Y: i16 = 4;
const BEST_TEXT_X: i16 = 180;
const BEST_TEXT_Y: i16 = 4;
const SCORE_LABEL_W: i16 = 166;
const BEST_LABEL_W: i16 = 140;

const TILE_EMPTY: u16 = theme::rgb565(205, 193, 180);
const TILE_2: u16 = theme::rgb565(238, 228, 218);
const TILE_4: u16 = theme::rgb565(237, 224, 200);
const TILE_8: u16 = theme::rgb565(242, 177, 121);
const TILE_16: u16 = theme::rgb565(245, 149, 99);
const TILE_32: u16 = theme::rgb565(246, 124, 95);
const TILE_64: u16 = theme::rgb565(246, 94, 59);
const TILE_128: u16 = theme::rgb565(237, 207, 114);
const TILE_256: u16 = theme::rgb565(237, 204, 97);
const TILE_512: u16 = theme::rgb565(237, 200, 80);
const TILE_1024: u16 = theme::rgb565(237, 197, 63);
const TILE_2048: u16 = theme::rgb565(237, 194, 46);
const TILE_SUPER: u16 = theme::rgb565(60, 58, 50);
const GRID_BG: u16 = theme::rgb565(187, 173, 160);
const DARK_TEXT: u16 = theme::rgb565(119, 110, 101);
const LIGHT_TEXT: u16 = theme::rgb565(249, 246, 242);

fn tile_color(value: u16) -> u16 {
    match value {
        0 => TILE_EMPTY,
        2 => TILE_2,
        4 => TILE_4,
        8 => TILE_8,
        16 => TILE_16,
        32 => TILE_32,
        64 => TILE_64,
        128 => TILE_128,
        256 => TILE_256,
        512 => TILE_512,
        1024 => TILE_1024,
        2048 => TILE_2048,
        _ => TILE_SUPER,
    }
}

fn tile_text_color(value: u16) -> u16 {
    if value <= 4 {
        DARK_TEXT
    } else {
        LIGHT_TEXT
    }
}

fn grid_layout(grid_size: GridSize) -> (i16, i16, i16, i16) {
    let n = grid_size.size() as i16;
    let avail_w = TFT_H_RES;
    let avail_h = 240 - SCORE_AREA_HEIGHT;

    let cell_w = (avail_w - (n + 1) * GRID_MARGIN) / n;
    let cell_h = (avail_h - (n + 1) * GRID_MARGIN) / n;
    let cell = cell_w.min(cell_h);

    let grid_w = n * cell + (n + 1) * GRID_MARGIN;
    let grid_h = n * cell + (n + 1) * GRID_MARGIN;

    let grid_x = (avail_w - grid_w) / 2;
    let grid_y = SCORE_AREA_HEIGHT + (avail_h - grid_h) / 2;

    (grid_x, grid_y, cell, grid_w)
}

fn choose_aa_scale(cell: i16, digit_count: usize) -> i16 {
    if digit_count == 0 {
        return 1;
    }
    let len = digit_count as i16;
    let max_w = cell - 2;
    for scale in [4, 3, 2] {
        let text_w = scale * (6 * len - 1);
        if text_w <= max_w {
            return scale;
        }
    }
    1
}

fn draw_single_tile(display: &mut impl DisplaySink, game: &Game2048, row: usize, col: usize) {
    let grid_size = game.grid_size();
    let (grid_x, grid_y, cell, _) = grid_layout(grid_size);

    let x = grid_x + GRID_MARGIN + col as i16 * (cell + GRID_MARGIN);
    let y = grid_y + GRID_MARGIN + row as i16 * (cell + GRID_MARGIN);
    let value = game.cell(row, col);

    fill_rect(display, x, y, cell, cell, tile_color(value));

    if value != 0 {
        let text_color = tile_text_color(value);
        let bg_color = tile_color(value);
        let mut buf = TextBuffer::<8>::new();
        write!(buf, "{}", value).unwrap();
        let scale = choose_aa_scale(cell, buf.as_str().chars().count());
        if scale >= 2 {
            crate::font::draw_centered_text_aa(
                display,
                x,
                y,
                cell,
                cell,
                buf.as_str(),
                text_color,
                bg_color,
                scale,
            );
        } else {
            draw_centered_text(display, x, y + 1, cell, buf.as_str(), text_color, 1);
        }
    }
}

pub fn render(display: &mut impl DisplaySink, game: &Game2048) {
    clear(display, theme::BG);
    render_score_area(display, game);
    render_grid_background(display, game);
    render_all_tiles(display, game);
    if game.mode() == Game2048Mode::GameOver {
        render_game_over_overlay(display, game);
    }
    flush(display);
}

pub fn render_move_delta(
    display: &mut impl DisplaySink,
    game: &Game2048,
    prev_grid: &[u16; MAX_CELLS],
    prev_score: u32,
    prev_best_score: u32,
) {
    render_score_delta(display, game, prev_score, prev_best_score);

    let n = game.grid_size().size();
    for row in 0..n {
        for col in 0..n {
            let idx = row * MAX_GRID_SIZE + col;
            if game.grid()[idx] != prev_grid[idx] {
                draw_single_tile(display, game, row, col);
            }
        }
    }

    flush(display);
}

pub fn render_choosing(display: &mut impl DisplaySink, game: &Game2048) {
    clear(display, theme::BG);

    draw_centered_text(display, 0, 20, TFT_H_RES, "2048", theme::TEXT, 3);

    let grid_size = game.grid_size();
    let label = grid_size.label();
    draw_centered_text(display, 0, 70, TFT_H_RES, label, theme::ACCENT, 2);

    let mut best_buf = TextBuffer::<16>::new();
    write!(best_buf, "BEST: {}", game.best_score()).unwrap();
    draw_centered_text(
        display,
        0,
        110,
        TFT_H_RES,
        best_buf.as_str(),
        theme::TEXT,
        1,
    );

    draw_centered_text(
        display,
        0,
        160,
        TFT_H_RES,
        "LR OR KNOB SIZE",
        theme::MUTED,
        1,
    );
    draw_centered_text(
        display,
        0,
        178,
        TFT_H_RES,
        "PRESS SW TO START",
        theme::TEXT,
        1,
    );

    flush(display);
}

fn render_score_area(display: &mut impl DisplaySink, game: &Game2048) {
    fill_rect(display, 0, 0, TFT_H_RES, SCORE_AREA_HEIGHT, theme::HUD);

    let mut score_buf = TextBuffer::<20>::new();
    write!(score_buf, "SCORE:{}", game.score()).unwrap();
    draw_text(
        display,
        SCORE_TEXT_X,
        SCORE_TEXT_Y,
        score_buf.as_str(),
        theme::TEXT,
        1,
    );

    let mut best_buf = TextBuffer::<20>::new();
    write!(best_buf, "BEST:{}", game.best_score()).unwrap();
    draw_text(
        display,
        BEST_TEXT_X,
        BEST_TEXT_Y,
        best_buf.as_str(),
        theme::ACCENT,
        1,
    );
}

fn render_score_delta(
    display: &mut impl DisplaySink,
    game: &Game2048,
    prev_score: u32,
    prev_best_score: u32,
) {
    let cur_score = game.score();
    let cur_best = game.best_score();

    if cur_score != prev_score {
        fill_rect(
            display,
            SCORE_TEXT_X,
            0,
            SCORE_LABEL_W,
            SCORE_AREA_HEIGHT,
            theme::HUD,
        );
        let mut score_buf = TextBuffer::<20>::new();
        write!(score_buf, "SCORE:{}", cur_score).unwrap();
        draw_text(
            display,
            SCORE_TEXT_X,
            SCORE_TEXT_Y,
            score_buf.as_str(),
            theme::TEXT,
            1,
        );
    }

    if cur_best != prev_best_score {
        fill_rect(
            display,
            BEST_TEXT_X,
            0,
            BEST_LABEL_W,
            SCORE_AREA_HEIGHT,
            theme::HUD,
        );
        let mut best_buf = TextBuffer::<20>::new();
        write!(best_buf, "BEST:{}", cur_best).unwrap();
        draw_text(
            display,
            BEST_TEXT_X,
            BEST_TEXT_Y,
            best_buf.as_str(),
            theme::ACCENT,
            1,
        );
    }
}

fn render_grid_background(display: &mut impl DisplaySink, game: &Game2048) {
    let grid_size = game.grid_size();
    let n = grid_size.size();
    let (grid_x, grid_y, cell, _) = grid_layout(grid_size);

    let grid_px_w = n as i16 * cell + (n as i16 + 1) * GRID_MARGIN;
    let grid_px_h = n as i16 * cell + (n as i16 + 1) * GRID_MARGIN;
    fill_rect(display, grid_x, grid_y, grid_px_w, grid_px_h, GRID_BG);

    for row in 0..n {
        for col in 0..n {
            let x = grid_x + GRID_MARGIN + col as i16 * (cell + GRID_MARGIN);
            let y = grid_y + GRID_MARGIN + row as i16 * (cell + GRID_MARGIN);
            fill_rect(display, x, y, cell, cell, TILE_EMPTY);
        }
    }
}

fn render_all_tiles(display: &mut impl DisplaySink, game: &Game2048) {
    let n = game.grid_size().size();
    for row in 0..n {
        for col in 0..n {
            if game.cell(row, col) != 0 {
                draw_single_tile(display, game, row, col);
            }
        }
    }
}

fn render_game_over_overlay(display: &mut impl DisplaySink, game: &Game2048) {
    let grid_size = game.grid_size();
    let n = grid_size.size() as i16;
    let (grid_x, grid_y, cell, grid_w) = grid_layout(grid_size);
    let grid_h = n * cell + (n + 1) * GRID_MARGIN;

    fill_rect(display, grid_x, grid_y, grid_w, grid_h, theme::OVERLAY);

    let cy = grid_y + grid_h / 2;

    draw_centered_text(display, grid_x, cy - 24, grid_w, "GAME OVER", theme::BAD, 2);

    let mut score_buf = TextBuffer::<24>::new();
    write!(score_buf, "SCORE:{}", game.score()).unwrap();
    draw_centered_text(
        display,
        grid_x,
        cy,
        grid_w,
        score_buf.as_str(),
        theme::TEXT,
        1,
    );

    draw_centered_text(
        display,
        grid_x,
        cy + 20,
        grid_w,
        "PRESS SW",
        theme::MUTED,
        1,
    );
}

pub fn render_pause_menu(display: &mut impl DisplaySink, game: &Game2048) {
    clear(display, theme::BG);
    render_score_area(display, game);

    let options_y = 80;
    let option_h = 30;

    let continue_selected = game.pause_action() != PauseAction::Exit;
    fill_rect(
        display,
        42,
        options_y,
        236,
        option_h,
        if continue_selected {
            theme::OVERLAY
        } else {
            theme::HUD
        },
    );
    if continue_selected {
        fill_rect(display, 50, options_y + 7, 6, 16, theme::ACCENT);
    }
    draw_text(
        display,
        66,
        options_y + 9,
        "CONTINUE",
        if continue_selected {
            theme::TEXT
        } else {
            theme::MUTED
        },
        1,
    );

    let exit_y = options_y + option_h + 4;
    fill_rect(
        display,
        42,
        exit_y,
        236,
        option_h,
        if !continue_selected {
            theme::OVERLAY
        } else {
            theme::HUD
        },
    );
    if !continue_selected {
        fill_rect(display, 50, exit_y + 7, 6, 16, theme::ACCENT);
    }
    draw_text(
        display,
        66,
        exit_y + 9,
        "EXIT",
        if !continue_selected {
            theme::TEXT
        } else {
            theme::MUTED
        },
        1,
    );

    flush(display);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::RecordingDisplay;
    use crate::store::MemoryHighScoreStore;

    #[test]
    fn render_choosing_draws_title() {
        let mut display = RecordingDisplay::new();
        let game = Game2048::default();
        render_choosing(&mut display, &game);
        assert!(display.commands().len() > 4);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_playing_draws_grid() {
        let mut display = RecordingDisplay::new();
        let mut game = Game2048::default();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = crate::rng::ScriptedRng::new([0, 5, 3, 7]);
        game.press_switch(&mut scores, &mut rng);
        render(&mut display, &game);
        assert!(display.commands().len() > 10);
    }

    #[test]
    fn render_move_delta_only_redraws_changed_cells() {
        let mut game = Game2048::default();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = crate::rng::ScriptedRng::new([0, 5, 3, 7]);
        game.press_switch(&mut scores, &mut rng);

        let prev_grid = *game.grid();
        let prev_score = game.score();
        let prev_best_score = game.best_score();

        let moved = game.slide(crate::config::Direction::Right);
        assert!(moved);
        game.place_random_tile(&mut rng);

        let mut display = RecordingDisplay::new();
        render_move_delta(&mut display, &game, &prev_grid, prev_score, prev_best_score);

        assert!(display.commands().len() > 2);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));

        let full_display_count = {
            let mut d = RecordingDisplay::new();
            render(&mut d, &game);
            d.commands().len()
        };
        assert!(display.commands().len() < full_display_count);
    }
}
