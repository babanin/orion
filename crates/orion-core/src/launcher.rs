use crate::config::wrap_index;
use crate::input::InputFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherView {
    Home,
    GameMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherAction {
    None,
    Redraw,
    Enter(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockTime {
    pub hour: u8,
    pub minute: u8,
}

impl ClockTime {
    pub const fn new(hour: u8, minute: u8) -> Self {
        Self { hour, minute }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl CalendarDate {
    pub const fn new(year: u16, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeStatus {
    Ready,
    Wifi,
    Time,
    Weather,
}

impl HomeStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ONLINE",
            Self::Wifi => "WIFI",
            Self::Time => "TIME",
            Self::Weather => "WEATHER",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomeSnapshot {
    pub time: Option<ClockTime>,
    pub date: Option<CalendarDate>,
    pub temperature_tenths_c: Option<i16>,
    pub status: HomeStatus,
}

impl HomeSnapshot {
    pub const fn placeholders() -> Self {
        Self {
            time: None,
            date: None,
            temperature_tenths_c: None,
            status: HomeStatus::Wifi,
        }
    }
}

impl Default for HomeSnapshot {
    fn default() -> Self {
        Self::placeholders()
    }
}

#[derive(Debug, Clone)]
pub struct Launcher<const N: usize> {
    titles: [&'static str; N],
    selected: usize,
    view: LauncherView,
    last_direction_us: i64,
    direction_repeat_us: i64,
}

impl<const N: usize> Launcher<N> {
    pub const fn new(titles: [&'static str; N]) -> Self {
        Self {
            titles,
            selected: 0,
            view: LauncherView::Home,
            last_direction_us: 0,
            direction_repeat_us: 250_000,
        }
    }

    pub fn update(&mut self, input: InputFrame, now_us: i64) -> LauncherAction {
        if self.view == LauncherView::Home {
            if input.joystick.switch_pressed || input.encoder.switch_pressed {
                self.view = LauncherView::GameMenu;
                return LauncherAction::Redraw;
            }
            return LauncherAction::None;
        }

        if input.joystick.switch_long_pressed || input.encoder.switch_long_pressed {
            self.view = LauncherView::Home;
            return LauncherAction::Redraw;
        }

        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            return LauncherAction::Enter(self.selected);
        }

        let mut changed = false;
        if input.encoder.detents != 0 {
            self.cycle(input.encoder.detents);
            changed = true;
        }

        if input.joystick.has_direction
            && now_us - self.last_direction_us >= self.direction_repeat_us
        {
            match input.joystick.direction {
                Some(crate::config::Direction::Up) => {
                    self.cycle(-1);
                    self.last_direction_us = now_us;
                    changed = true;
                }
                Some(crate::config::Direction::Down) => {
                    self.cycle(1);
                    self.last_direction_us = now_us;
                    changed = true;
                }
                _ => {}
            }
        }

        if changed {
            LauncherAction::Redraw
        } else {
            LauncherAction::None
        }
    }

    pub fn cycle(&mut self, detents: i32) {
        self.selected = wrap_index(self.selected as i32 + detents, N as i32) as usize;
    }

    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    pub const fn selected_title(&self) -> &'static str {
        self.titles[self.selected]
    }

    pub const fn view(&self) -> LauncherView {
        self.view
    }

    pub fn show_home(&mut self) {
        self.view = LauncherView::Home;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{EncoderEvent, InputFrame, JoystickEvent};

    #[test]
    fn wraps_selection_with_encoder() {
        let mut launcher = Launcher::new(["Flags", "Snake"]);
        launcher.update(
            InputFrame {
                encoder: EncoderEvent {
                    switch_pressed: true,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        launcher.update(
            InputFrame {
                encoder: EncoderEvent {
                    detents: -1,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        assert_eq!(launcher.selected_title(), "Snake");
    }

    #[test]
    fn switch_enters_selected_app() {
        let mut launcher = Launcher::new(["Flags", "Snake"]);
        launcher.update(
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        launcher.cycle(1);
        let action = launcher.update(
            InputFrame {
                encoder: EncoderEvent {
                    switch_pressed: true,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        assert_eq!(action, LauncherAction::Enter(1));
    }

    #[test]
    fn switch_on_home_opens_game_menu() {
        let mut launcher = Launcher::new(["Flags", "Snake"]);
        let action = launcher.update(
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        assert_eq!(action, LauncherAction::Redraw);
        assert_eq!(launcher.view(), LauncherView::GameMenu);
    }

    #[test]
    fn long_switch_in_game_menu_returns_home() {
        let mut launcher = Launcher::new(["Flags", "Snake"]);
        launcher.update(
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            1,
        );
        let action = launcher.update(
            InputFrame {
                encoder: EncoderEvent {
                    switch_long_pressed: true,
                    ..EncoderEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(action, LauncherAction::Redraw);
        assert_eq!(launcher.view(), LauncherView::Home);
    }
}
