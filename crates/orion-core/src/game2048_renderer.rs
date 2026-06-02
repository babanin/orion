use crate::config::TFT_H_RES;
use crate::font::{draw_centered_text, draw_text, TextBuffer};
use crate::game2048::{
    Game2048, Game2048ChoosingField, Game2048GameOverAction, Game2048Mode, GridSize, PauseAction,
    MAX_CELLS, MAX_GRID_SIZE,
};
use crate::render::{clear, fill_rect, flush, DisplaySink};
use crate::theme;
use crate::ui_widgets::draw_option_row;
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

    draw_option_row(
        display,
        42,
        70,
        236,
        "SIZE",
        game.grid_size().label(),
        game.choosing_field() == Game2048ChoosingField::Size,
    );

    let mut best_buf = TextBuffer::<16>::new();
    write!(best_buf, "BEST:{}", game.best_score()).unwrap();
    draw_centered_text(
        display,
        0,
        102,
        TFT_H_RES,
        best_buf.as_str(),
        theme::TEXT,
        1,
    );

    draw_option_row(
        display,
        42,
        128,
        236,
        "",
        "EXIT",
        game.choosing_field() == Game2048ChoosingField::Exit,
    );

    if game.choosing_field() == Game2048ChoosingField::Size {
        draw_centered_text(
            display,
            0,
            170,
            TFT_H_RES,
            "LR OR KNOB SIZE",
            theme::MUTED,
            1,
        );
        draw_centered_text(
            display,
            0,
            188,
            TFT_H_RES,
            "PRESS SW TO START",
            theme::TEXT,
            1,
        );
    } else {
        draw_centered_text(
            display,
            0,
            170,
            TFT_H_RES,
            "PRESS SW TO EXIT",
            theme::TEXT,
            1,
        );
    }

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

    draw_centered_text(display, grid_x, cy - 34, grid_w, "GAME OVER", theme::BAD, 2);

    let mut score_buf = TextBuffer::<24>::new();
    write!(score_buf, "SCORE:{}", game.score()).unwrap();
    draw_centered_text(
        display,
        grid_x,
        cy - 8,
        grid_w,
        score_buf.as_str(),
        theme::TEXT,
        1,
    );

    let restart_selected = game.game_over_action() == Game2048GameOverAction::Restart;
    let restart_y = cy + 10;
    fill_rect(
        display,
        grid_x + 4,
        restart_y,
        grid_w - 8,
        18,
        if restart_selected {
            theme::OVERLAY
        } else {
            theme::HUD
        },
    );
    if restart_selected {
        fill_rect(display, grid_x + 8, restart_y + 4, 4, 10, theme::ACCENT);
    }
    draw_text(
        display,
        grid_x + 16,
        restart_y + 4,
        "RESTART",
        if restart_selected {
            theme::TEXT
        } else {
            theme::MUTED
        },
        1,
    );

    let exit_y = restart_y + 22;
    fill_rect(
        display,
        grid_x + 4,
        exit_y,
        grid_w - 8,
        18,
        if !restart_selected {
            theme::OVERLAY
        } else {
            theme::HUD
        },
    );
    if !restart_selected {
        fill_rect(display, grid_x + 8, exit_y + 4, 4, 10, theme::ACCENT);
    }
    draw_text(
        display,
        grid_x + 16,
        exit_y + 4,
        "EXIT",
        if !restart_selected {
            theme::TEXT
        } else {
            theme::MUTED
        },
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
    use crate::game2048::{Game2048ChoosingField, Game2048GameOverAction, PauseAction};
    use crate::render::RecordingDisplay;
    use crate::store::MemoryHighScoreStore;

    fn start_game() -> (Game2048, MemoryHighScoreStore, crate::rng::ScriptedRng) {
        let mut game = Game2048::default();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = crate::rng::ScriptedRng::new([0, 5, 3, 7]);
        game.press_switch(&mut scores, &mut rng);
        (game, scores, rng)
    }

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

    #[test]
    fn render_game_over_draws_overlay() {
        let mut game = Game2048::default();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = crate::rng::ScriptedRng::new([0, 5, 3, 7]);
        game.press_switch(&mut scores, &mut rng);
        game.mode = Game2048Mode::GameOver;
        game.game_over_action = Game2048GameOverAction::Restart;
        let mut display = RecordingDisplay::new();
        render(&mut display, &game);
        assert!(display.commands().len() > 10);
    }

    #[test]
    fn render_game_over_with_exit_selected() {
        let mut game = Game2048::default();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = crate::rng::ScriptedRng::new([0, 5, 3, 7]);
        game.press_switch(&mut scores, &mut rng);
        game.mode = Game2048Mode::GameOver;
        game.game_over_action = Game2048GameOverAction::Exit;
        let mut display = RecordingDisplay::new();
        render(&mut display, &game);
        assert!(display.commands().len() > 10);
    }

    #[test]
    fn render_pause_menu_with_continue_selected() {
        let mut display = RecordingDisplay::new();
        let mut game = Game2048::default();
        game.mode = Game2048Mode::Paused;
        game.pause_action = PauseAction::Continue;
        render_pause_menu(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_pause_menu_with_exit_selected() {
        let mut display = RecordingDisplay::new();
        let mut game = Game2048::default();
        game.mode = Game2048Mode::Paused;
        game.pause_action = PauseAction::Exit;
        render_pause_menu(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_choosing_with_exit_field() {
        let mut display = RecordingDisplay::new();
        let mut game = Game2048::default();
        game.choosing_field = Game2048ChoosingField::Exit;
        render_choosing(&mut display, &game);
        assert!(display.commands().len() > 4);
    }

    #[test]
    fn render_move_delta_with_score_change() {
        let (mut game, _scores, mut rng) = start_game();
        let prev_grid = *game.grid();
        let prev_score = game.score();
        let prev_best = game.best_score();
        game.slide(crate::config::Direction::Right);
        game.place_random_tile(&mut rng);

        let mut display = RecordingDisplay::new();
        render_move_delta(&mut display, &game, &prev_grid, prev_score, prev_best);
        assert!(display.commands().len() > 2);
    }

    #[test]
    fn render_move_delta_with_best_score_change() {
        let (mut game, mut scores, mut rng) = start_game();
        let prev_grid = *game.grid();
        let prev_score = game.score();
        game.slide(crate::config::Direction::Right);
        game.place_random_tile(&mut rng);
        game.update_best_score(&mut scores);

        let mut display = RecordingDisplay::new();
        render_move_delta(
            &mut display,
            &game,
            &prev_grid,
            prev_score,
            game.best_score(),
        );
        assert!(display.commands().len() > 2);
    }

    #[test]
    fn tile_color_covers_all_values() {
        assert_eq!(tile_color(0), TILE_EMPTY);
        assert_eq!(tile_color(2), TILE_2);
        assert_eq!(tile_color(4), TILE_4);
        assert_eq!(tile_color(8), TILE_8);
        assert_eq!(tile_color(16), TILE_16);
        assert_eq!(tile_color(32), TILE_32);
        assert_eq!(tile_color(64), TILE_64);
        assert_eq!(tile_color(128), TILE_128);
        assert_eq!(tile_color(256), TILE_256);
        assert_eq!(tile_color(512), TILE_512);
        assert_eq!(tile_color(1024), TILE_1024);
        assert_eq!(tile_color(2048), TILE_2048);
        assert_eq!(tile_color(4096), TILE_SUPER);
    }

    #[test]
    fn tile_text_color_dark_for_low_values() {
        assert_eq!(tile_text_color(2), DARK_TEXT);
        assert_eq!(tile_text_color(4), DARK_TEXT);
        assert_eq!(tile_text_color(8), LIGHT_TEXT);
        assert_eq!(tile_text_color(128), LIGHT_TEXT);
    }

    #[test]
    fn grid_layout_varies_by_size() {
        let (_, _, cell_small, _) = grid_layout(GridSize::Small);
        let (_, _, cell_classic, _) = grid_layout(GridSize::Classic);
        let (_, _, cell_large, _) = grid_layout(GridSize::Large);
        assert!(cell_small > cell_classic);
        assert!(cell_classic > cell_large);
    }

    #[test]
    fn choose_aa_scale_returns_appropriate_scale() {
        assert_eq!(choose_aa_scale(70, 1), 4);
        assert_eq!(choose_aa_scale(50, 2), 4);
        assert_eq!(choose_aa_scale(24, 2), 2);
        assert_eq!(choose_aa_scale(50, 3), 2);
        assert_eq!(choose_aa_scale(20, 3), 1);
        assert_eq!(choose_aa_scale(30, 0), 1);
    }
}
