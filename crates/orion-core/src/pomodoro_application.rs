use crate::app::AppAction;
use crate::config::Direction;
use crate::input::InputFrame;
use crate::melody::{MelodyPlayer, Note};
use crate::pomodoro::{PomodoroMode, PomodoroPauseAction, PomodoroTimer};
use crate::pomodoro_renderer;
use crate::render::DisplaySink;
use crate::speaker::Speaker;
use crate::pomodoro_store::PomodoroSettingsStore;

const MENU_REPEAT_US: i64 = 250_000;
const ALERT_TOTAL_US: i64 = 10_000_000;

const NOKIA_TUNE: [Note; 14] = [
    Note::tone(1319, 167_000),
    Note::tone(1175, 167_000),
    Note::tone(740, 83_000),
    Note::tone(831, 83_000),
    Note::tone(1109, 167_000),
    Note::tone(988, 167_000),
    Note::tone(587, 83_000),
    Note::tone(659, 83_000),
    Note::tone(988, 167_000),
    Note::tone(880, 167_000),
    Note::tone(554, 83_000),
    Note::tone(659, 83_000),
    Note::tone(880, 667_000),
    Note::rest(333_000),
];

#[derive(Debug, Clone)]
pub struct PomodoroApplication {
    timer: PomodoroTimer,
    last_menu_direction_us: i64,
    alert_started_us: i64,
    alert_active: bool,
    melody: MelodyPlayer,
}

impl PomodoroApplication {
    pub const fn new() -> Self {
        Self {
            timer: PomodoroTimer::new(),
            last_menu_direction_us: 0,
            alert_started_us: 0,
            alert_active: false,
            melody: MelodyPlayer::new(),
        }
    }

