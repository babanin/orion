use crate::app::AppAction;
use crate::config::Direction;
use crate::input::InputFrame;
use crate::render::DisplaySink;
use crate::rng::Rng;
use crate::snake::{GameMode, SelectionField, SnakeGame};
use crate::snake_renderer;
use crate::store::HighScoreStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PauseAction {
    Continue,
    Exit,
}

#[derive(Debug, Clone)]
pub struct SnakeApplication {
    game: SnakeGame,
    pause_action: PauseAction,
    last_menu_direction_us: i64,
}

impl SnakeApplication {
    pub fn new() -> Self {
        Self {
            game: SnakeGame::default(),
            pause_action: PauseAction::Continue,
            last_menu_direction_us: 0,
        }
    }

    pub const fn title(&self) -> &'static str {
        "SNAKE"
    }

    pub fn enter(&mut self, high_scores: &impl HighScoreStore) {
        self.game.enter_choosing(high_scores);
        self.pause_action = PauseAction::Continue;
    }

    pub fn update(
        &mut self,
        display: &mut impl DisplaySink,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        let mut action = AppAction::None;
        let mut needs_full_render =
            self.handle_input(display, high_scores, rng, input, now_us, &mut action);
        if action == AppAction::ExitToLauncher {
            return action;
        }

        if self.game.due_for_tick(now_us) {
            let old_tail = self.game.tail();
            let old_length = self.game.length();
            let old_score = self.game.score();
            let old_best_score = self.game.best_score();
            self.game.mark_ticked(now_us);
            self.game.tick(high_scores, rng);
            if self.game.mode() == GameMode::Playing {
                snake_renderer::render_tick_delta(
                    display,
                    &self.game,
                    old_tail,
                    old_score,
                    old_best_score,
                    self.game.length() > old_length,
                );
            } else {
                needs_full_render = true;
            }
        }

        if needs_full_render {
            AppAction::RedrawFull
        } else {
            AppAction::None
        }
    }

    pub fn render_full(&self, display: &mut impl DisplaySink) {
        if self.game.mode() == GameMode::Paused {
            snake_renderer::render_pause_menu(
                display,
                &self.game,
                if self.pause_action == PauseAction::Continue {
                    0
                } else {
                    1
                },
            );
        } else {
            snake_renderer::render(display, &self.game);
        }
    }

    pub const fn game(&self) -> &SnakeGame {
        &self.game
    }

    fn handle_input(
        &mut self,
        display: &mut impl DisplaySink,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
        action: &mut AppAction,
    ) -> bool {
        match self.game.mode() {
            GameMode::Playing => {
                self.handle_playing_input(display, high_scores, rng, input, now_us)
            }
            GameMode::Paused => self.handle_paused_input(high_scores, rng, input, now_us, action),
            GameMode::Choosing | GameMode::Over => {
                let (changed, setup_action) =
                    self.handle_setup_input(high_scores, rng, input, now_us);
                if setup_action == AppAction::ExitToLauncher {
                    *action = AppAction::ExitToLauncher;
                }
                changed
            }
        }
    }

    fn handle_playing_input(
        &mut self,
        display: &mut impl DisplaySink,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> bool {
        if let Some(direction) = input
            .joystick
            .direction
            .filter(|_| input.joystick.has_direction)
        {
            self.game.request_direction(direction);
        }
        if input.encoder.detents != 0 && self.game.adjust_speed(high_scores, input.encoder.detents)
        {
            snake_renderer::render_hud(display, &self.game);
        }
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            self.game.press_switch(high_scores, rng, now_us);
            self.pause_action = PauseAction::Continue;
            return true;
        }
        false
    }

    fn handle_paused_input(
        &mut self,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
        action: &mut AppAction,
    ) -> bool {
        let mut changed = false;
        if input.encoder.detents != 0 {
            self.cycle_pause_action();
            changed = true;
        }
        if input.joystick.has_direction
            && self.accept_menu_direction(now_us)
            && matches!(
                input.joystick.direction,
                Some(Direction::Up | Direction::Down)
            )
        {
            self.cycle_pause_action();
            changed = true;
        }
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            if self.pause_action == PauseAction::Continue {
                self.game.press_switch(high_scores, rng, now_us);
                changed = true;
            } else {
                self.game.enter_choosing(high_scores);
                *action = AppAction::ExitToLauncher;
            }
        }
        changed
    }

    fn handle_setup_input(
        &mut self,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> (bool, AppAction) {
        let mode_before = self.game.mode();
        let mut changed = false;
        let mut action = AppAction::None;
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            if self.game.selected_field() == SelectionField::Exit {
                action = AppAction::ExitToLauncher;
            } else {
                changed = self.game.press_switch(high_scores, rng, now_us) || changed;
            }
        }
        if action == AppAction::None && input.encoder.detents != 0 {
            changed = self
                .game
                .adjust_selected_value(high_scores, input.encoder.detents)
                || changed;
        }
        if action == AppAction::None && input.joystick.has_direction && self.accept_menu_direction(now_us) {
            match input.joystick.direction {
                Some(Direction::Left) => {
                    changed = self.game.adjust_selected_value(high_scores, -1) || changed
                }
                Some(Direction::Right) => {
                    changed = self.game.adjust_selected_value(high_scores, 1) || changed
                }
                Some(Direction::Up) => changed = self.game.select_previous_field() || changed,
                Some(Direction::Down) => changed = self.game.select_next_field() || changed,
                None => {}
            }
        }
        (changed || self.game.mode() != mode_before, action)
    }

    fn accept_menu_direction(&mut self, now_us: i64) -> bool {
        if now_us - self.last_menu_direction_us < 250_000 {
            return false;
        }
        self.last_menu_direction_us = now_us;
        true
    }

    fn cycle_pause_action(&mut self) {
        self.pause_action = if self.pause_action == PauseAction::Continue {
            PauseAction::Exit
        } else {
            PauseAction::Continue
        };
    }
}

