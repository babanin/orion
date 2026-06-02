use crate::app::AppAction;
use crate::config::Direction;
use crate::input::InputFrame;
use crate::render::DisplaySink;
use crate::rng::Rng;
use crate::tetris::{
    TetrisChoosingAction, TetrisGame, TetrisMode, TetrisPauseAction, TetrisPiece, Tetromino,
    TETRIS_CELLS,
};
use crate::tetris_renderer;

#[derive(Debug, Clone)]
pub struct TetrisApplication {
    game: TetrisGame,
    last_direction_us: i64,
}

impl TetrisApplication {
    pub fn new() -> Self {
        Self {
            game: TetrisGame::default(),
            last_direction_us: 0,
        }
    }

    pub const fn title(&self) -> &'static str {
        "TETRIS"
    }

    pub fn enter(&mut self) {
        self.game.enter_choosing();
        self.last_direction_us = 0;
    }

    pub fn update(
        &mut self,
        display: &mut impl DisplaySink,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        let mut action = self.handle_input(display, rng, input, now_us);
        if action == AppAction::ExitToLauncher {
            return action;
        }

        if self.game.due_for_tick(now_us) {
            let previous = TetrisRenderState::capture(&self.game);
            self.game.mark_ticked(now_us);
            self.game.tick(rng);
            if self.game.mode() == TetrisMode::Playing {
                self.render_delta(display, previous);
                action = AppAction::None;
            } else {
                action = AppAction::RedrawFull;
            }
        }

        action
    }

    pub fn render_full(&self, display: &mut impl DisplaySink) {
        match self.game.mode() {
            TetrisMode::Choosing => tetris_renderer::render_choosing(display, &self.game),
            TetrisMode::Playing | TetrisMode::GameOver => {
                tetris_renderer::render(display, &self.game)
            }
            TetrisMode::Paused => tetris_renderer::render_pause_menu(display, &self.game),
        }
    }

    pub const fn game(&self) -> &TetrisGame {
        &self.game
    }

    fn handle_input(
        &mut self,
        display: &mut impl DisplaySink,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        match self.game.mode() {
            TetrisMode::Choosing => self.handle_choosing_input(rng, input, now_us),
            TetrisMode::GameOver => {
                if input.joystick.switch_pressed || input.encoder.switch_pressed {
                    self.game.press_switch(rng, now_us);
                    AppAction::RedrawFull
                } else {
                    AppAction::None
                }
            }
            TetrisMode::Playing => self.handle_playing_input(display, rng, input, now_us),
            TetrisMode::Paused => self.handle_paused_input(rng, input, now_us),
        }
    }

    fn handle_choosing_input(
        &mut self,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        if input.encoder.detents != 0 {
            self.game.cycle_choosing_action();
            return AppAction::RedrawFull;
        }
        if input.joystick.has_direction && self.accept_direction(now_us, input.joystick.direction.unwrap_or(Direction::Down))
            && matches!(
                input.joystick.direction,
                Some(Direction::Up | Direction::Down)
            )
        {
            self.game.cycle_choosing_action();
            return AppAction::RedrawFull;
        }
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            if self.game.choosing_action() == TetrisChoosingAction::Exit {
                self.game.enter_choosing();
                return AppAction::ExitToLauncher;
            }
            self.game.press_switch(rng, now_us);
            return AppAction::RedrawFull;
        }
        AppAction::None
    }

    fn handle_playing_input(
        &mut self,
        display: &mut impl DisplaySink,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        if input.joystick.switch_long_pressed || input.encoder.switch_long_pressed {
            self.game.press_switch(rng, now_us);
            return AppAction::RedrawFull;
        }

        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            let previous = TetrisRenderState::capture(&self.game);
            if self.game.move_active(Direction::Up) {
                self.render_delta(display, previous);
                return AppAction::None;
            }
        }

        if input.encoder.detents != 0 {
            let direction = if input.encoder.detents < 0 {
                Direction::Left
            } else {
                Direction::Right
            };
            let previous = TetrisRenderState::capture(&self.game);
            if self.game.move_active(direction) {
                self.render_delta(display, previous);
                return AppAction::None;
            }
        }

        if let Some(direction) = input
            .joystick
            .direction
            .filter(|_| input.joystick.has_direction)
        {
            if self.accept_direction(now_us, direction) {
                let previous = TetrisRenderState::capture(&self.game);
                if self.game.move_active(direction) {
                    self.render_delta(display, previous);
                    return AppAction::None;
                }
            }
        }

        AppAction::None
    }

    fn handle_paused_input(
        &mut self,
        rng: &mut impl Rng,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        if input.encoder.detents != 0 {
            self.game.cycle_pause_action();
            return AppAction::RedrawFull;
        }

        if input.joystick.has_direction
            && self.accept_direction(now_us, input.joystick.direction.unwrap_or(Direction::Down))
            && matches!(
                input.joystick.direction,
                Some(Direction::Up | Direction::Down)
            )
        {
            self.game.cycle_pause_action();
            return AppAction::RedrawFull;
        }

        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            if self.game.pause_action() == TetrisPauseAction::Exit {
                self.game.enter_choosing();
                return AppAction::ExitToLauncher;
            }
            self.game.press_switch(rng, now_us);
            return AppAction::RedrawFull;
        }

        AppAction::None
    }

    fn accept_direction(&mut self, now_us: i64, direction: Direction) -> bool {
        let repeat_us = if direction == Direction::Down {
            55_000
        } else {
            130_000
        };
        if now_us - self.last_direction_us < repeat_us {
            return false;
        }
        self.last_direction_us = now_us;
        true
    }

    fn render_delta(&self, display: &mut impl DisplaySink, previous: TetrisRenderState) {
        tetris_renderer::render_play_delta(
            display,
            &self.game,
            &previous.board,
            previous.active,
            previous.score,
            previous.lines,
            previous.next,
        );
    }
}

