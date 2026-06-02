use core::fmt::Write;

use crate::config::{
    on_board, Cell, Direction, BOARD_COLS, BOARD_ROWS, CELL_SIZE, HUD_HEIGHT, TFT_H_RES, TFT_V_RES,
};
use crate::font::{draw_centered_text, draw_text, TextBuffer};
use crate::render::{clear, fill_rect, flush, DisplaySink};
use crate::snake::{border_mode_name, speed_config, GameMode, SelectionField, SnakeGame};
use crate::theme;
use crate::ui_widgets::draw_option_row;

pub fn render(display: &mut impl DisplaySink, game: &SnakeGame) {
    if game.mode() == GameMode::Choosing {
        draw_start_screen(display, game);
        flush(display);
        return;
    }

    draw_hud(display, game);
    fill_rect(
        display,
        0,
        HUD_HEIGHT,
        TFT_H_RES,
        TFT_V_RES - HUD_HEIGHT,
        theme::BG,
    );
    for y in 0..BOARD_ROWS {
        for x in 0..BOARD_COLS {
            fill_rect(
                display,
                x * CELL_SIZE,
                HUD_HEIGHT + y * CELL_SIZE,
                1,
                CELL_SIZE,
                theme::GRID,
            );
            fill_rect(
                display,
                x * CELL_SIZE,
                HUD_HEIGHT + y * CELL_SIZE,
                CELL_SIZE,
                1,
                theme::GRID,
            );
        }
    }

    draw_food(display, game.food());
    for index in (0..game.length()).rev() {
        draw_snake_segment(display, game, index);
    }

    match game.mode() {
        GameMode::Paused => {
            fill_rect(display, 86, 100, 148, 40, theme::OVERLAY);
            draw_text(display, 104, 113, "PAUSED", theme::TEXT, 2);
        }
        GameMode::Over => {
            fill_rect(display, 58, 92, 204, 56, theme::OVERLAY);
            draw_text(display, 76, 102, "GAME OVER", theme::TEXT, 2);
            draw_text(display, 64, 128, "PRESS SW TO SET", theme::MUTED, 1);
        }
        _ => {}
    }
    flush(display);
}

pub fn render_pause_menu(display: &mut impl DisplaySink, game: &SnakeGame, selected_index: usize) {
    render(display, game);
    fill_rect(display, 70, 74, 180, 92, theme::OVERLAY);
    draw_centered_text(display, 70, 84, 180, "PAUSED", theme::TEXT, 2);
    draw_option_row(display, 92, 112, 236, "DO", "CONTINUE", selected_index == 0);
    draw_option_row(display, 92, 142, 236, "DO", "EXIT", selected_index == 1);
    flush(display);
}

pub fn render_hud(display: &mut impl DisplaySink, game: &SnakeGame) {
    draw_hud(display, game);
    flush(display);
}

pub fn render_tick_delta(
    display: &mut impl DisplaySink,
    game: &SnakeGame,
    old_tail: Cell,
    old_score: u32,
    old_best_score: u32,
    grew: bool,
) {
    if old_score != game.score() || old_best_score != game.best_score() {
        draw_hud(display, game);
    }
    if !grew {
        clear_board_cell(display, old_tail);
    }
    if game.length() > 1 {
        draw_snake_body_at(display, game, 1);
    }
    draw_snake_tail(display, game);
    draw_food(display, game.food());
    draw_snake_head(display, game.head(), game.direction());
    flush(display);
}

fn direction_between(from: Cell, to: Cell) -> Direction {
    if to.x == from.x + 1 || (from.x == BOARD_COLS - 1 && to.x == 0) {
        Direction::Right
    } else if to.x == from.x - 1 || (from.x == 0 && to.x == BOARD_COLS - 1) {
        Direction::Left
    } else if to.y == from.y + 1 || (from.y == BOARD_ROWS - 1 && to.y == 0) {
        Direction::Down
    } else {
        Direction::Up
    }
}

