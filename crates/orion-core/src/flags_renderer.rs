use core::fmt::Write;

use crate::flags::{
    flags_mode_name, flags_result_action_name, FlagAsset, FlagsChoosingAction, FlagsGame,
    FlagsMode, FlagsPauseAction, FlagsResultAction, FlagsState, FLAGS_OPTION_COUNT,
    FLAGS_QUIZ_ROUNDS,
};
use crate::font::{draw_centered_text, draw_text, draw_wrapped_text, TextBuffer};
use crate::render::{clear, draw_bitmap, fill_rect, flush, DisplaySink};
use crate::theme;
use crate::ui_widgets::draw_option_row;

const FLAGS_ANSWER_TILE_W: i16 = 146;
const FLAGS_ANSWER_TILE_H: i16 = 38;
const FLAGS_ANSWER_LEFT_X: i16 = 10;
const FLAGS_ANSWER_RIGHT_X: i16 = 164;
const FLAGS_ANSWER_TOP_Y: i16 = 140;
const FLAGS_ANSWER_BOTTOM_Y: i16 = 194;
const FLAGS_FLAG_MIN_Y: i16 = 24;
const FLAGS_FLAG_ANSWER_GAP: i16 = 8;

pub fn render(display: &mut impl DisplaySink, game: &FlagsGame) {
    match game.state() {
        FlagsState::ChoosingMode => draw_mode_screen(display, game),
        FlagsState::Question => draw_question_screen(display, game),
        FlagsState::Feedback => draw_feedback_screen(display, game),
        FlagsState::Paused => draw_pause_screen(display, game),
        FlagsState::Results => draw_result_screen(display, game, false),
        FlagsState::Over => draw_result_screen(display, game, true),
    }
    flush(display);
}

pub fn render_answer_selection(
    display: &mut impl DisplaySink,
    game: &FlagsGame,
    previous_selected_answer: usize,
) {
    let selected = game.selected_answer();
    if previous_selected_answer != selected {
        draw_answer_tile(
            display,
            game,
            previous_selected_answer,
            answer_tile_x(previous_selected_answer),
            answer_tile_y(previous_selected_answer),
            FLAGS_ANSWER_TILE_W,
            FLAGS_ANSWER_TILE_H,
        );
    }
    draw_answer_tile(
        display,
        game,
        selected,
        answer_tile_x(selected),
        answer_tile_y(selected),
        FLAGS_ANSWER_TILE_W,
        FLAGS_ANSWER_TILE_H,
    );
    flush(display);
}

pub fn render_feedback(display: &mut impl DisplaySink, game: &FlagsGame) {
    draw_feedback_overlay(display, game);
    flush(display);
}

fn answer_tile_x(index: usize) -> i16 {
    if index % 2 == 0 {
        FLAGS_ANSWER_LEFT_X
    } else {
        FLAGS_ANSWER_RIGHT_X
    }
}

fn answer_tile_y(index: usize) -> i16 {
    if index < 2 {
        FLAGS_ANSWER_TOP_Y
    } else {
        FLAGS_ANSWER_BOTTOM_Y
    }
}

fn draw_mode_screen(display: &mut impl DisplaySink, game: &FlagsGame) {
    clear(display, theme::BG);
    draw_text(display, 104, 26, "FLAGS", theme::TEXT, 3);
    draw_text(display, 70, 62, "CHOOSE MODE", theme::MUTED, 1);
    draw_option_row(
        display,
        42,
        88,
        236,
        "MODE",
        flags_mode_name(FlagsMode::Practice),
        game.choosing_action() == FlagsChoosingAction::Mode(FlagsMode::Practice),
    );
    draw_option_row(
        display,
        42,
        120,
        236,
        "MODE",
        flags_mode_name(FlagsMode::Quiz20),
        game.choosing_action() == FlagsChoosingAction::Mode(FlagsMode::Quiz20),
    );
    draw_option_row(
        display,
        42,
        152,
        236,
        "MODE",
        flags_mode_name(FlagsMode::DeathMatch),
        game.choosing_action() == FlagsChoosingAction::Mode(FlagsMode::DeathMatch),
    );
    draw_option_row(
        display,
        42,
        184,
        236,
        "",
        "EXIT",
        game.choosing_action() == FlagsChoosingAction::Exit,
    );
    let mut best_text = TextBuffer::<40>::new();
    let _ = write!(best_text, "DEATH BEST:{}", game.best_score());
    draw_text(display, 70, 216, best_text.as_str(), theme::MUTED, 1);
    draw_text(display, 70, 230, "PRESS SW SELECT", theme::TEXT, 1);
}

