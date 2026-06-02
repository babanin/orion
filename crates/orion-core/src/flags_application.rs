use crate::app::AppAction;
use crate::config::Direction;
use crate::flags::{
    FlagsChoosingAction, FlagsGame, FlagsPauseAction, FlagsResultAction, FlagsState,
    FLAGS_FEEDBACK_MS,
};
use crate::flags_renderer;
use crate::input::InputFrame;
use crate::render::DisplaySink;
use crate::rng::Rng;
use crate::store::HighScoreStore;

#[derive(Debug, Clone)]
pub struct FlagsApplication {
    game: FlagsGame,
    last_menu_direction_us: i64,
    feedback_start_us: i64,
}

impl FlagsApplication {
    pub fn new(flag_count: usize) -> Self {
        Self {
            game: FlagsGame::new(flag_count),
            last_menu_direction_us: 0,
            feedback_start_us: 0,
        }
    }

    pub const fn title(&self) -> &'static str {
        "FLAGS"
    }

    pub fn enter(&mut self, high_scores: &impl HighScoreStore) {
        self.game.enter_choosing(high_scores);
        self.feedback_start_us = 0;
    }

    pub fn update(
        &mut self,
        display: &mut impl DisplaySink,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        if self.game.state() == FlagsState::Feedback
            && now_us - self.feedback_start_us >= FLAGS_FEEDBACK_MS as i64 * 1000
        {
            self.game.finish_feedback(high_scores, rng);
            return AppAction::RedrawFull;
        }

        let mut action = AppAction::None;
        let changed = match self.game.state() {
            FlagsState::ChoosingMode => {
                self.handle_mode_input(high_scores, rng, input, now_us, &mut action)
            }
            FlagsState::Question => self.handle_question_input(display, input, now_us),
            FlagsState::Feedback => false,
            FlagsState::Paused => {
                self.handle_paused_input(high_scores, rng, input, now_us, &mut action)
            }
            FlagsState::Results | FlagsState::Over => {
                self.handle_result_input(high_scores, rng, input, now_us, &mut action)
            }
        };

        if action != AppAction::None {
            action
        } else if changed {
            AppAction::RedrawFull
        } else {
            AppAction::None
        }
    }

    pub fn render_full(&self, display: &mut impl DisplaySink) {
        flags_renderer::render(display, &self.game);
    }

    pub const fn game(&self) -> &FlagsGame {
        &self.game
    }

    fn handle_mode_input(
        &mut self,
        high_scores: &impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
        action: &mut AppAction,
    ) -> bool {
        let mut changed = false;
        if input.encoder.detents != 0 {
            if input.encoder.detents > 0 {
                changed = self.game.cycle_choosing_action_down() || changed;
            } else {
                changed = self.game.cycle_choosing_action_up() || changed;
            }
        }
        if input.joystick.has_direction && self.accept_menu_direction(now_us) {
            if input.joystick.direction == Some(Direction::Up) {
                changed = self.game.cycle_choosing_action_up() || changed;
            } else if input.joystick.direction == Some(Direction::Down) {
                changed = self.game.cycle_choosing_action_down() || changed;
            }
        }
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            if self.game.choosing_action() == FlagsChoosingAction::Exit {
                *action = AppAction::ExitToLauncher;
                return false;
            }
            self.game.start_selected_mode(high_scores, rng);
            changed = true;
        }
        changed
    }

    fn handle_question_input(
        &mut self,
        display: &mut impl DisplaySink,
        input: InputFrame,
        now_us: i64,
    ) -> bool {
        if input.joystick.switch_long_pressed || input.encoder.switch_long_pressed {
            self.game.enter_paused();
            return true;
        }

        let previous = self.game.selected_answer();
        let mut answer_changed = false;
        if input.encoder.detents != 0 {
            answer_changed =
                self.game.cycle_answer_selection(input.encoder.detents) || answer_changed;
        }
        if input.joystick.has_direction && self.accept_menu_direction(now_us) {
            if let Some(direction) = input.joystick.direction {
                answer_changed = self.game.move_answer_selection(direction) || answer_changed;
            }
        }
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            self.game.confirm_answer();
            self.feedback_start_us = now_us;
            flags_renderer::render_feedback(display, &self.game);
        } else if answer_changed {
            flags_renderer::render_answer_selection(display, &self.game, previous);
        }
        false
    }

    fn handle_paused_input(
        &mut self,
        high_scores: &impl HighScoreStore,
        _rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
        action: &mut AppAction,
    ) -> bool {
        if input.encoder.detents != 0 {
            self.game.cycle_pause_action();
            return true;
        }
        if input.joystick.has_direction
            && self.accept_menu_direction(now_us)
            && matches!(
                input.joystick.direction,
                Some(Direction::Up | Direction::Down)
            )
        {
            self.game.cycle_pause_action();
            return true;
        }
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            if self.game.pause_action() == FlagsPauseAction::Exit {
                self.game.enter_choosing(high_scores);
                *action = AppAction::ExitToLauncher;
                return false;
            }
            self.game.resume_from_pause();
            return true;
        }
        false
    }

    fn handle_result_input(
        &mut self,
        high_scores: &impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
        action: &mut AppAction,
    ) -> bool {
        let mut changed = false;
        if input.encoder.detents != 0 {
            changed = self.game.cycle_result_action(input.encoder.detents) || changed;
        }
        if input.joystick.has_direction
            && self.accept_menu_direction(now_us)
            && matches!(
                input.joystick.direction,
                Some(Direction::Up | Direction::Down)
            )
        {
            changed = self.game.select_next_result_action() || changed;
        }
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            if self.game.result_action() == FlagsResultAction::Restart {
                self.game.start_selected_mode(high_scores, rng);
                changed = true;
            } else {
                self.game.enter_choosing(high_scores);
                *action = AppAction::ExitToLauncher;
            }
        }
        changed
    }

    fn accept_menu_direction(&mut self, now_us: i64) -> bool {
        if now_us - self.last_menu_direction_us < 250_000 {
            return false;
        }
        self.last_menu_direction_us = now_us;
        true
    }
}

