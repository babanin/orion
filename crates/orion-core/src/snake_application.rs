use crate::app::AppAction;
use crate::config::Direction;
use crate::input::InputFrame;
use crate::render::DisplaySink;
use crate::rng::Rng;
use crate::snake::{GameMode, SnakeGame};
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
                self.handle_setup_input(high_scores, rng, input, now_us)
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
    ) -> bool {
        let mode_before = self.game.mode();
        let mut changed = false;
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            changed = self.game.press_switch(high_scores, rng, now_us) || changed;
        }
        if input.encoder.detents != 0 {
            changed = self
                .game
                .adjust_selected_value(high_scores, input.encoder.detents)
                || changed;
        }
        if input.joystick.has_direction && self.accept_menu_direction(now_us) {
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
        changed || self.game.mode() != mode_before
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
}