fn clear_board_cell(display: &mut impl DisplaySink, cell: Cell) {
    if !on_board(cell) {
        return;
    }
    let x = cell.x * CELL_SIZE;
    let y = HUD_HEIGHT + cell.y * CELL_SIZE;
    fill_rect(display, x, y, CELL_SIZE, CELL_SIZE, theme::BG);
    fill_rect(display, x, y, 1, CELL_SIZE, theme::GRID);
    fill_rect(display, x, y, CELL_SIZE, 1, theme::GRID);
}

fn draw_snake_segment(display: &mut impl DisplaySink, game: &SnakeGame, index: usize) {
    if index == 0 {
        draw_snake_head(display, game.head(), game.direction());
    } else if index == game.length() - 1 {
        draw_snake_tail(display, game);
    } else {
        draw_snake_body_at(display, game, index);
    }
}

fn draw_snake_body_at(display: &mut impl DisplaySink, game: &SnakeGame, index: usize) {
    if index == 0 || index >= game.length() - 1 {
        draw_snake_segment(display, game, index);
        return;
    }
    let cell = game.snake_at(index);
    draw_snake_body(
        display,
        cell,
        direction_between(cell, game.snake_at(index - 1)),
        direction_between(cell, game.snake_at(index + 1)),
    );
}

fn draw_snake_body(
    display: &mut impl DisplaySink,
    cell: Cell,
    first: Direction,
    second: Direction,
) {
    let x = cell.x * CELL_SIZE;
    let y = HUD_HEIGHT + cell.y * CELL_SIZE;
    clear_board_cell(display, cell);
    draw_snake_body_arm(display, x, y, first);
    draw_snake_body_arm(display, x, y, second);
    fill_rect(display, x + 3, y + 3, 10, 10, theme::SNAKE);
    fill_rect(display, x + 5, y + 5, 2, 2, theme::SNAKE_MARK);
    fill_rect(display, x + 9, y + 8, 2, 2, theme::SNAKE_MARK);
    fill_rect(display, x + 6, y + 11, 3, 1, theme::SNAKE_MARK);
}

fn draw_snake_body_arm(display: &mut impl DisplaySink, x: i16, y: i16, direction: Direction) {
    match direction {
        Direction::Up => fill_rect(display, x + 3, y + 1, 10, 7, theme::SNAKE),
        Direction::Down => fill_rect(display, x + 3, y + 8, 10, 7, theme::SNAKE),
        Direction::Left => fill_rect(display, x + 1, y + 3, 7, 10, theme::SNAKE),
        Direction::Right => fill_rect(display, x + 8, y + 3, 7, 10, theme::SNAKE),
    }
}

fn draw_snake_head(display: &mut impl DisplaySink, cell: Cell, direction: Direction) {
    let x = cell.x * CELL_SIZE;
    let y = HUD_HEIGHT + cell.y * CELL_SIZE;
    clear_board_cell(display, cell);
    let draw_direction = direction.opposite();
    match draw_direction {
        Direction::Up => {
            fill_rect(display, x + 3, y + 5, 10, 10, theme::HEAD);
            fill_rect(display, x + 1, y + 1, 14, 7, theme::HEAD);
            draw_snake_head_marks(display, x, y, draw_direction);
            fill_rect(display, x + 5, y + 3, 2, 2, theme::EYE);
            fill_rect(display, x + 10, y + 3, 2, 2, theme::EYE);
        }
        Direction::Down => {
            fill_rect(display, x + 3, y + 1, 10, 10, theme::HEAD);
            fill_rect(display, x + 1, y + 8, 14, 7, theme::HEAD);
            draw_snake_head_marks(display, x, y, draw_direction);
            fill_rect(display, x + 5, y + 11, 2, 2, theme::EYE);
            fill_rect(display, x + 10, y + 11, 2, 2, theme::EYE);
        }
        Direction::Left => {
            fill_rect(display, x + 5, y + 3, 10, 10, theme::HEAD);
            fill_rect(display, x + 1, y + 1, 7, 14, theme::HEAD);
            draw_snake_head_marks(display, x, y, draw_direction);
            fill_rect(display, x + 3, y + 5, 2, 2, theme::EYE);
            fill_rect(display, x + 3, y + 10, 2, 2, theme::EYE);
        }
        Direction::Right => {
            fill_rect(display, x + 1, y + 3, 10, 10, theme::HEAD);
            fill_rect(display, x + 8, y + 1, 7, 14, theme::HEAD);
            draw_snake_head_marks(display, x, y, draw_direction);
            fill_rect(display, x + 11, y + 5, 2, 2, theme::EYE);
            fill_rect(display, x + 11, y + 10, 2, 2, theme::EYE);
        }
    }
}