    pub const fn title(&self) -> &'static str {
        "POMODORO"
    }

    pub fn enter(&mut self, store: &impl PomodoroSettingsStore) {
        self.timer.load_settings(store.pomodoro_minutes(), store.pomodoro_seconds());
        self.timer.reset_to_setup();
        self.last_menu_direction_us = 0;
        self.alert_started_us = 0;
        self.alert_active = false;
        self.melody = MelodyPlayer::new();
    }

    pub fn update(
        &mut self,
        display: &mut impl DisplaySink,
        speaker: &mut impl Speaker,
        store: &mut impl PomodoroSettingsStore,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        let mut action = match self.timer.mode() {
            PomodoroMode::Setup => self.handle_setup_input(display, input, now_us),
            PomodoroMode::Running => self.handle_running_input(input, now_us),
            PomodoroMode::Paused => self.handle_paused_input(input, now_us),
            PomodoroMode::Finished => self.handle_finished_input(speaker, input),
        };

        if action == AppAction::ExitToLauncher {
            store.save_pomodoro_settings(self.timer.minutes(), self.timer.seconds());
            self.stop_alert(speaker);
            return action;
        }

        let previous_remaining_seconds = self.timer.remaining_seconds();
        if self.timer.mode() == PomodoroMode::Running && self.timer.update_running(now_us) {
            if self.timer.mode() == PomodoroMode::Finished {
                self.start_alert(speaker, now_us);
                action = AppAction::RedrawFull;
            } else if action == AppAction::None {
                pomodoro_renderer::render_running_time_delta(
                    display,
                    previous_remaining_seconds,
                    self.timer.remaining_seconds(),
                );
                action = AppAction::None;
            } else {
                action = AppAction::RedrawFull;
            }
        }

        if self.timer.mode() == PomodoroMode::Finished {
            self.update_alert(speaker, now_us);
        }

        action
    }

    pub fn render_full(&self, display: &mut impl DisplaySink) {
        pomodoro_renderer::render(display, &self.timer);
    }

    pub const fn timer(&self) -> &PomodoroTimer {
        &self.timer
    }

    fn handle_setup_input(
        &mut self,
        display: &mut impl DisplaySink,
        input: InputFrame,
        now_us: i64,
    ) -> AppAction {
        if input.joystick.switch_long_pressed || input.encoder.switch_long_pressed {
            return AppAction::ExitToLauncher;
        }
        if input.encoder.switch_pressed {
            self.timer.toggle_active_field();
            pomodoro_renderer::render_setup_editor_delta(display, &self.timer, false);
            return AppAction::None;
        }
        if input.encoder.detents != 0 && self.adjust_setup_duration(display, input.encoder.detents)
        {
            return AppAction::None;
        }
        if input.joystick.has_direction && self.accept_menu_direction(now_us) {
            match input.joystick.direction {
                Some(Direction::Up) => {
                    if self.adjust_setup_duration(display, 1) {
                        return AppAction::None;
                    }
                }
                Some(Direction::Down) => {
                    if self.adjust_setup_duration(display, -1) {
                        return AppAction::None;
                    }
                }
                _ => {}
            }
        }
        if input.joystick.switch_pressed && self.timer.start(now_us) {
            return AppAction::RedrawFull;
        }
        AppAction::None
    }

    fn adjust_setup_duration(&mut self, display: &mut impl DisplaySink, delta: i32) -> bool {
        let was_startable = self.timer.can_start();
        if !self.timer.adjust_active_field(delta) {
            return false;
        }
        pomodoro_renderer::render_setup_editor_delta(
            display,
            &self.timer,
            was_startable != self.timer.can_start(),
        );
        true
    }

    fn handle_running_input(&mut self, input: InputFrame, now_us: i64) -> AppAction {
        if input.joystick.switch_long_pressed || input.encoder.switch_long_pressed {
            return AppAction::ExitToLauncher;
        }
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            if self.timer.pause(now_us) {
                return AppAction::RedrawFull;
            }
        }
        AppAction::None
    }

    fn handle_paused_input(&mut self, input: InputFrame, now_us: i64) -> AppAction {
        if input.joystick.switch_long_pressed || input.encoder.switch_long_pressed {
            return AppAction::ExitToLauncher;
        }
        if input.encoder.detents != 0 {
            self.timer.toggle_pause_action();
            return AppAction::RedrawFull;
        }
        if input.joystick.has_direction
            && self.accept_menu_direction(now_us)
            && matches!(
                input.joystick.direction,
                Some(Direction::Up | Direction::Down)
            )
        {
            self.timer.toggle_pause_action();
            return AppAction::RedrawFull;
        }
        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            if self.timer.pause_action() == PomodoroPauseAction::Exit {
                self.timer.reset_to_setup();
                return AppAction::ExitToLauncher;
            }
            if self.timer.resume(now_us) {
                return AppAction::RedrawFull;
            }
        }
        AppAction::None
    }

    fn handle_finished_input(
        &mut self,
        speaker: &mut impl Speaker,
        input: InputFrame,
    ) -> AppAction {
        if input.joystick.switch_pressed
            || input.encoder.switch_pressed
            || input.joystick.switch_long_pressed
            || input.encoder.switch_long_pressed
        {
            self.stop_alert(speaker);
            self.timer.reset_to_setup();
            return AppAction::RedrawFull;
        }
        AppAction::None
    }

    fn accept_menu_direction(&mut self, now_us: i64) -> bool {
        if now_us - self.last_menu_direction_us < MENU_REPEAT_US {
            return false;
        }
        self.last_menu_direction_us = now_us;
        true
    }

    fn start_alert(&mut self, speaker: &mut impl Speaker, now_us: i64) {
        self.alert_started_us = now_us;
        self.alert_active = true;
        speaker.set_volume(100);
        self.melody.start(&NOKIA_TUNE, speaker, now_us);
    }

    fn update_alert(&mut self, speaker: &mut impl Speaker, now_us: i64) {
        if !self.alert_active {
            return;
        }
        if now_us - self.alert_started_us >= ALERT_TOTAL_US {
            self.melody.stop(speaker);
            self.alert_active = false;
            return;
        }
        self.melody.update(speaker, now_us);
    }

    fn stop_alert(&mut self, speaker: &mut impl Speaker) {
        if self.alert_active || self.melody.is_active() {
            self.melody.stop(speaker);
        }
        self.alert_active = false;
    }
}

impl Default for PomodoroApplication {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{EncoderEvent, JoystickEvent};
    use crate::render::RecordingDisplay;
    use crate::pomodoro_store::MemoryPomodoroSettingsStore;
    use crate::pomodoro_store::PomodoroSettingsStore;

