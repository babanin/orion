use crate::speaker::Speaker;

#[derive(Debug, Clone, Copy)]
pub struct Note {
    pub freq_hz: u32,
    pub duration_us: i64,
}

impl Note {
    pub const fn rest(duration_us: i64) -> Self {
        Self {
            freq_hz: 0,
            duration_us,
        }
    }

    pub const fn tone(freq_hz: u32, duration_us: i64) -> Self {
        Self {
            freq_hz,
            duration_us,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MelodyPlayer {
    notes: &'static [Note],
    index: usize,
    note_started_us: i64,
    active: bool,
}

impl MelodyPlayer {
    pub const fn new() -> Self {
        Self {
            notes: &[],
            index: 0,
            note_started_us: 0,
            active: false,
        }
    }

    pub fn start(&mut self, notes: &'static [Note], speaker: &mut impl Speaker, now_us: i64) {
        if notes.is_empty() {
            return;
        }
        self.notes = notes;
        self.index = 0;
        self.note_started_us = now_us;
        self.active = true;
        play_note(notes, 0, speaker);
    }

    pub fn update(&mut self, speaker: &mut impl Speaker, now_us: i64) {
        if !self.active || self.notes.is_empty() {
            return;
        }
        if now_us - self.note_started_us >= self.notes[self.index].duration_us {
            self.index += 1;
            if self.index >= self.notes.len() {
                self.index = 0;
            }
            self.note_started_us = now_us;
            play_note(self.notes, self.index, speaker);
        }
    }

    pub fn stop(&mut self, speaker: &mut impl Speaker) {
        if self.active {
            speaker.stop();
        }
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for MelodyPlayer {
    fn default() -> Self {
        Self::new()
    }
}

fn play_note(notes: &[Note], index: usize, speaker: &mut impl Speaker) {
    let note = &notes[index];
    if note.freq_hz == 0 {
        speaker.stop();
    } else {
        speaker.play_tone(note.freq_hz);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingSpeaker {
        tones: Vec<u32>,
        stops: usize,
    }

    impl Speaker for RecordingSpeaker {
        fn play_tone(&mut self, freq_hz: u32) {
            self.tones.push(freq_hz);
        }

        fn stop(&mut self) {
            self.stops += 1;
        }

        fn set_volume(&mut self, _volume: u8) {}
    }

    static SIMPLE_MELODY: [Note; 3] = [
        Note::tone(440, 100_000),
        Note::tone(880, 200_000),
        Note::rest(100_000),
    ];

    #[test]
    fn melody_plays_notes_in_sequence() {
        let mut player = MelodyPlayer::new();
        let mut speaker = RecordingSpeaker::default();

        player.start(&SIMPLE_MELODY, &mut speaker, 0);
        assert_eq!(speaker.tones, [440]);
        assert!(player.is_active());

        player.update(&mut speaker, 50_000);
        assert_eq!(speaker.tones.len(), 1);

        player.update(&mut speaker, 100_000);
        assert_eq!(speaker.tones, [440, 880]);

        player.update(&mut speaker, 300_000);
        assert_eq!(speaker.stops, 1);

        player.update(&mut speaker, 400_000);
        assert_eq!(speaker.tones.len(), 3);
        assert_eq!(speaker.tones[2], 440);
    }

    #[test]
    fn melody_stops() {
        let mut player = MelodyPlayer::new();
        let mut speaker = RecordingSpeaker::default();

        player.start(&SIMPLE_MELODY, &mut speaker, 0);
        assert!(player.is_active());

        player.stop(&mut speaker);
        assert!(!player.is_active());
        assert_eq!(speaker.stops, 1);

        player.update(&mut speaker, 200_000);
        assert_eq!(speaker.tones.len(), 1);
    }

    #[test]
    fn empty_melody_is_no_op() {
        let mut player = MelodyPlayer::new();
        let mut speaker = RecordingSpeaker::default();

        player.start(&[], &mut speaker, 0);
        assert!(!player.is_active());
        assert_eq!(speaker.tones.len(), 0);
    }
}