impl Default for FlagsApplication {
    fn default() -> Self {
        Self::new(crate::generated::flags_assets::FLAG_ASSET_COUNT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::FlagsMode;
    use crate::input::JoystickEvent;
    use crate::render::{DrawCommand, RecordingDisplay};
    use crate::rng::ScriptedRng;
    use crate::store::MemoryHighScoreStore;

    #[test]
    fn switch_starts_selected_mode() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );

        assert_eq!(action, AppAction::RedrawFull);
        assert_eq!(app.game().state(), FlagsState::Question);
    }

    #[test]
    fn answer_confirm_renders_feedback_without_full_redraw_action() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        display.clear();

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );

        assert_eq!(action, AppAction::None);
        assert_eq!(app.game().state(), FlagsState::Feedback);
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn choosing_exit_returns_to_launcher() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        app.enter(&scores);

        app.game.cycle_choosing_action_down();
        app.game.cycle_choosing_action_down();
        app.game.cycle_choosing_action_down();

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );

        assert_eq!(action, AppAction::ExitToLauncher);
    }

    #[test]
    fn long_press_during_question_enters_pause() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        assert_eq!(app.game().state(), FlagsState::Question);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_long_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(action, AppAction::RedrawFull);
        assert_eq!(app.game().state(), FlagsState::Paused);
    }

    #[test]
    fn pause_exit_returns_to_launcher() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );

        app.game.enter_paused();
        app.game.cycle_pause_action();

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(action, AppAction::ExitToLauncher);
    }

    #[test]
    fn choosing_encoder_down_cycles_action() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        app.enter(&scores);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                encoder: crate::input::EncoderEvent {
                    detents: 1,
                    ..crate::input::EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            1_000_000,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn choosing_encoder_up_cycles_action() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        app.enter(&scores);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                encoder: crate::input::EncoderEvent {
                    detents: -1,
                    ..crate::input::EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            1_000_000,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn choosing_joystick_up_cycles_action() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        app.enter(&scores);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    has_direction: true,
                    direction: Some(Direction::Up),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1_000_000,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn choosing_joystick_down_cycles_action() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        app.enter(&scores);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    has_direction: true,
                    direction: Some(Direction::Down),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1_000_000,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn question_encoder_moves_answer() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                encoder: crate::input::EncoderEvent {
                    detents: 1,
                    ..crate::input::EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(action, AppAction::None);
    }

    #[test]
    fn question_joystick_direction_moves_answer() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    has_direction: true,
                    direction: Some(Direction::Right),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(action, AppAction::None);
    }

    #[test]
    fn question_encoder_switch_confirms_answer() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                encoder: crate::input::EncoderEvent {
                    switch_pressed: true,
                    ..crate::input::EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(action, AppAction::None);
        assert_eq!(app.game().state(), FlagsState::Feedback);
    }

    #[test]
    fn question_encoder_long_press_enters_pause() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                encoder: crate::input::EncoderEvent {
                    switch_long_pressed: true,
                    ..crate::input::EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(action, AppAction::RedrawFull);
        assert_eq!(app.game().state(), FlagsState::Paused);
    }

    #[test]
    fn paused_encoder_cycles_action() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        app.game.enter_paused();

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                encoder: crate::input::EncoderEvent {
                    detents: 1,
                    ..crate::input::EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn paused_joystick_direction_cycles_action() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        app.game.enter_paused();

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    has_direction: true,
                    direction: Some(Direction::Down),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            2_000_000,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn paused_switch_resumes() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        app.game.enter_paused();

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(action, AppAction::RedrawFull);
        assert_eq!(app.game().state(), FlagsState::Question);
    }

    #[test]
    fn result_encoder_cycles_action() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        app.game.set_selected_mode(FlagsMode::DeathMatch);
        app.game.start_selected_mode(&scores, &mut rng);
        assert_eq!(app.game().state(), FlagsState::Question);

        app.game.move_answer_selection(Direction::Right);
        app.game.confirm_answer();
        assert_eq!(app.game().state(), FlagsState::Feedback);

        app.game.finish_feedback(&mut scores, &mut rng);
        assert_eq!(app.game().state(), FlagsState::Over);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                encoder: crate::input::EncoderEvent {
                    detents: 1,
                    ..crate::input::EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            1_000_000,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn result_joystick_cycles_action() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        app.game.set_selected_mode(FlagsMode::DeathMatch);
        app.game.start_selected_mode(&scores, &mut rng);
        assert_eq!(app.game().state(), FlagsState::Question);

        app.game.move_answer_selection(Direction::Right);
        app.game.confirm_answer();
        assert_eq!(app.game().state(), FlagsState::Feedback);

        app.game.finish_feedback(&mut scores, &mut rng);
        assert_eq!(app.game().state(), FlagsState::Over);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    has_direction: true,
                    direction: Some(Direction::Down),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1_000_000,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn result_switch_restarts() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        app.game.set_selected_mode(FlagsMode::DeathMatch);
        app.game.start_selected_mode(&scores, &mut rng);
        assert_eq!(app.game().state(), FlagsState::Question);

        app.game.move_answer_selection(Direction::Right);
        app.game.confirm_answer();
        assert_eq!(app.game().state(), FlagsState::Feedback);

        app.game.finish_feedback(&mut scores, &mut rng);
        assert_eq!(app.game().state(), FlagsState::Over);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        assert_eq!(action, AppAction::RedrawFull);
        assert_eq!(app.game().state(), FlagsState::Question);
    }

    #[test]
    fn result_switch_exit_enters_choosing() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        app.game.set_selected_mode(FlagsMode::DeathMatch);
        app.game.start_selected_mode(&scores, &mut rng);
        assert_eq!(app.game().state(), FlagsState::Question);

        app.game.move_answer_selection(Direction::Right);
        app.game.confirm_answer();
        assert_eq!(app.game().state(), FlagsState::Feedback);

        app.game.finish_feedback(&mut scores, &mut rng);
        assert_eq!(app.game().state(), FlagsState::Over);

        app.game.cycle_result_action(1);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        assert_eq!(action, AppAction::ExitToLauncher);
    }

    #[test]
    fn feedback_auto_advances_after_timeout() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        app.enter(&scores);
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(app.game().state(), FlagsState::Feedback);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame::default(),
            2 + FLAGS_FEEDBACK_MS as i64 * 1000 + 1,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn render_full_calls_renderer() {
        let mut app = FlagsApplication::new(5);
        let scores = MemoryHighScoreStore::new();
        app.enter(&scores);
        let mut display = RecordingDisplay::new();
        app.render_full(&mut display);
        assert!(!display.commands().is_empty());
    }

    #[test]
    fn title_returns_flags() {
        let app = FlagsApplication::new(5);
        assert_eq!(app.title(), "FLAGS");
    }

    #[test]
    fn default_creates_with_flag_asset_count() {
        let app = FlagsApplication::default();
        assert!(matches!(app.game().state(), FlagsState::ChoosingMode));
    }

    #[test]
    fn menu_direction_throttle_rejects_rapid_input() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        app.enter(&scores);

        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    has_direction: true,
                    direction: Some(Direction::Down),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1_000_000,
        );

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    has_direction: true,
                    direction: Some(Direction::Down),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1_000_100,
        );
        assert_eq!(action, AppAction::None);
    }

    #[test]
    fn no_input_returns_none() {
        let mut app = FlagsApplication::new(5);
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        app.enter(&scores);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame::default(),
            1,
        );
        assert_eq!(action, AppAction::None);
    }
}
