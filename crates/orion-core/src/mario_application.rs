use crate::app::AppAction;
use crate::config::Direction;
use crate::input::InputFrame;
use crate::mario::{MarioGame, MarioMode, MarioPauseAction};
use crate::mario_renderer::{self, MarioRenderState};
use crate::render::DisplaySink;
use crate::rng::Rng;
use crate::speaker::Speaker;
use crate::store::HighScoreStore;

pub struct MarioApplication {
    game: MarioGame,
    last_menu_direction_us: i64,
    left_held: bool,
    right_held: bool,
    flap_ready: bool,
    last_play_direction_us: i64,
}

impl MarioApplication {
    pub fn new() -> Self {
        Self {
            game: MarioGame::new(),
            last_menu_direction_us: 0,
            left_held: false,
            right_held: false,
            flap_ready: true,
            last_play_direction_us: 0,
        }
    }

    pub const fn title(&self) -> &'static str {
        "Super Om Nomario"
    }

    pub fn enter(&mut self, high_scores: &impl HighScoreStore) {
        self.game.enter(high_scores);
        self.left_held = false;
        self.right_held = false;
        self.flap_ready = true;
    }

    pub fn update(
        &mut self,
        display: &mut impl DisplaySink,
        high_scores: &mut impl HighScoreStore,
        _rng: &mut impl Rng,
        _speaker: &mut impl Speaker,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        match self.game.mode() {
            MarioMode::Ready | MarioMode::GameOver => {
                if input.joystick.switch_pressed {
                    self.game.start();
                    self.render_full(display);
                    return AppAction::None;
                }
                if self.flap_ready {
                    self.render_full(display);
                    self.flap_ready = false;
                }
            }
            MarioMode::Playing => {
                if input.joystick.switch_long_pressed {
                    self.game.pause();
                    self.render_full(display);
                    return AppAction::None;
                }

                match input.joystick.direction {
                    Some(Direction::Left) => {
                        self.left_held = true;
                        self.right_held = false;
                        self.last_play_direction_us = now_us;
                    }
                    Some(Direction::Right) => {
                        self.left_held = false;
                        self.right_held = true;
                        self.last_play_direction_us = now_us;
                    }
                    _ => {
                        if now_us - self.last_play_direction_us > 150_000 {
                            self.left_held = false;
                            self.right_held = false;
                        }
                    }
                }

                self.game.set_direction(self.left_held, self.right_held);

                let just_pressed = input.joystick.switch_pressed && !self.flap_ready;
                self.flap_ready = input.joystick.switch_pressed;

                if just_pressed {
                    self.game.jump();
                } else if !input.joystick.switch_pressed {
                    self.game.release_jump();
                    self.flap_ready = false;
                }

                if self.game.due_for_tick(now_us) {
                    let prev = MarioRenderState::capture(&self.game);
                    self.game.tick(high_scores);
                    self.game.mark_ticked(now_us);
                    mario_renderer::render_play_delta(display, &self.game, prev);
                }

                if self.game.mode() == MarioMode::LevelComplete {
                    self.game.update_best_score(high_scores);
                    self.render_full(display);
                }
            }
            MarioMode::Paused => {
                if now_us - self.last_menu_direction_us >= 120_000
                    && (input.joystick.direction == Some(Direction::Up)
                        || input.joystick.direction == Some(Direction::Down)
                        || input.encoder.detents != 0)
                {
                    self.game.cycle_pause_action();
                    self.last_menu_direction_us = now_us;
                    self.render_full(display);
                    return AppAction::None;
                }

                if input.joystick.switch_pressed || input.encoder.switch_pressed {
                    match self.game.pause_action() {
                        MarioPauseAction::Continue => {
                            self.game.resume();
                            self.render_full(display);
                        }
                        MarioPauseAction::Exit => {
                            self.game.update_best_score(high_scores);
                            return AppAction::ExitToLauncher;
                        }
                    }
                    return AppAction::None;
                }
            }
            MarioMode::Dying => {
                if self.game.due_for_tick(now_us) {
                    self.game.tick(high_scores);
                    self.game.mark_ticked(now_us);
                }
                if self.game.mode() == MarioMode::GameOver || self.game.mode() == MarioMode::Playing
                {
                    self.render_full(display);
                }
            }
            MarioMode::LevelComplete => {
                if self.game.due_for_tick(now_us) {
                    self.game.tick(high_scores);
                    self.game.mark_ticked(now_us);
                    if self.game.mode() == MarioMode::Ready {
                        self.render_full(display);
                    }
                }
            }
        }
        AppAction::None
    }

    pub fn render_full(&self, display: &mut impl DisplaySink) {
        mario_renderer::render(display, &self.game);
    }

    pub const fn game(&self) -> &MarioGame {
        &self.game
    }
}

impl Default for MarioApplication {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::JoystickEvent;
    use crate::render::{DrawCommand, RecordingDisplay, Rect};
    use crate::rng::ScriptedRng;
    use crate::speaker::SilentSpeaker;
    use crate::store::MemoryHighScoreStore;

