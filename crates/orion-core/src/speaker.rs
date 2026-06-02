pub trait Speaker {
    fn play_tone(&mut self, freq_hz: u32);
    fn stop(&mut self);
    fn set_volume(&mut self, volume: u8);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SilentSpeaker;

impl Speaker for SilentSpeaker {
    fn play_tone(&mut self, _freq_hz: u32) {}
    fn stop(&mut self) {}
    fn set_volume(&mut self, _volume: u8) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_speaker_is_no_op() {
        let mut s = SilentSpeaker;
        s.play_tone(440);
        s.set_volume(200);
        s.stop();
    }
}