fn draw_snake_head_marks(display: &mut impl DisplaySink, x: i16, y: i16, direction: Direction) {
    match direction {
        Direction::Up => {
            fill_rect(display, x + 6, y + 8, 4, 1, theme::HEAD_MARK);
            fill_rect(display, x + 4, y + 11, 3, 1, theme::HEAD_MARK);
            fill_rect(display, x + 9, y + 11, 3, 1, theme::HEAD_MARK);
        }
        Direction::Down => {
            fill_rect(display, x + 6, y + 7, 4, 1, theme::HEAD_MARK);
            fill_rect(display, x + 4, y + 4, 3, 1, theme::HEAD_MARK);
            fill_rect(display, x + 9, y + 4, 3, 1, theme::HEAD_MARK);
        }
        Direction::Left => {
            fill_rect(display, x + 8, y + 6, 1, 4, theme::HEAD_MARK);
            fill_rect(display, x + 11, y + 4, 1, 3, theme::HEAD_MARK);
            fill_rect(display, x + 11, y + 9, 1, 3, theme::HEAD_MARK);
        }
        Direction::Right => {
            fill_rect(display, x + 7, y + 6, 1, 4, theme::HEAD_MARK);
            fill_rect(display, x + 4, y + 4, 1, 3, theme::HEAD_MARK);
            fill_rect(display, x + 4, y + 9, 1, 3, theme::HEAD_MARK);
        }
    }
}

fn draw_snake_tail(display: &mut impl DisplaySink, game: &SnakeGame) {
    if game.length() < 2 {
        draw_snake_head(display, game.head(), game.direction());
        return;
    }
    draw_snake_tail_cell(
        display,
        game.tail(),
        direction_between(game.tail(), game.snake_at(game.length() - 2)),
    );
}

fn draw_snake_tail_cell(display: &mut impl DisplaySink, cell: Cell, body_direction: Direction) {
    let x = cell.x * CELL_SIZE;
    let y = HUD_HEIGHT + cell.y * CELL_SIZE;
    clear_board_cell(display, cell);
    match body_direction.opposite() {
        Direction::Up => {
            fill_rect(display, x + 4, y + 8, 8, 7, theme::SNAKE);
            fill_rect(display, x + 5, y + 5, 6, 3, theme::SNAKE);
            fill_rect(display, x + 6, y + 3, 4, 2, theme::SNAKE);
        }
        Direction::Down => {
            fill_rect(display, x + 4, y + 1, 8, 7, theme::SNAKE);
            fill_rect(display, x + 5, y + 8, 6, 3, theme::SNAKE);
            fill_rect(display, x + 6, y + 11, 4, 2, theme::SNAKE);
        }
        Direction::Left => {
            fill_rect(display, x + 8, y + 4, 7, 8, theme::SNAKE);
            fill_rect(display, x + 5, y + 5, 3, 6, theme::SNAKE);
            fill_rect(display, x + 3, y + 6, 2, 4, theme::SNAKE);
        }
        Direction::Right => {
            fill_rect(display, x + 1, y + 4, 7, 8, theme::SNAKE);
            fill_rect(display, x + 8, y + 5, 3, 6, theme::SNAKE);
            fill_rect(display, x + 11, y + 6, 2, 4, theme::SNAKE);
        }
    }
}