impl Default for TetrisApplication {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
struct TetrisRenderState {
    board: [u8; TETRIS_CELLS],
    active: TetrisPiece,
    score: u32,
    lines: u32,
    next: Tetromino,
}

impl TetrisRenderState {
    fn capture(game: &TetrisGame) -> Self {
        Self {
            board: *game.board(),
            active: game.active(),
            score: game.score(),
            lines: game.lines(),
            next: game.next(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{EncoderEvent, JoystickEvent};
    use crate::render::{DrawCommand, RecordingDisplay, Rect};
    use crate::rng::ScriptedRng;

    #[test]
    fn switch_starts_game_from_choosing() {
        let mut app = TetrisApplication::new();
        let mut display = RecordingDisplay::new();
        let mut rng = ScriptedRng::new([0, 1]);
        app.enter();

        let action = app.update(
            &mut display,
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
        assert_eq!(app.game().mode(), TetrisMode::Playing);
    }

    #[test]
    fn joystick_moves_active_piece() {
        let mut app = TetrisApplication::new();
        let mut display = RecordingDisplay::new();
        let mut rng = ScriptedRng::new([1, 1]);
        app.enter();
        let _ = app.update(
            &mut display,
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
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    has_direction: true,
                    direction: Some(Direction::Left),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            200_000,
        );

        assert_eq!(action, AppAction::None);
        assert_eq!(app.game().active().x, 2);
        assert_delta_rendered_without_full_clear(&display);
    }

    #[test]
    fn encoder_moves_piece_horizontally() {
        let mut app = TetrisApplication::new();
        let mut display = RecordingDisplay::new();
        let mut rng = ScriptedRng::new([1, 1]);
        app.enter();
        let _ = app.update(
            &mut display,
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
        assert_eq!(app.game().active().x, 4);
        assert_delta_rendered_without_full_clear(&display);
    }

    #[test]
    fn gravity_tick_renders_delta() {
        let mut app = TetrisApplication::new();
        let mut display = RecordingDisplay::new();
        let mut rng = ScriptedRng::new([1, 1]);
        app.enter();
        let _ = app.update(
            &mut display,
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

        let action = app.update(&mut display, &mut rng, InputFrame::default(), 800_001);

        assert_eq!(action, AppAction::None);
        assert_eq!(app.game().active().y, 1);
        assert_delta_rendered_without_full_clear(&display);
    }

    #[test]
    fn pause_exit_returns_to_launcher() {
        let mut app = TetrisApplication::new();
        let mut display = RecordingDisplay::new();
        let mut rng = ScriptedRng::new([1, 1]);
        app.enter();
        let _ = app.update(
            &mut display,
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
        let _ = app.update(
            &mut display,
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

        let action = app.update(
            &mut display,
            &mut rng,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            4,
        );

        assert_eq!(action, AppAction::ExitToLauncher);
    }

    #[test]
    fn short_switch_rotates_piece_during_play() {
        let mut app = TetrisApplication::new();
        let mut display = RecordingDisplay::new();
        let mut rng = ScriptedRng::new([0, 1]);
        app.enter();
        let _ = app.update(
            &mut display,
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
        assert_eq!(app.game().mode(), TetrisMode::Playing);
        assert_eq!(app.game().active().rotation, 1);
        assert_delta_rendered_without_full_clear(&display);
    }

    #[test]
    fn long_switch_pauses_during_play() {
        let mut app = TetrisApplication::new();
        let mut display = RecordingDisplay::new();
        let mut rng = ScriptedRng::new([0, 1]);
        app.enter();
        let _ = app.update(
            &mut display,
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
        assert_eq!(app.game().mode(), TetrisMode::Paused);
    }

    fn assert_delta_rendered_without_full_clear(display: &RecordingDisplay) {
        assert!(display.commands().len() > 1);
        assert!(!display.commands().iter().any(|command| matches!(
            command,
            DrawCommand::Fill {
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 320,
                    h: 240
                },
                ..
            }
        )));
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }
}
