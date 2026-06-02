use crate::config::{wrap_index, Direction, MENU_COLS};
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
    GoHome,
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
            return LauncherAction::GoHome;
        }

        if input.joystick.switch_pressed || input.encoder.switch_pressed {
            if self.selected == N - 1 {
                self.view = LauncherView::Home;
                return LauncherAction::GoHome;
            }
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
                Some(Direction::Up) => {
                    self.move_up();
                    self.last_direction_us = now_us;
                    changed = true;
                }
                Some(Direction::Down) => {
                    self.move_down();
                    self.last_direction_us = now_us;
                    changed = true;
                }
                Some(Direction::Left) => {
                    self.move_left();
                    self.last_direction_us = now_us;
                    changed = true;
                }
                Some(Direction::Right) => {
                    self.move_right();
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

    fn move_up(&mut self) {
        let col = self.selected % MENU_COLS;
        if self.selected >= MENU_COLS {
            self.selected -= MENU_COLS;
        } else {
            let bottom = column_bottom(col, N);
            self.selected = bottom;
        }
    }

    fn move_down(&mut self) {
        let col = self.selected % MENU_COLS;
        let bottom = column_bottom(col, N);
        if self.selected < bottom {
            self.selected += MENU_COLS;
        } else {
            self.selected = col;
        }
    }

    fn move_left(&mut self) {
        if self.selected % MENU_COLS == 0 {
            let target = self.selected + MENU_COLS - 1;
            if target < N {
                self.selected = target;
            }
        } else {
            self.selected -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.selected % MENU_COLS == MENU_COLS - 1 {
            let target = self.selected + 1 - MENU_COLS;
            if target < N {
                self.selected = target;
            }
        } else {
            let target = self.selected + 1;
            if target < N {
                self.selected = target;
            }
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

    pub fn show_game_menu(&mut self) {
        self.view = LauncherView::GameMenu;
    }
}

fn column_bottom(col: usize, count: usize) -> usize {
    let rows = (count + MENU_COLS - 1) / MENU_COLS;
    let row = rows - 1;
    let idx = row * MENU_COLS + col;
    if idx < count { idx } else { idx - MENU_COLS }
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
        let mut launcher = Launcher::new(["Flags", "Snake", "Tetris", "HOME"]);
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
    fn home_item_returns_go_home() {
        let mut launcher = Launcher::new(["Flags", "Snake", "HOME"]);
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
        launcher.cycle(2);
        let action = launcher.update(
            InputFrame {
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(action, LauncherAction::GoHome);
        assert_eq!(launcher.view(), LauncherView::Home);
    }

    #[test]
    fn game_item_returns_enter() {
        let mut launcher = Launcher::new(["Flags", "Snake", "HOME"]);
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
                joystick: JoystickEvent {
                    switch_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(action, LauncherAction::Enter(0));
    }

    #[test]
    fn long_press_joystick_switch_returns_go_home() {
        let mut launcher = Launcher::new(["Flags", "Snake", "2048", "Tetris", "HOME"]);
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
                joystick: JoystickEvent {
                    switch_long_pressed: true,
                    ..JoystickEvent::default()
                },
                ..InputFrame::default()
            },
            2,
        );
        assert_eq!(action, LauncherAction::GoHome);
    }

    #[test]
    fn long_press_encoder_switch_returns_go_home() {
        let mut launcher = Launcher::new(["Flags", "Snake", "2048", "Tetris", "HOME"]);
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
        assert_eq!(action, LauncherAction::GoHome);
    }

    #[test]
    fn move_right_goes_to_next_column() {
        let mut launcher = Launcher::new(["FLAGS", "SNAKE", "2048", "TETRIS", "HOME"]);
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
        launcher.move_right();
        assert_eq!(launcher.selected_index(), 1);
    }

    #[test]
    fn move_right_wraps_from_rightmost_to_leftmost() {
        let mut launcher = Launcher::new(["FLAGS", "SNAKE", "2048", "TETRIS", "HOME"]);
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
        let input = InputFrame {
            joystick: JoystickEvent {
                has_direction: true,
                direction: Some(Direction::Right),
                ..JoystickEvent::default()
            },
            ..InputFrame::default()
        };
        launcher.selected = 1;
        launcher.last_direction_us = 0;
        let action = launcher.update(input, 500_000);
        assert_eq!(launcher.selected_index(), 0);
        assert_eq!(action, LauncherAction::Redraw);
    }

    #[test]
    fn move_left_wraps_from_leftmost_to_rightmost() {
        let mut launcher = Launcher::new(["FLAGS", "SNAKE", "2048", "TETRIS", "HOME"]);
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
        launcher.selected = 0;
        launcher.move_left();
        assert_eq!(launcher.selected_index(), 1);
    }

    #[test]
    fn move_left_clamps_on_odd_count_empty_cell() {
        let mut launcher = Launcher::new(["A", "B", "C"]);
        launcher.selected = 2;
        launcher.move_left();
        assert_eq!(launcher.selected_index(), 2);
    }

    #[test]
    fn move_right_clamps_on_odd_count_empty_cell() {
        let mut launcher = Launcher::new(["A", "B", "C"]);
        launcher.selected = 2;
        launcher.move_right();
        assert_eq!(launcher.selected_index(), 2);
    }

    #[test]
    fn move_down_goes_same_column() {
        let mut l = Launcher::new(["FLAGS", "SNAKE", "2048", "TETRIS", "HOME"]);
        l.selected = 0;
        l.move_down();
        assert_eq!(l.selected_index(), 2);
    }

    #[test]
    fn move_up_wraps_within_column() {
        let mut l = Launcher::new(["FLAGS", "SNAKE", "2048", "TETRIS", "HOME"]);
        l.selected = 0;
        l.move_up();
        assert_eq!(l.selected_index(), 4);
    }

    #[test]
    fn move_down_wraps_within_column() {
        let mut l = Launcher::new(["FLAGS", "SNAKE", "2048", "TETRIS", "HOME"]);
        l.selected = 4;
        l.move_down();
        assert_eq!(l.selected_index(), 0);
    }

    #[test]
    fn column_bottom_for_odd_count() {
        assert_eq!(column_bottom(0, 5), 4);
        assert_eq!(column_bottom(1, 5), 3);
    }

    #[test]
    fn column_bottom_for_even_count() {
        assert_eq!(column_bottom(0, 4), 2);
        assert_eq!(column_bottom(1, 4), 3);
    }
}
