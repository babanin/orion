use esp_idf_sys as sys;

use crate::hardware;

pub struct Speaker {
    initialized: bool,
    timer: sys::ledc_timer_t,
    channel: sys::ledc_channel_t,
    volume: u8,
}

impl Speaker {
    pub fn new() -> Self {
        Self {
            initialized: false,
            timer: hardware::SPEAKER_LEDC_TIMER,
            channel: hardware::SPEAKER_LEDC_CHANNEL,
            volume: 100,
        }
    }

    pub fn init(&mut self) -> Result<(), sys::EspError> {
        let timer_config = sys::ledc_timer_config_t {
            speed_mode: sys::ledc_mode_t_LEDC_LOW_SPEED_MODE,
            duty_resolution: sys::ledc_timer_bit_t_LEDC_TIMER_8_BIT,
            timer_num: self.timer,
            freq_hz: 1000,
            clk_cfg: sys::soc_periph_ledc_clk_src_legacy_t_LEDC_AUTO_CLK,
            deconfigure: false,
        };
        unsafe {
            sys::esp!(sys::ledc_timer_config(&timer_config))?;
        }

        let channel_config = sys::ledc_channel_config_t {
            gpio_num: hardware::SPEAKER_PIN as ::core::ffi::c_int,
            speed_mode: sys::ledc_mode_t_LEDC_LOW_SPEED_MODE,
            channel: self.channel,
            intr_type: sys::ledc_intr_type_t_LEDC_INTR_DISABLE,
            timer_sel: self.timer,
            duty: 0,
            hpoint: 0,
            flags: Default::default(),
        };
        unsafe {
            sys::esp!(sys::ledc_channel_config(&channel_config))?;
        }

        self.initialized = true;
        Ok(())
    }

    pub fn play_tone(&mut self, freq_hz: u32) {
        if !self.initialized {
            return;
        }
        let duty = (self.volume as u32) * 255 / 100;
        unsafe {
            sys::ledc_set_freq(
                sys::ledc_mode_t_LEDC_LOW_SPEED_MODE,
                self.timer,
                freq_hz,
            );
            sys::ledc_set_duty(
                sys::ledc_mode_t_LEDC_LOW_SPEED_MODE,
                self.channel,
                duty,
            );
            sys::ledc_update_duty(
                sys::ledc_mode_t_LEDC_LOW_SPEED_MODE,
                self.channel,
            );
        }
    }

    pub fn stop(&mut self) {
        if !self.initialized {
            return;
        }
        unsafe {
            sys::ledc_stop(
                sys::ledc_mode_t_LEDC_LOW_SPEED_MODE,
                self.channel,
                0,
            );
        }
    }

    pub fn beep(&mut self, freq_hz: u32, duration_ms: u32) {
        self.play_tone(freq_hz);
        unsafe {
            let ticks = ((duration_ms * sys::configTICK_RATE_HZ) + 999) / 1000;
            sys::vTaskDelay(ticks.max(1));
        }
        self.stop();
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume.min(100);
    }
}

impl orion_core::Speaker for Speaker {
    fn play_tone(&mut self, freq_hz: u32) {
        self.play_tone(freq_hz);
    }

    fn stop(&mut self) {
        self.stop();
    }

    fn set_volume(&mut self, volume: u8) {
        self.set_volume(volume);
    }

    fn beep(&mut self, freq_hz: u32, duration_ms: u32) {
        self.beep(freq_hz, duration_ms);
    }
}

impl Default for Speaker {
    fn default() -> Self {
        Self::new()
    }
}