fn draw_food(display: &mut impl DisplaySink, cell: Cell) {
    if !on_board(cell) {
        return;
    }
    let x = cell.x * CELL_SIZE;
    let y = HUD_HEIGHT + cell.y * CELL_SIZE;
    clear_board_cell(display, cell);
    fill_rect(display, x + 5, y + 1, 2, 4, theme::STEM);
    fill_rect(display, x + 8, y + 2, 4, 2, theme::LEAF);
    fill_rect(display, x + 9, y + 3, 3, 2, theme::LEAF);
    fill_rect(display, x + 4, y + 5, 8, 1, theme::APPLE);
    fill_rect(display, x + 3, y + 6, 10, 2, theme::APPLE);
    fill_rect(display, x + 2, y + 8, 12, 4, theme::APPLE);
    fill_rect(display, x + 3, y + 12, 10, 2, theme::APPLE);
    fill_rect(display, x + 5, y + 14, 6, 1, theme::APPLE);
    fill_rect(display, x + 11, y + 8, 2, 5, theme::APPLE_DARK);
    fill_rect(display, x + 9, y + 13, 3, 1, theme::APPLE_DARK);
    fill_rect(display, x + 4, y + 7, 2, 2, theme::APPLE_HIGHLIGHT);
}

fn draw_start_screen(display: &mut impl DisplaySink, game: &SnakeGame) {
    clear(display, theme::BG);
    draw_text(display, 116, 20, "SNAKE", theme::TEXT, 3);
    let mut best_text = TextBuffer::<40>::new();
    let _ = write!(best_text, "BEST:{}", game.best_score());
    draw_text(display, 124, 54, best_text.as_str(), theme::MUTED, 1);
    draw_option_row(
        display,
        42,
        78,
        236,
        "SPEED",
        speed_config(game.speed_tier()).name,
        game.selected_field() == SelectionField::Speed,
    );
    draw_option_row(
        display,
        42,
        110,
        236,
        "MODE",
        border_mode_name(game.border_mode()),
        game.selected_field() == SelectionField::Border,
    );
    draw_option_row(
        display,
        42,
        142,
        236,
        "",
        "EXIT",
        game.selected_field() == SelectionField::Exit,
    );
    draw_text(display, 70, 176, "UD SELECT", theme::MUTED, 1);
    draw_text(display, 70, 192, "LR OR KNOB CHANGES", theme::MUTED, 1);
    draw_text(display, 70, 210, "PRESS SW TO START", theme::TEXT, 1);
    draw_text(display, 70, 226, "PRESS SW EXIT MENU", theme::MUTED, 1);
}