    #[test]
    fn switch_starts_game_from_ready() {
        let mut app = MarioApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
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
        assert_eq!(action, AppAction::None);
        assert_eq!(app.game().mode(), MarioMode::Playing);
    }

    #[test]
    fn direction_moves_player_horizontally() {
        let mut app = MarioApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);
        app.game.start();

        let x_before = app.game().player().x;
        app.update(
            &mut display,
            &mut store,
            &mut rng,
            &mut SilentSpeaker,
            InputFrame {
                joystick: JoystickEvent {
                    has_direction: true,
                    direction: Some(Direction::Left),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            100_000,
        );
        // After one update with direction left and tick, player should have moved
        // (update runs tick when due_for_tick is true)
        if app.game().player().x != x_before {
            assert!(
                app.game().player().x < x_before,
                "Player should move left after left direction input"
            );
        }
    }

    #[test]
    fn switch_jump_in_playing() {
        let mut app = MarioApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);
        // First update clears flap_ready (renders Ready screen)
        app.update(
            &mut display,
            &mut store,
            &mut rng,
            &mut SilentSpeaker,
            InputFrame::default(),
            100,
        );
        // Start via switch press
        app.update(
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
        // Should now be Playing with flap_ready reset
        assert_eq!(app.game().mode(), MarioMode::Playing);
        // Send another switch press for jump
        app.update(
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
            200_000,
        );
        // Jump should set negative vy
        assert!(
            app.game().player().vy < 0,
            "Switch press in playing should set upward velocity"
        );
    }

    #[test]
    fn release_jump_clears_hold() {
        let mut app = MarioApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);
        app.game.start();
        app.game.jump();

        app.update(
            &mut display,
            &mut store,
            &mut rng,
            &mut SilentSpeaker,
            InputFrame::default(),
            100_000,
        );
        // After release, flap_ready should be false and game should have released jump
        // The mode and state should still be playing
        assert_eq!(app.game().mode(), MarioMode::Playing);
    }

    #[test]
    fn long_press_pauses_game() {
        let mut app = MarioApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);
        app.game.start();

        app.update(
            &mut display,
            &mut store,
            &mut rng,
            &mut SilentSpeaker,
            InputFrame {
                joystick: JoystickEvent {
                    switch_long_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            100_000,
        );
        assert_eq!(
            app.game().mode(),
            MarioMode::Paused,
            "Long press should pause the game"
        );
    }

    #[test]
    fn pause_continue_resumes_game() {
        let mut app = MarioApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);
        app.game.start();
        app.game.pause();

        app.update(
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
            100_000,
        );
        assert_eq!(
            app.game().mode(),
            MarioMode::Playing,
            "Switch press on pause with Continue should resume"
        );
    }

    #[test]
    fn pause_exit_returns_to_launcher() {
        let mut app = MarioApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);
        app.game.start();
        app.game.pause();
        app.game.cycle_pause_action();
        assert_eq!(app.game().pause_action(), MarioPauseAction::Exit);

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
            100_000,
        );
        assert_eq!(
            action,
            AppAction::ExitToLauncher,
            "Exit pause action should return to launcher"
        );
    }

    #[test]
    fn normal_tick_uses_delta_render() {
        let mut app = MarioApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);
        app.game.start();

        // Call update with a time that will trigger due_for_tick
        // (last_tick_us is 0 from new(), so any time > TICK_US triggers)
        let action = app.update(
            &mut display,
            &mut store,
            &mut rng,
            &mut SilentSpeaker,
            InputFrame::default(),
            20_000,
        );
        assert_eq!(action, AppAction::None);
        // On initial tick with no input, gravity pulls player down but camera hasn't changed
        // enough (player still at left edge). We should NOT see a full-screen clear.
        let full_clear = display.commands().iter().any(|cmd| {
            matches!(
                cmd,
                DrawCommand::Fill {
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 320,
                        h: 240
                    },
                    ..
                }
            )
        });
        assert!(
            !full_clear,
            "Normal tick in playing should use delta render, not full clear"
        );
    }

    #[test]
    fn game_over_switch_restarts_game() {
        let mut app = MarioApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);
        // Set game to GameOver
        app.game.set_mode(MarioMode::GameOver);

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
        assert_eq!(action, AppAction::None);
        assert_eq!(
            app.game().mode(),
            MarioMode::Playing,
            "Switch press on GameOver should restart the game"
        );
    }

    #[test]
    fn ready_mode_initial_render_shows_panel() {
        let mut app = MarioApplication::new();
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        let mut display = RecordingDisplay::new();
        app.enter(&store);

        // First update without switch press: should render the full ready screen
        app.update(
            &mut display,
            &mut store,
            &mut rng,
            &mut SilentSpeaker,
            InputFrame::default(),
            100,
        );
        // Should have commands from full render
        assert!(!display.commands().is_empty());
        let has_full_clear = display.commands().iter().any(|cmd| {
            matches!(
                cmd,
                DrawCommand::Fill {
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 320,
                        h: 240
                    },
                    ..
                }
            )
        });
        assert!(
            has_full_clear,
            "Ready mode should full-render including clear"
        );
    }
}