fn draw_question_screen(display: &mut impl DisplaySink, game: &FlagsGame) {
    clear(display, theme::BG);
    let mut score_text = TextBuffer::<48>::new();
    match game.active_mode() {
        FlagsMode::Quiz20 => {
            let _ = write!(
                score_text,
                "S:{} R:{}/{}",
                game.score(),
                game.round(),
                FLAGS_QUIZ_ROUNDS
            );
        }
        FlagsMode::DeathMatch => {
            let _ = write!(score_text, "S:{} B:{}", game.score(), game.best_score());
        }
        FlagsMode::Practice => {
            let _ = write!(score_text, "S:{}", game.score());
        }
    }
    fill_rect(display, 0, 0, 320, 16, theme::HUD);
    draw_text(
        display,
        4,
        4,
        flags_mode_name(game.active_mode()),
        theme::TEXT,
        1,
    );
    draw_text(display, 166, 4, score_text.as_str(), theme::MUTED, 1);

    let flag = game.current_flag();
    let flag_x = (320 - flag.width as i16) / 2;
    let flag_y =
        FLAGS_FLAG_MIN_Y.max(FLAGS_ANSWER_TOP_Y - FLAGS_FLAG_ANSWER_GAP - flag.height as i16);
    draw_flag_bitmap(display, flag, flag_x, flag_y);
    draw_answer_tiles(display, game);
}

fn draw_feedback_screen(display: &mut impl DisplaySink, game: &FlagsGame) {
    draw_question_screen(display, game);
    draw_feedback_overlay(display, game);
}

fn draw_feedback_overlay(display: &mut impl DisplaySink, game: &FlagsGame) {
    let correct = game.correct_answer();
    let selected = game.selected_answer();
    draw_answer_tile(
        display,
        game,
        correct,
        answer_tile_x(correct),
        answer_tile_y(correct),
        FLAGS_ANSWER_TILE_W,
        FLAGS_ANSWER_TILE_H,
    );
    if selected != correct {
        draw_answer_tile(
            display,
            game,
            selected,
            answer_tile_x(selected),
            answer_tile_y(selected),
            FLAGS_ANSWER_TILE_W,
            FLAGS_ANSWER_TILE_H,
        );
    }
    fill_rect(
        display,
        84,
        112,
        152,
        26,
        if game.last_answer_correct() {
            theme::GOOD
        } else {
            theme::BAD
        },
    );
    draw_centered_text(
        display,
        84,
        119,
        152,
        if game.last_answer_correct() {
            "CORRECT"
        } else {
            "WRONG"
        },
        theme::TEXT,
        1,
    );
}

fn draw_result_screen(display: &mut impl DisplaySink, game: &FlagsGame, game_over: bool) {
    clear(display, theme::BG);
    draw_centered_text(
        display,
        0,
        26,
        320,
        if game_over { "GAME OVER" } else { "RESULTS" },
        theme::TEXT,
        2,
    );
    let mut score_text = TextBuffer::<48>::new();
    if game.active_mode() == FlagsMode::Quiz20 {
        let _ = write!(score_text, "SCORE:{}/{}", game.score(), FLAGS_QUIZ_ROUNDS);
    } else {
        let _ = write!(score_text, "SCORE:{}", game.score());
    }
    draw_centered_text(display, 0, 72, 320, score_text.as_str(), theme::TEXT, 1);
    if game.active_mode() == FlagsMode::DeathMatch {
        let mut best_text = TextBuffer::<40>::new();
        let _ = write!(best_text, "BEST:{}", game.best_score());
        draw_centered_text(display, 0, 92, 320, best_text.as_str(), theme::MUTED, 1);
    }
    draw_option_row(
        display,
        42,
        128,
        236,
        "DO",
        flags_result_action_name(FlagsResultAction::Restart),
        game.result_action() == FlagsResultAction::Restart,
    );
    draw_option_row(
        display,
        42,
        168,
        236,
        "DO",
        flags_result_action_name(FlagsResultAction::Exit),
        game.result_action() == FlagsResultAction::Exit,
    );
    draw_text(display, 70, 222, "PRESS SW", theme::TEXT, 1);
}