impl Default for SnakeApplication {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{EncoderEvent, JoystickEvent};
    use crate::render::{DrawCommand, RecordingDisplay};
    use crate::rng::ScriptedRng;
    use crate::store::MemoryHighScoreStore;

    fn start_playing() -> (SnakeApplication, RecordingDisplay, MemoryHighScoreStore, ScriptedRng) {
        let mut app = SnakeApplication::new();
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 12, 7]);
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
        assert_eq!(app.game().mode(), GameMode::Playing);
        (app, display, scores, rng)
    }

    #[test]
    fn switch_starts_game_and_requests_full_redraw() {
        let mut app = SnakeApplication::new();
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 12, 7]);
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
        assert_eq!(app.game().mode(), GameMode::Playing);
    }

    #[test]
    fn encoder_speed_change_renders_hud_while_playing() {
        let (mut app, mut display, mut scores, mut rng) = start_playing();
        display.clear();

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                encoder: EncoderEvent {
                    detents: 1,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );

        assert_eq!(action, AppAction::None);
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn joystick_direction_requests_direction_while_playing() {
        let (mut app, mut display, mut scores, mut rng) = start_playing();
        let _ = app.update(
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
            2,
        );
        assert_eq!(app.game().direction(), Direction::Right);
    }

    #[test]
    fn switch_pauses_while_playing() {
        let (mut app, mut display, mut scores, mut rng) = start_playing();
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
        assert_eq!(app.game().mode(), GameMode::Paused);
    }

    #[test]
    fn pause_encoder_cycles_action() {
        let (mut app, mut display, mut scores, mut rng) = start_playing();
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
        assert_eq!(app.game().mode(), GameMode::Paused);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                encoder: EncoderEvent {
                    detents: 1,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            3,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn pause_resume_continues_playing() {
        let (mut app, mut display, mut scores, mut rng) = start_playing();
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
            3,
        );
        assert_eq!(action, AppAction::RedrawFull);
        assert_eq!(app.game().mode(), GameMode::Playing);
    }

    #[test]
    fn pause_exit_returns_to_launcher() {
        let (mut app, mut display, mut scores, mut rng) = start_playing();
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
        app.cycle_pause_action();
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
            3,
        );
        assert_eq!(action, AppAction::ExitToLauncher);
    }

#[test]
    fn choosing_joystick_up_down_changes_field() {
        let mut app = SnakeApplication::new();
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
            500_000,
        );
        assert_eq!(action, AppAction::RedrawFull);
        assert_eq!(app.game().selected_field(), SelectionField::Border);
    }

    #[test]
    fn choosing_joystick_left_right_changes_speed() {
        let mut app = SnakeApplication::new();
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
                    direction: Some(Direction::Right),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            500_000,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn choosing_encoder_changes_value() {
        let mut app = SnakeApplication::new();
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        app.enter(&scores);

        let action = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                encoder: EncoderEvent {
                    detents: 1,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn choosing_switch_on_exit_returns_to_launcher() {
        let mut app = SnakeApplication::new();
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        app.enter(&scores);
        app.game.select_next_field();
        app.game.select_next_field();
        assert_eq!(app.game().selected_field(), SelectionField::Exit);

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
    fn pause_joystick_direction_cycles_action() {
        let (mut app, mut display, mut scores, mut rng) = start_playing();
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
            500_000,
        );
        assert_eq!(action, AppAction::RedrawFull);
    }

    #[test]
    fn accept_menu_direction_throttles() {
        let mut app = SnakeApplication::new();
        assert!(app.accept_menu_direction(1_000_000));
        assert!(!app.accept_menu_direction(1_000_100));
        assert!(app.accept_menu_direction(1_500_000));
    }

    #[test]
    fn render_full_when_paused() {
        let (mut app, _display, _scores, _rng) = start_playing();
        let _ = app.game.press_switch(&_scores, &mut ScriptedRng::new([0]), 2);
        assert_eq!(app.game().mode(), GameMode::Paused);
        let mut display = RecordingDisplay::new();
        app.render_full(&mut display);
        assert!(display.commands().len() > 5);
    }

    #[test]
    fn title_returns_snake() {
        let app = SnakeApplication::new();
        assert_eq!(app.title(), "SNAKE");
    }

    #[test]
    fn game_over_switch_returns_to_choosing() {
        let (mut app, mut display, mut scores, mut rng) = start_playing();
        app.game.set_mode(GameMode::Over);
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
    }
}
