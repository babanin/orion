pub trait Speaker {
    fn play_tone(&mut self, freq_hz: u32);
    fn stop(&mut self);
    fn set_volume(&mut self, volume: u8);

    /// Non-blocking convenience: start a tone immediately.
    /// On firmware this blocks for `duration_ms` then stops;
    /// on host/test doubles this is a no-op.
    fn beep(&mut self, _freq_hz: u32, _duration_ms: u32) {}
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