    #[derive(Default)]
    struct RecordingSpeaker {
        tones: [u32; 16],
        tone_count: usize,
        stops: usize,
        volume: u8,
    }

    impl Speaker for RecordingSpeaker {
        fn play_tone(&mut self, freq_hz: u32) {
            self.tones[self.tone_count] = freq_hz;
            self.tone_count += 1;
        }

        fn stop(&mut self) {
            self.stops += 1;
        }

        fn set_volume(&mut self, volume: u8) {
            self.volume = volume;
        }
    }

    #[test]
    fn encoder_press_switches_setup_field() {
        let mut app = PomodoroApplication::new();
        let mut display = RecordingDisplay::new();
        let mut speaker = RecordingSpeaker::default();
        let mut store = MemoryPomodoroSettingsStore::new();
        let action = app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame {
                encoder: EncoderEvent {
                    switch_pressed: true,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            0,
        );
        assert_eq!(action, AppAction::None);
        assert_eq!(
            app.timer().active_field(),
            crate::pomodoro::PomodoroField::Seconds
        );
        assert!(display.commands().iter().all(|command| !matches!(
            command,
            crate::render::DrawCommand::Fill {
                rect: crate::render::Rect {
                    x: 0,
                    y: 0,
                    w: crate::config::TFT_H_RES,
                    h: crate::config::TFT_V_RES
                },
                ..
            }
        )));
    }

    #[test]
    fn encoder_and_joystick_adjust_duration() {
        let mut app = PomodoroApplication::new();
        let mut display = RecordingDisplay::new();
        let mut speaker = RecordingSpeaker::default();
        let mut store = MemoryPomodoroSettingsStore::new();
        app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame {
                encoder: EncoderEvent {
                    detents: 2,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            0,
        );
        assert_eq!(app.timer().minutes(), 27);
        app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame {
                joystick: JoystickEvent {
                    has_direction: true,
                    direction: Some(Direction::Down),
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            300_000,
        );
        assert_eq!(app.timer().minutes(), 26);
    }

    #[test]
    fn encoder_rotation_adjusts_selected_seconds_field() {
        let mut app = PomodoroApplication::new();
        let mut display = RecordingDisplay::new();
        let mut speaker = RecordingSpeaker::default();
        let mut store = MemoryPomodoroSettingsStore::new();
        app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame {
                encoder: EncoderEvent {
                    switch_pressed: true,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            0,
        );
        app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame {
                encoder: EncoderEvent {
                    detents: 15,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            100_000,
        );
        assert_eq!(app.timer().minutes(), 25);
        assert_eq!(app.timer().seconds(), 15);
    }

    #[test]
    fn zero_zero_start_is_disabled() {
        let mut app = PomodoroApplication::new();
        let mut display = RecordingDisplay::new();
        let mut speaker = RecordingSpeaker::default();
        let mut store = MemoryPomodoroSettingsStore::new();
        app.timer.adjust_active_field(-25);
        let action = app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            0,
        );
        assert_eq!(action, AppAction::None);
        assert_eq!(app.timer().mode(), PomodoroMode::Setup);
    }

    #[test]
    fn start_pause_resume_and_exit_flow() {
        let mut app = PomodoroApplication::new();
        let mut display = RecordingDisplay::new();
        let mut speaker = RecordingSpeaker::default();
        let mut store = MemoryPomodoroSettingsStore::new();
        app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            0,
        );
        assert_eq!(app.timer().mode(), PomodoroMode::Running);
        app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1_000_000,
        );
        assert_eq!(app.timer().mode(), PomodoroMode::Paused);
        app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame {
                encoder: EncoderEvent {
                    detents: 1,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            1_100_000,
        );
        assert_eq!(app.timer().pause_action(), PomodoroPauseAction::Exit);
        let action = app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1_200_000,
        );
        assert_eq!(action, AppAction::ExitToLauncher);
    }

    #[test]
    fn countdown_redraws_on_visible_second_change() {
        let mut app = PomodoroApplication::new();
        let mut display = RecordingDisplay::new();
        let mut speaker = RecordingSpeaker::default();
        let mut store = MemoryPomodoroSettingsStore::new();
        app.timer.adjust_active_field(-24);
        app.timer.start(0);
        assert_eq!(
            app.update(&mut display, &mut speaker, &mut store, InputFrame::default(), 500_000),
            AppAction::None
        );
        display.clear();
        assert_eq!(
            app.update(&mut display, &mut speaker, &mut store, InputFrame::default(), 1_000_000),
            AppAction::None
        );
        assert_eq!(app.timer().remaining_seconds(), 59);
        assert!(display.commands().iter().all(|command| !matches!(
            command,
            crate::render::DrawCommand::Fill {
                rect: crate::render::Rect {
                    x: 0,
                    y: 0,
                    w: crate::config::TFT_H_RES,
                    h: crate::config::TFT_V_RES
                },
                ..
            }
        )));
    }

    #[test]
    fn alert_plays_melody_dismisses_and_times_out() {
        let mut app = PomodoroApplication::new();
        let mut display = RecordingDisplay::new();
        let mut speaker = RecordingSpeaker::default();
        let mut store = MemoryPomodoroSettingsStore::new();
        app.timer.adjust_active_field(-25);
        app.timer.toggle_active_field();
        app.timer.adjust_active_field(1);
        app.timer.start(0);
        app.update(&mut display, &mut speaker, &mut store, InputFrame::default(), 1_000_000);
        assert_eq!(app.timer().mode(), PomodoroMode::Finished);
        assert_eq!(speaker.volume, 100);
        assert_eq!(speaker.tones[0], 1319);

        app.update(&mut display, &mut speaker, &mut store, InputFrame::default(), 1_200_000);
        assert!(speaker.tone_count > 1);

        app.update(&mut display, &mut speaker, &mut store, InputFrame::default(), 2_000_000);
        assert!(speaker.tone_count > 2);

        let action = app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame {
                encoder: EncoderEvent {
                    switch_pressed: true,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            2_100_000,
        );
        assert_eq!(action, AppAction::RedrawFull);
        assert_eq!(app.timer().mode(), PomodoroMode::Setup);

        app.timer.start(3_000_000);
        app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame::default(),
            3_001_000_000,
        );
        app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame::default(),
            3_011_000_000,
        );
        assert_eq!(app.timer().mode(), PomodoroMode::Finished);
        let stops_after_timeout = speaker.stops;
        app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame::default(),
            3_012_000_000,
        );
        assert_eq!(speaker.stops, stops_after_timeout);
    }

