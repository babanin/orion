use crate::app::AppAction;
use crate::config::Direction;
use crate::game2048::{Game2048, Game2048Mode, MAX_CELLS};
use crate::game2048_renderer;
use crate::input::InputFrame;
use crate::render::DisplaySink;
use crate::rng::Rng;
use crate::store::HighScoreStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderResult {
    None,
    FullRedraw,
    Delta,
}

#[derive(Debug, Clone)]
pub struct Game2048Application {
    game: Game2048,
    prev_grid: [u16; MAX_CELLS],
    prev_score: u32,
    prev_best_score: u32,
    last_menu_direction_us: i64,
}

impl Game2048Application {
    pub fn new() -> Self {
        Self {
            game: Game2048::default(),
            prev_grid: [0; MAX_CELLS],
            prev_score: 0,
            prev_best_score: 0,
            last_menu_direction_us: 0,
        }
    }

    pub const fn title(&self) -> &'static str {
        "2048"
    }

    pub fn enter(&mut self, high_scores: &impl HighScoreStore) {
        self.game.enter_choosing(high_scores);
    }

    pub fn update(
        &mut self,
        display: &mut impl DisplaySink,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        let result = self.handle_input(high_scores, rng, input, now_us);

        match result {
            RenderResult::None => AppAction::None,
            RenderResult::FullRedraw => AppAction::RedrawFull,
            RenderResult::Delta => {
                game2048_renderer::render_move_delta(
                    display,
                    &self.game,
                    &self.prev_grid,
                    self.prev_score,
                    self.prev_best_score,
                );
                AppAction::None
            }
        }
    }

    pub fn render_full(&self, display: &mut impl DisplaySink) {
        match self.game.mode() {
            Game2048Mode::Choosing => {
                game2048_renderer::render_choosing(display, &self.game);
            }
            Game2048Mode::Playing | Game2048Mode::GameOver => {
                game2048_renderer::render(display, &self.game);
            }
            Game2048Mode::Paused => {
                game2048_renderer::render_pause_menu(display, &self.game);
            }
        }
    }

    pub const fn game(&self) -> &Game2048 {
        &self.game
    }

    fn handle_input(
        &mut self,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> RenderResult {
        match self.game.mode() {
            Game2048Mode::Choosing => {
                self.handle_choosing_input(high_scores, rng, input, now_us)
            }
            Game2048Mode::Playing => {
                self.handle_playing_input(high_scores, rng, input)
            }
            Game2048Mode::Paused => {
                self.handle_paused_input(high_scores, rng, input, now_us)
            }
            Game2048Mode::GameOver => {
                self.handle_game_over_input(high_scores, rng, input)
            }
        }
    }

    fn handle_choosing_input(
        &mut self,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> RenderResult {
        let mut changed = false;
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            self.game.press_switch(high_scores, rng);
            return RenderResult::FullRedraw;
        }
        if input.encoder.detents != 0 {
            changed = self.game.adjust_grid_size(input.encoder.detents);
        }
        if input.joystick.has_direction && self.accept_menu_direction(now_us) {
            match input.joystick.direction {
                Some(Direction::Left) | Some(Direction::Up) => {
                    changed = self.game.adjust_grid_size(-1) || changed;
                }
                Some(Direction::Right) | Some(Direction::Down) => {
                    changed = self.game.adjust_grid_size(1) || changed;
                }
                None => {}
            }
        }
        if changed {
            self.game.refresh_best_score(high_scores);
        }
        if changed { RenderResult::FullRedraw } else { RenderResult::None }
    }

    fn handle_playing_input(
        &mut self,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
    ) -> RenderResult {
        if let Some(direction) = input
            .joystick
            .direction
            .filter(|_| input.joystick.has_direction)
        {
            self.prev_grid = *self.game.grid();
            self.prev_score = self.game.score();
            self.prev_best_score = self.game.best_score();

            if self.game.slide(direction) {
                self.game.place_random_tile(rng);
                if self.game.is_game_over() {
                    self.game.update_best_score(high_scores);
                    return RenderResult::FullRedraw;
                }
                return RenderResult::Delta;
            } else if self.game.is_game_over() {
                self.game.enter_game_over(high_scores);
                return RenderResult::FullRedraw;
            }
        }
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            self.game.press_switch(high_scores, rng);
            return RenderResult::FullRedraw;
        }
        RenderResult::None
    }

    fn handle_paused_input(
        &mut self,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> RenderResult {
        if input.encoder.detents != 0 {
            self.game.cycle_pause_action();
            return RenderResult::FullRedraw;
        }
        if input.joystick.has_direction && self.accept_menu_direction(now_us) {
            if matches!(
                input.joystick.direction,
                Some(Direction::Up | Direction::Down)
            ) {
                self.game.cycle_pause_action();
                return RenderResult::FullRedraw;
            }
        }
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            self.game.press_switch(high_scores, rng);
            return RenderResult::FullRedraw;
        }
        RenderResult::None
    }

    fn handle_game_over_input(
        &mut self,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        input: InputFrame,
    ) -> RenderResult {
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            self.game.press_switch(high_scores, rng);
            return RenderResult::FullRedraw;
        }
        RenderResult::None
    }

    fn accept_menu_direction(&mut self, now_us: i64) -> bool {
        if now_us - self.last_menu_direction_us < 250_000 {
            return false;
        }
        self.last_menu_direction_us = now_us;
        true
    }
}

impl Default for Game2048Application {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{EncoderEvent, JoystickEvent};
    use crate::render::RecordingDisplay;
    use crate::rng::ScriptedRng;
    use crate::store::MemoryHighScoreStore;

    #[test]
    fn switch_starts_game_from_choosing() {
        let mut app = Game2048Application::new();
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 5, 3, 7]);
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
        assert_eq!(app.game().mode(), Game2048Mode::Playing);
    }

    #[test]
    fn direction_slides_tiles_during_play() {
        let mut app = Game2048Application::new();
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 5, 3, 7, 2, 8]);
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
        assert_eq!(app.game().mode(), Game2048Mode::Playing);
        display.clear();

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
        assert!(display.commands().len() > 1);
    }

    #[test]
    fn grid_size_adjustment_in_choosing() {
        let mut app = Game2048Application::new();
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
    fn pause_switch_resumes_playing() {
        let mut app = Game2048Application::new();
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 5, 3, 7]);
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
        assert_eq!(app.game().mode(), Game2048Mode::Playing);

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
        assert_eq!(app.game().mode(), Game2048Mode::Paused);

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
            3,
        );
        assert_eq!(app.game().mode(), Game2048Mode::Playing);
    }

    #[test]
    fn delta_render_produces_draw_commands() {
        let mut app = Game2048Application::new();
        let mut display = RecordingDisplay::new();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 5, 3, 7]);
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
                    has_direction: true,
                    direction: Some(Direction::Right),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );

        display.clear();
        let _ = app.update(
            &mut display,
            &mut scores,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    has_direction: true,
                    direction: Some(Direction::Left),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            3,
        );

        assert!(display.commands().len() > 1);
        assert!(matches!(display.commands().last(), Some(crate::render::DrawCommand::Flush)));
    }
}
