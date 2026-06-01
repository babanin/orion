use crate::config::{
    Direction, BUTTON_DEBOUNCE_MS, ENCODER_STEPS_PER_DETENT, JOYSTICK_DEADZONE, JOYSTICK_THRESHOLD,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ButtonEvent {
    pub pressed: bool,
    pub long_pressed: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JoystickEvent {
    pub has_direction: bool,
    pub direction: Option<Direction>,
    pub switch_pressed: bool,
    pub switch_long_pressed: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EncoderEvent {
    pub detents: i32,
    pub switch_pressed: bool,
    pub switch_long_pressed: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputFrame {
    pub joystick: JoystickEvent,
    pub encoder: EncoderEvent,
}

#[derive(Debug, Clone)]
pub struct DebouncedButton {
    stable_pressed: bool,
    last_raw_pressed: bool,
    last_change_ms: u64,
    press_start_ms: u64,
    long_reported: bool,
}

impl DebouncedButton {
    pub const fn new() -> Self {
        Self {
            stable_pressed: false,
            last_raw_pressed: false,
            last_change_ms: 0,
            press_start_ms: 0,
            long_reported: false,
        }
    }

    pub fn poll(&mut self, raw_pressed: bool, now_ms: u64, long_press_ms: u64) -> ButtonEvent {
        if raw_pressed != self.last_raw_pressed {
            self.last_raw_pressed = raw_pressed;
            self.last_change_ms = now_ms;
        }

        let mut event = ButtonEvent::default();
        if now_ms.saturating_sub(self.last_change_ms) >= BUTTON_DEBOUNCE_MS
            && self.stable_pressed != raw_pressed
        {
            self.stable_pressed = raw_pressed;
            if raw_pressed {
                self.press_start_ms = now_ms;
                self.long_reported = false;
                event.pressed = true;
            }
        }

        if self.stable_pressed
            && !self.long_reported
            && now_ms.saturating_sub(self.press_start_ms) >= long_press_ms
        {
            self.long_reported = true;
            event.long_pressed = true;
        }

        event
    }
}

impl Default for DebouncedButton {
    fn default() -> Self {
        Self::new()
    }
}

pub fn joystick_direction(
    raw_x: i32,
    raw_y: i32,
    center_x: i32,
    center_y: i32,
    invert_x: bool,
    invert_y: bool,
) -> Option<Direction> {
    let mut dx = raw_x - center_x;
    let mut dy = raw_y - center_y;
    if invert_x {
        dx = -dx;
    }
    if invert_y {
        dy = -dy;
    }

    let abs_x = dx.abs();
    let abs_y = dy.abs();
    if abs_x < JOYSTICK_DEADZONE && abs_y < JOYSTICK_DEADZONE {
        return None;
    }
    if abs_x >= abs_y && abs_x >= JOYSTICK_THRESHOLD {
        Some(if dx > 0 {
            Direction::Right
        } else {
            Direction::Left
        })
    } else if abs_y > abs_x && abs_y >= JOYSTICK_THRESHOLD {
        Some(if dy > 0 {
            Direction::Down
        } else {
            Direction::Up
        })
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct EncoderDecoder {
    last_encoded: u8,
    steps: i8,
}

impl EncoderDecoder {
    pub const fn new(initial_clk: bool, initial_dt: bool) -> Self {
        Self {
            last_encoded: ((initial_clk as u8) << 1) | initial_dt as u8,
            steps: 0,
        }
    }

    pub fn poll(&mut self, clk: bool, dt: bool) -> i32 {
        const MOVEMENT_TABLE: [i8; 16] = [0, -1, 1, 0, 1, 0, 0, -1, -1, 0, 0, 1, 0, 1, -1, 0];
        let encoded = ((clk as u8) << 1) | dt as u8;
        if encoded == self.last_encoded {
            return 0;
        }
        let movement = MOVEMENT_TABLE[((self.last_encoded << 2) | encoded) as usize];
        let mut detents = 0;
        if movement != 0 {
            self.steps += movement;
            if self.steps >= ENCODER_STEPS_PER_DETENT {
                detents = 1;
                self.steps = 0;
            } else if self.steps <= -ENCODER_STEPS_PER_DETENT {
                detents = -1;
                self.steps = 0;
            }
        } else {
            self.steps = 0;
        }
        self.last_encoded = encoded;
        detents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joystick_uses_dominant_axis_and_threshold() {
        assert_eq!(
            joystick_direction(2048, 2048, 2048, 2048, false, false),
            None
        );
        assert_eq!(
            joystick_direction(3000, 2200, 2048, 2048, false, false),
            Some(Direction::Right)
        );
        assert_eq!(
            joystick_direction(1800, 3100, 2048, 2048, false, false),
            Some(Direction::Down)
        );
        assert_eq!(
            joystick_direction(3000, 2200, 2048, 2048, true, false),
            Some(Direction::Left)
        );
    }

    #[test]
    fn button_debounces_press_and_reports_long_press_once() {
        let mut button = DebouncedButton::new();
        assert_eq!(button.poll(true, 10, 500), ButtonEvent::default());
        assert_eq!(
            button.poll(true, 90, 500),
            ButtonEvent {
                pressed: true,
                long_pressed: false
            }
        );
        assert_eq!(
            button.poll(true, 590, 500),
            ButtonEvent {
                pressed: false,
                long_pressed: true
            }
        );
        assert_eq!(button.poll(true, 700, 500), ButtonEvent::default());
    }

    #[test]
    fn encoder_counts_quadrature_detent() {
        let mut encoder = EncoderDecoder::new(false, false);
        let sequence = [(false, true), (true, true), (true, false), (false, false)];
        let detents: i32 = sequence
            .into_iter()
            .map(|(clk, dt)| encoder.poll(clk, dt))
            .sum();
        assert_eq!(detents, -1);
    }
}
