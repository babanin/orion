#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PomodoroMode {
    Setup,
    Running,
    Paused,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PomodoroField {
    Minutes,
    Seconds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PomodoroPauseAction {
    Continue,
    Exit,
}

#[derive(Debug, Clone)]
pub struct PomodoroTimer {
    minutes: u8,
    seconds: u8,
    mode: PomodoroMode,
    active_field: PomodoroField,
    pause_action: PomodoroPauseAction,
    remaining_seconds: u16,
    end_us: i64,
}

impl PomodoroTimer {
    pub const fn new() -> Self {
        Self {
            minutes: 25,
            seconds: 0,
            mode: PomodoroMode::Setup,
            active_field: PomodoroField::Minutes,
            pause_action: PomodoroPauseAction::Continue,
            remaining_seconds: 25 * 60,
            end_us: 0,
        }
    }

    pub const fn mode(&self) -> PomodoroMode {
        self.mode
    }

    pub const fn active_field(&self) -> PomodoroField {
        self.active_field
    }

    pub const fn pause_action(&self) -> PomodoroPauseAction {
        self.pause_action
    }

    pub const fn minutes(&self) -> u8 {
        self.minutes
    }

    pub const fn seconds(&self) -> u8 {
        self.seconds
    }

    pub const fn duration_seconds(&self) -> u16 {
        self.minutes as u16 * 60 + self.seconds as u16
    }

    pub const fn remaining_seconds(&self) -> u16 {
        self.remaining_seconds
    }

    pub const fn can_start(&self) -> bool {
        self.duration_seconds() > 0
    }

    pub fn toggle_active_field(&mut self) {
        self.active_field = match self.active_field {
            PomodoroField::Minutes => PomodoroField::Seconds,
            PomodoroField::Seconds => PomodoroField::Minutes,
        };
    }

    pub fn adjust_active_field(&mut self, delta: i32) -> bool {
        match self.active_field {
            PomodoroField::Minutes => {
                let next = clamp_i32(self.minutes as i32 + delta, 0, 99) as u8;
                let changed = next != self.minutes;
                self.minutes = next;
                changed
            }
            PomodoroField::Seconds => {
                let next = clamp_i32(self.seconds as i32 + delta, 0, 59) as u8;
                let changed = next != self.seconds;
                self.seconds = next;
                changed
            }
        }
    }

    pub fn start(&mut self, now_us: i64) -> bool {
        if !self.can_start() {
            return false;
        }
        self.remaining_seconds = self.duration_seconds();
        self.end_us = now_us + self.remaining_seconds as i64 * 1_000_000;
        self.mode = PomodoroMode::Running;
        true
    }

    pub fn update_running(&mut self, now_us: i64) -> bool {
        if self.mode != PomodoroMode::Running {
            return false;
        }
        let previous_remaining = self.remaining_seconds;
        let previous_mode = self.mode;
        self.remaining_seconds = visible_remaining(self.end_us, now_us);
        if self.remaining_seconds == 0 {
            self.mode = PomodoroMode::Finished;
        }
        previous_remaining != self.remaining_seconds || previous_mode != self.mode
    }

    pub fn pause(&mut self, now_us: i64) -> bool {
        if self.mode != PomodoroMode::Running {
            return false;
        }
        self.update_running(now_us);
        if self.mode != PomodoroMode::Running {
            return false;
        }
        self.mode = PomodoroMode::Paused;
        self.pause_action = PomodoroPauseAction::Continue;
        true
    }

    pub fn resume(&mut self, now_us: i64) -> bool {
        if self.mode != PomodoroMode::Paused || self.remaining_seconds == 0 {
            return false;
        }
        self.end_us = now_us + self.remaining_seconds as i64 * 1_000_000;
        self.mode = PomodoroMode::Running;
        true
    }

    pub fn toggle_pause_action(&mut self) {
        self.pause_action = match self.pause_action {
            PomodoroPauseAction::Continue => PomodoroPauseAction::Exit,
            PomodoroPauseAction::Exit => PomodoroPauseAction::Continue,
        };
    }

    pub fn reset_to_setup(&mut self) {
        self.mode = PomodoroMode::Setup;
        self.active_field = PomodoroField::Minutes;
        self.pause_action = PomodoroPauseAction::Continue;
        self.remaining_seconds = self.duration_seconds();
        self.end_us = 0;
    }
}

impl Default for PomodoroTimer {
    fn default() -> Self {
        Self::new()
    }
}

fn visible_remaining(end_us: i64, now_us: i64) -> u16 {
    if now_us >= end_us {
        return 0;
    }
    ((end_us - now_us + 999_999) / 1_000_000) as u16
}

fn clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_twenty_five_minutes() {
        let timer = PomodoroTimer::new();
        assert_eq!(timer.mode(), PomodoroMode::Setup);
        assert_eq!(timer.minutes(), 25);
        assert_eq!(timer.seconds(), 0);
        assert_eq!(timer.duration_seconds(), 1500);
    }

    #[test]
    fn active_field_switches_and_adjusts_with_clamps() {
        let mut timer = PomodoroTimer::new();
        timer.adjust_active_field(100);
        assert_eq!(timer.minutes(), 99);
        timer.adjust_active_field(-120);
        assert_eq!(timer.minutes(), 0);
        timer.toggle_active_field();
        timer.adjust_active_field(80);
        assert_eq!(timer.seconds(), 59);
        timer.adjust_active_field(-80);
        assert_eq!(timer.seconds(), 0);
    }

    #[test]
    fn zero_duration_does_not_start() {
        let mut timer = PomodoroTimer::new();
        timer.adjust_active_field(-25);
        assert!(!timer.start(0));
        assert_eq!(timer.mode(), PomodoroMode::Setup);
    }

    #[test]
    fn running_countdown_uses_visible_seconds() {
        let mut timer = PomodoroTimer::new();
        timer.adjust_active_field(-24);
        assert!(timer.start(1_000));
        assert_eq!(timer.remaining_seconds(), 60);
        assert!(!timer.update_running(500_000));
        assert_eq!(timer.remaining_seconds(), 60);
        assert!(timer.update_running(1_001_000));
        assert_eq!(timer.remaining_seconds(), 59);
    }

    #[test]
    fn countdown_finishes_at_zero() {
        let mut timer = PomodoroTimer::new();
        timer.adjust_active_field(-25);
        timer.toggle_active_field();
        timer.adjust_active_field(1);
        assert!(timer.start(0));
        assert!(timer.update_running(1_000_000));
        assert_eq!(timer.remaining_seconds(), 0);
        assert_eq!(timer.mode(), PomodoroMode::Finished);
    }

    #[test]
    fn pause_and_resume_preserve_remaining_time() {
        let mut timer = PomodoroTimer::new();
        timer.adjust_active_field(-24);
        timer.start(0);
        assert!(timer.pause(1_000_000));
        assert_eq!(timer.remaining_seconds(), 59);
        assert_eq!(timer.mode(), PomodoroMode::Paused);
        assert!(timer.resume(10_000_000));
        assert_eq!(timer.mode(), PomodoroMode::Running);
        timer.update_running(11_000_000);
        assert_eq!(timer.remaining_seconds(), 58);
    }

    #[test]
    fn reset_to_setup_selects_minutes_field() {
        let mut timer = PomodoroTimer::new();
        timer.toggle_active_field();
        timer.start(0);
        timer.reset_to_setup();
        assert_eq!(timer.mode(), PomodoroMode::Setup);
        assert_eq!(timer.active_field(), PomodoroField::Minutes);
    }
}