    #[test]
    fn long_press_exits_and_stops_alert() {
        let mut app = PomodoroApplication::new();
        let mut display = RecordingDisplay::new();
        let mut speaker = RecordingSpeaker::default();
        let mut store = MemoryPomodoroSettingsStore::new();
        app.timer.start(0);
        let action = app.update(
            &mut display,
            &mut speaker,
            &mut store,
            InputFrame {
                joystick: JoystickEvent {
                    switch_long_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            100,
        );
        assert_eq!(action, AppAction::ExitToLauncher);
    }

    #[test]
    fn enter_loads_persisted_settings() {
        let mut store = MemoryPomodoroSettingsStore::new();
        store.save_pomodoro_settings(30, 45);
        let mut app = PomodoroApplication::new();
        app.enter(&store);
        assert_eq!(app.timer().minutes(), 30);
        assert_eq!(app.timer().seconds(), 45);
    }

    #[test]
    fn exit_saves_persisted_settings() {
        let mut app = PomodoroApplication::new();
        let mut store = MemoryPomodoroSettingsStore::new();
        app.timer.adjust_active_field(5);
        let action = app.update(
            &mut RecordingDisplay::new(),
            &mut RecordingSpeaker::default(),
            &mut store,
            InputFrame {
                joystick: JoystickEvent {
                    switch_long_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            0,
        );
        assert_eq!(action, AppAction::ExitToLauncher);
        assert_eq!(store.pomodoro_minutes(), 30);
        assert_eq!(store.pomodoro_seconds(), 0);
    }
}
