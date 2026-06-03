pub trait PomodoroSettingsStore {
    fn pomodoro_minutes(&self) -> u8;
    fn pomodoro_seconds(&self) -> u8;
    fn save_pomodoro_settings(&mut self, minutes: u8, seconds: u8);
}

#[derive(Debug, Clone)]
pub struct MemoryPomodoroSettingsStore {
    minutes: u8,
    seconds: u8,
}

impl MemoryPomodoroSettingsStore {
    pub const fn new() -> Self {
        Self {
            minutes: 25,
            seconds: 0,
        }
    }
}

impl Default for MemoryPomodoroSettingsStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PomodoroSettingsStore for MemoryPomodoroSettingsStore {
    fn pomodoro_minutes(&self) -> u8 {
        self.minutes
    }

    fn pomodoro_seconds(&self) -> u8 {
        self.seconds
    }

    fn save_pomodoro_settings(&mut self, minutes: u8, seconds: u8) {
        self.minutes = minutes;
        self.seconds = seconds;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_twenty_five_minutes() {
        let store = MemoryPomodoroSettingsStore::new();
        assert_eq!(store.pomodoro_minutes(), 25);
        assert_eq!(store.pomodoro_seconds(), 0);
    }

    #[test]
    fn save_and_load_round_trips() {
        let mut store = MemoryPomodoroSettingsStore::new();
        store.save_pomodoro_settings(30, 45);
        assert_eq!(store.pomodoro_minutes(), 30);
        assert_eq!(store.pomodoro_seconds(), 45);
    }
}