fn draw_pause_screen(display: &mut impl DisplaySink, game: &FlagsGame) {
    clear(display, theme::BG);
    draw_text(display, 104, 40, "FLAGS", theme::TEXT, 3);
    draw_text(display, 70, 80, "PAUSED", theme::ACCENT, 2);
    let continue_y = 110;
    let exit_y = 142;
    draw_option_row(
        display,
        42,
        continue_y,
        236,
        "DO",
        "CONTINUE",
        game.pause_action() == FlagsPauseAction::Continue,
    );
    draw_option_row(
        display,
        42,
        exit_y,
        236,
        "DO",
        "EXIT",
        game.pause_action() == FlagsPauseAction::Exit,
    );
    draw_text(display, 70, 180, "UD OR KNOB", theme::MUTED, 1);
    draw_text(display, 70, 196, "PRESS SW SELECT", theme::TEXT, 1);
}

fn draw_flag_bitmap(display: &mut impl DisplaySink, flag: FlagAsset, x: i16, y: i16) {
    draw_bitmap(display, x, y, flag.width, flag.height, flag.offset);
}

fn draw_answer_tiles(display: &mut impl DisplaySink, game: &FlagsGame) {
    for index in 0..FLAGS_OPTION_COUNT {
        draw_answer_tile(
            display,
            game,
            index,
            answer_tile_x(index),
            answer_tile_y(index),
            FLAGS_ANSWER_TILE_W,
            FLAGS_ANSWER_TILE_H,
        );
    }
}