fn draw_hud(display: &mut impl DisplaySink, game: &SnakeGame) {
    let mut score_text = TextBuffer::<24>::new();
    let mut best_text = TextBuffer::<24>::new();
    let _ = write!(score_text, "S:{}", game.score());
    let _ = write!(best_text, "B:{}", game.best_score());
    fill_rect(display, 0, 0, TFT_H_RES, HUD_HEIGHT, theme::HUD);
    draw_text(display, 4, 4, score_text.as_str(), theme::TEXT, 1);
    draw_text(display, 70, 4, best_text.as_str(), theme::MUTED, 1);
    draw_text(
        display,
        142,
        4,
        speed_config(game.speed_tier()).name,
        theme::TEXT,
        1,
    );
    draw_text(
        display,
        226,
        4,
        border_mode_name(game.border_mode()),
        theme::MUTED,
        1,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::ScriptedRng;
    use crate::store::MemoryHighScoreStore;

    fn start_playing() -> (SnakeGame, MemoryHighScoreStore, ScriptedRng) {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 12, 7]);
        let mut game = SnakeGame::default();
        game.reset(&scores, &mut rng, 0);
        (game, scores, rng)
    }

    #[test]
    fn start_screen_records_flush_and_title_pixels() {
        let scores = MemoryHighScoreStore::new();
        let mut game = SnakeGame::default();
        game.enter_choosing(&scores);
        let mut display = crate::render::RecordingDisplay::new();
        render(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
        assert!(display.commands().len() > 10);
    }

    #[test]
    fn playing_render_draws_bitmap_free_pixel_art() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 12, 7]);
        let mut game = SnakeGame::default();
        game.reset(&scores, &mut rng, 0);
        let mut display = crate::render::RecordingDisplay::new();
        render(&mut display, &game);
        assert!(display.commands().iter().any(|command| matches!(
            command,
            crate::render::DrawCommand::Fill {
                color: theme::APPLE,
                ..
            }
        )));
    }

    #[test]
    fn render_game_over_state() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 12, 7]);
        let mut game = SnakeGame::default();
        game.reset(&scores, &mut rng, 0);
        game.set_mode(GameMode::Over);
        let mut display = crate::render::RecordingDisplay::new();
        render(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_paused_state_classic() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 12, 7]);
        let mut game = SnakeGame::default();
        game.reset(&scores, &mut rng, 0);
        game.set_mode(GameMode::Paused);
        let mut display = crate::render::RecordingDisplay::new();
        render(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_pause_menu_draws_options() {
        let (game, _scores, _rng) = start_playing();
        let mut display = crate::render::RecordingDisplay::new();
        render_pause_menu(&mut display, &game, 0);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_pause_menu_with_exit_selected() {
        let (game, _scores, _rng) = start_playing();
        let mut display = crate::render::RecordingDisplay::new();
        render_pause_menu(&mut display, &game, 1);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_tick_delta_draws_head_and_clears_tail() {
        let (game, _scores, _rng) = start_playing();
        let old_tail = game.tail();
        let old_score = game.score();
        let old_best_score = game.best_score();
        let mut display = crate::render::RecordingDisplay::new();
        render_tick_delta(
            &mut display,
            &game,
            old_tail,
            old_score,
            old_best_score,
            false,
        );
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_tick_delta_with_score_change_redraws_hud() {
        let (game, _scores, _rng) = start_playing();
        let old_tail = game.tail();
        let mut display = crate::render::RecordingDisplay::new();
        render_tick_delta(&mut display, &game, old_tail, 0, 0, false);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_tick_delta_with_growth_skips_tail_clear() {
        let (game, _scores, _rng) = start_playing();
        let old_tail = game.tail();
        let old_score = game.score();
        let old_best = game.best_score();
        let mut display = crate::render::RecordingDisplay::new();
        render_tick_delta(&mut display, &game, old_tail, old_score, old_best, true);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_hud_draws_score_and_best() {
        let (game, _scores, _rng) = start_playing();
        let mut display = crate::render::RecordingDisplay::new();
        render_hud(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn direction_between_all_directions() {
        assert_eq!(
            direction_between(Cell::new(3, 5), Cell::new(4, 5)),
            Direction::Right
        );
        assert_eq!(
            direction_between(Cell::new(3, 5), Cell::new(2, 5)),
            Direction::Left
        );
        assert_eq!(
            direction_between(Cell::new(3, 5), Cell::new(3, 6)),
            Direction::Down
        );
        assert_eq!(
            direction_between(Cell::new(3, 5), Cell::new(3, 4)),
            Direction::Up
        );
    }

    #[test]
    fn direction_between_wraps_horizontally() {
        assert_eq!(
            direction_between(Cell::new(BOARD_COLS - 1, 5), Cell::new(0, 5)),
            Direction::Right
        );
        assert_eq!(
            direction_between(Cell::new(0, 5), Cell::new(BOARD_COLS - 1, 5)),
            Direction::Left
        );
    }

    #[test]
    fn direction_between_wraps_vertically() {
        assert_eq!(
            direction_between(Cell::new(3, BOARD_ROWS - 1), Cell::new(3, 0)),
            Direction::Down
        );
        assert_eq!(
            direction_between(Cell::new(3, 0), Cell::new(3, BOARD_ROWS - 1)),
            Direction::Up
        );
    }

    #[test]
    fn clear_board_cell_skips_off_board() {
        let mut display = crate::render::RecordingDisplay::new();
        clear_board_cell(&mut display, Cell::new(-1, -1));
        assert!(display.commands().is_empty());
    }

    #[test]
    fn draw_food_skips_off_board() {
        let mut display = crate::render::RecordingDisplay::new();
        draw_food(&mut display, Cell::new(-1, -1));
        assert!(display.commands().is_empty());
    }
}
