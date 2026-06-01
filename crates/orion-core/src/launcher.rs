use crate::config::wrap_index;
use crate::input::InputFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherAction {
    None,
    Redraw,
    Enter(usize),
}

#[derive(Debug, Clone)]
pub struct Launcher<const N: usize> {
    titles: [&'static str; N],
    selected: usize,
    last_direction_us: i64,
    direction_repeat_us: i64,
}

impl<const N: usize> Launcher<N> {
    pub const fn new(titles: [&'static str; N]) -> Self {
        Self {
            titles,
            selected: 0,
            last_direction_us: 0,
            direction_repeat_us: 250_000,
        }
    }

    pub fn update(&mut self, input: InputFrame, now_us: i64) -> LauncherAction {
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
            && matches!(
                input.joystick.direction,
                Some(crate::config::Direction::Up | crate::config::Direction::Down)
            )
        {
            self.cycle(1);
            self.last_direction_us = now_us;
            changed = true;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{EncoderEvent, InputFrame};

    #[test]
    fn wraps_selection_with_encoder() {
        let mut launcher = Launcher::new(["Flags", "Snake"]);
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
}