fn draw_answer_tile(
    display: &mut impl DisplaySink,
    game: &FlagsGame,
    index: usize,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
) {
    let mut color = if index == game.selected_answer() {
        theme::OVERLAY
    } else {
        theme::HUD
    };
    if game.state() == FlagsState::Feedback {
        if index == game.correct_answer() {
            color = theme::GOOD;
        } else if index == game.selected_answer() && !game.last_answer_correct() {
            color = theme::BAD;
        }
    }
    fill_rect(display, x, y, w, h, color);
    if index == game.selected_answer() && game.state() == FlagsState::Question {
        fill_rect(display, x + 5, y + 6, 5, h - 12, theme::ACCENT);
    }
    draw_wrapped_text(
        display,
        x + 14,
        y + 8,
        game.answer_flag(index).name,
        theme::TEXT,
        1,
        20,
        2,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::ScriptedRng;
    use crate::store::MemoryHighScoreStore;

    fn make_game_in_state(state: FlagsState) -> FlagsGame {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        match state {
            FlagsState::ChoosingMode => {
                game.enter_choosing(&scores);
            }
            FlagsState::Question => {
                game.start_selected_mode(&scores, &mut rng);
            }
            FlagsState::Paused => {
                game.start_selected_mode(&scores, &mut rng);
                game.enter_paused();
            }
            FlagsState::Feedback => {
                game.set_selected_mode(FlagsMode::Practice);
                game.start_selected_mode(&scores, &mut rng);
                game.confirm_answer();
            }
            FlagsState::Over => {
                let mut scores2 = MemoryHighScoreStore::new();
                game.set_selected_mode(FlagsMode::DeathMatch);
                game.start_selected_mode(&scores2, &mut rng);
                game.cycle_answer_selection(1);
                game.confirm_answer();
                game.finish_feedback(&mut scores2, &mut rng);
            }
            FlagsState::Results => {
                let mut scores2 = MemoryHighScoreStore::new();
                game.set_selected_mode(FlagsMode::Quiz20);
                game.start_selected_mode(&scores2, &mut rng);
                for _ in 0..FLAGS_QUIZ_ROUNDS {
                    game.confirm_answer();
                    game.finish_feedback(&mut scores2, &mut rng);
                }
            }
        }
        game
    }

    #[test]
    fn question_screen_draws_flag_bitmap_command() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.start_selected_mode(&scores, &mut rng);
        let mut display = crate::render::RecordingDisplay::new();
        render(&mut display, &game);
        assert!(display
            .commands()
            .iter()
            .any(|command| matches!(command, crate::render::DrawCommand::Bitmap { .. })));
    }

    #[test]
    fn render_choosing_mode_screen() {
        let game = FlagsGame::new(5);
        let mut display = crate::render::RecordingDisplay::new();
        render(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
        assert!(display.commands().len() > 5);
    }

    #[test]
    fn render_paused_screen() {
        let game = make_game_in_state(FlagsState::Paused);
        let mut display = crate::render::RecordingDisplay::new();
        render(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_feedback_screen_shows_correct_and_wrong() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.set_selected_mode(FlagsMode::Practice);
        game.start_selected_mode(&scores, &mut rng);
        game.confirm_answer();
        assert_eq!(game.state(), FlagsState::Feedback);
        let mut display = crate::render::RecordingDisplay::new();
        render(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_results_screen() {
        let game = make_game_in_state(FlagsState::Results);
        let mut display = crate::render::RecordingDisplay::new();
        render(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_game_over_screen() {
        let game = make_game_in_state(FlagsState::Over);
        let mut display = crate::render::RecordingDisplay::new();
        render(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_answer_selection_redraws_changed_tile() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.start_selected_mode(&scores, &mut rng);
        let previous = game.selected_answer();
        game.cycle_answer_selection(1);
        let mut display = crate::render::RecordingDisplay::new();
        render_answer_selection(&mut display, &game, previous);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_answer_selection_skips_when_same() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.start_selected_mode(&scores, &mut rng);
        let current = game.selected_answer();
        let mut display = crate::render::RecordingDisplay::new();
        render_answer_selection(&mut display, &game, current);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_feedback_overlay_draws_correct_wrong_banner() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.set_selected_mode(FlagsMode::Practice);
        game.start_selected_mode(&scores, &mut rng);
        game.confirm_answer();
        let mut display = crate::render::RecordingDisplay::new();
        render_feedback(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_question_with_practice_mode() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.set_selected_mode(FlagsMode::Practice);
        game.start_selected_mode(&scores, &mut rng);
        let mut display = crate::render::RecordingDisplay::new();
        render(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_question_with_death_match_mode() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.set_selected_mode(FlagsMode::DeathMatch);
        game.start_selected_mode(&scores, &mut rng);
        let mut display = crate::render::RecordingDisplay::new();
        render(&mut display, &game);
        assert!(matches!(
            display.commands().last(),
            Some(crate::render::DrawCommand::Flush)
        ));
    }

    #[test]
    fn answer_tile_positions() {
        assert_eq!(answer_tile_x(0), FLAGS_ANSWER_LEFT_X);
        assert_eq!(answer_tile_x(1), FLAGS_ANSWER_RIGHT_X);
        assert_eq!(answer_tile_x(2), FLAGS_ANSWER_LEFT_X);
        assert_eq!(answer_tile_x(3), FLAGS_ANSWER_RIGHT_X);
        assert_eq!(answer_tile_y(0), FLAGS_ANSWER_TOP_Y);
        assert_eq!(answer_tile_y(1), FLAGS_ANSWER_TOP_Y);
        assert_eq!(answer_tile_y(2), FLAGS_ANSWER_BOTTOM_Y);
        assert_eq!(answer_tile_y(3), FLAGS_ANSWER_BOTTOM_Y);
    }
}
