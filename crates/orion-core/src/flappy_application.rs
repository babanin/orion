use crate::app::AppAction;
use crate::config::Direction;
use crate::flappy::{FlappyGame, FlappyMode, FlappyPauseAction};
use crate::flappy_renderer::{self, FlappyRenderState};
use crate::input::InputFrame;
use crate::render::DisplaySink;
use crate::rng::Rng;
use crate::speaker::Speaker;
use crate::store::HighScoreStore;

#[derive(Debug, Clone)]
pub struct FlappyApplication {
    game: FlappyGame,
    last_menu_direction_us: i64,
    last_flap_us: i64,
    flap_ready: bool,
}

impl FlappyApplication {
    pub fn new() -> Self {
        Self {
            game: FlappyGame::default(),
            last_menu_direction_us: 0,
            last_flap_us: 0,
            flap_ready: true,
        }
    }

    pub const fn title(&self) -> &'static str {
        "OM NOM"
    }

    pub fn enter(&mut self, high_scores: &impl HighScoreStore) {
        self.game.enter(high_scores);
        self.last_menu_direction_us = 0;
        self.last_flap_us = 0;
        self.flap_ready = true;
    }

    pub fn update(
        &mut self,
        display: &mut impl DisplaySink,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        speaker: &mut impl Speaker,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        let mode_before = self.game.mode();
        let mut action = self.handle_input(display, high_scores, rng, speaker, input, now_us);
        if action == AppAction::ExitToLauncher {
            return action;
        }

        if self.game.due_for_tick(now_us) {
            let previous = FlappyRenderState::capture(&self.game);
            self.game.mark_ticked(now_us);
            self.game.tick(high_scores, rng);
            if self.game.mode() == FlappyMode::Playing {
                flappy_renderer::render_play_delta(display, &self.game, previous);
                action = AppAction::None;
            } else {
                action = AppAction::RedrawFull;
            }
        }

        if mode_before == FlappyMode::Playing && self.game.mode() == FlappyMode::GameOver {
            speaker.beep(220, 300);
        }

        action
    }

    pub fn render_full(&self, display: &mut impl DisplaySink) {
        flappy_renderer::render(display, &self.game);
    }

    pub const fn game(&self) -> &FlappyGame {
        &self.game
    }

    fn handle_input(
        &mut self,
        display: &mut impl DisplaySink,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
        speaker: &mut impl Speaker,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        match self.game.mode() {
            FlappyMode::Ready | FlappyMode::GameOver => {
                if input.joystick.switch_pressed || input.encoder.switch_pressed {
                    self.game.start(high_scores, rng, now_us);
                    self.flap_ready = false;
                    return AppAction::RedrawFull;
                }
                AppAction::None
            }
            FlappyMode::Playing => {
                if input.joystick.switch_pressed || input.encoder.switch_pressed {
                    self.game.pause();
                    return AppAction::RedrawFull;
                }

                let up_held =
                    input.joystick.has_direction && input.joystick.direction == Some(Direction::Up);
                if !up_held {
                    self.flap_ready = true;
                }
                if up_held && self.flap_ready && now_us - self.last_flap_us >= 120_000 {
                    let previous = FlappyRenderState::capture(&self.game);
                    self.game.flap();
                    self.last_flap_us = now_us;
                    self.flap_ready = false;
                    speaker.beep(880, 40);
                    flappy_renderer::render_play_delta(display, &self.game, previous);
                    return AppAction::None;
                }
                AppAction::None
            }
            FlappyMode::Paused => self.handle_paused_input(input, now_us),
        }
    }

    fn handle_paused_input(&mut self, input: InputFrame, now_us: i64) -> AppAction {
        if input.encoder.detents != 0 {
            self.game.cycle_pause_action();
            return AppAction::RedrawFull;
        }
        if input.joystick.has_direction
            && self.accept_menu_direction(now_us)
            && matches!(
                input.joystick.direction,
                Some(Direction::Up | Direction::Down)
            )
        {
            self.game.cycle_pause_action();
            return AppAction::RedrawFull;
        }
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            if self.game.pause_action() == FlappyPauseAction::Exit {
                return AppAction::ExitToLauncher;
            }
            self.game.resume(now_us);
            self.flap_ready = false;
            return AppAction::RedrawFull;
        }
        AppAction::None
    }

    fn accept_menu_direction(&mut self, now_us: i64) -> bool {
        if now_us - self.last_menu_direction_us < 250_000 {
            return false;
        }
        self.last_menu_direction_us = now_us;
        true
    }
}

impl Default for FlappyApplication {
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
    use crate::speaker::SilentSpeaker;
    use crate::store::MemoryHighScoreStore;

    #[test]
    fn switch_starts_game() {
        let mut app = FlappyApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);

        let action = app.update(
            &mut display,
            &mut store,
            &mut rng,
            &mut SilentSpeaker,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            100,
        );
        assert_eq!(action, AppAction::RedrawFull);
        assert_eq!(app.game().mode(), FlappyMode::Playing);
    }

    #[test]
    fn joystick_up_flaps_and_held_input_is_guarded() {
        let mut app = FlappyApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);
        app.game.start(&store, &mut rng, 0);

        let input = InputFrame {
            joystick: JoystickEvent {
                has_direction: true,
                direction: Some(Direction::Up),
                ..JoystickEvent::default()
            },
            ..InputFrame::default()
        };
        app.last_flap_us = -200_000;
        app.update(&mut display, &mut store, &mut rng, &mut SilentSpeaker, input, 200_000);
        let first_velocity = app.game().velocity_fp();
        app.game.mark_ticked(400_000);
        app.update(&mut display, &mut store, &mut rng, &mut SilentSpeaker, input, 400_000);
        assert_eq!(app.game().velocity_fp(), first_velocity);
    }

    #[test]
    fn switch_pauses_and_continue_resumes() {
        let mut app = FlappyApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);
        app.game.start(&store, &mut rng, 0);

        app.update(
            &mut display,
            &mut store,
            &mut rng,
            &mut SilentSpeaker,
            InputFrame {
                encoder: EncoderEvent {
                    switch_pressed: true,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            100,
        );
        assert_eq!(app.game().mode(), FlappyMode::Paused);

        let action = app.update(
            &mut display,
            &mut store,
            &mut rng,
            &mut SilentSpeaker,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            200,
        );
        assert_eq!(action, AppAction::RedrawFull);
        assert_eq!(app.game().mode(), FlappyMode::Playing);
    }

    #[test]
    fn pause_exit_returns_to_launcher() {
        let mut app = FlappyApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);
        app.game.start(&store, &mut rng, 0);
        app.game.pause();
        app.game.cycle_pause_action();

        let action = app.update(
            &mut display,
            &mut store,
            &mut rng,
            &mut SilentSpeaker,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            100,
        );
        assert_eq!(action, AppAction::ExitToLauncher);
    }

    #[test]
    fn normal_tick_uses_delta_render() {
        let mut app = FlappyApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);
        app.game.start(&store, &mut rng, 0);

        let action = app.update(
            &mut display,
            &mut store,
            &mut rng,
            &mut SilentSpeaker,
            InputFrame::default(),
            crate::flappy::FLAPPY_TICK_US + 1,
        );
        assert_eq!(action, AppAction::None);
        assert!(!display.commands().iter().any(|command| {
            matches!(
                command,
                DrawCommand::Fill {
                    rect: crate::render::Rect {
                        x: 0,
                        y: 0,
                        w: 320,
                        h: 240
                    },
                    ..
                }
            )
        }));
    }
}
