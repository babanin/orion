use esp_idf_sys as sys;
use orion_core::{DebouncedButton, EncoderDecoder, EncoderEvent, FLAGS_PRACTICE_EXIT_HOLD_MS};

use crate::hardware;

pub struct Encoder {
    decoder: EncoderDecoder,
    button: DebouncedButton,
}

impl Encoder {
    pub fn new() -> Self {
        Self {
            decoder: EncoderDecoder::new(false, false),
            button: DebouncedButton::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), sys::EspError> {
        if !hardware::KY040_ENABLED {
            return Ok(());
        }
        let pin_mask = (1_u64 << hardware::KY040_PIN_CLK)
            | (1_u64 << hardware::KY040_PIN_DT)
            | (1_u64 << hardware::KY040_PIN_SW);
        let config = sys::gpio_config_t {
            pin_bit_mask: pin_mask,
            mode: sys::gpio_mode_t_GPIO_MODE_INPUT,
            pull_up_en: sys::gpio_pullup_t_GPIO_PULLUP_ENABLE,
            pull_down_en: sys::gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
            intr_type: sys::gpio_int_type_t_GPIO_INTR_DISABLE,
        };
        unsafe {
            sys::esp!(sys::gpio_config(&config))?;
        }
        let (clk, dt) = read_state();
        self.decoder = EncoderDecoder::new(clk, dt);
        Ok(())
    }

    pub fn poll(&mut self, now_us: i64) -> EncoderEvent {
        if !hardware::KY040_ENABLED {
            return EncoderEvent::default();
        }
        let raw_pressed = unsafe { sys::gpio_get_level(hardware::KY040_PIN_SW) == 0 };
        let button = self.button.poll(
            raw_pressed,
            (now_us / 1000) as u64,
            FLAGS_PRACTICE_EXIT_HOLD_MS,
        );
        let (clk, dt) = read_state();
        EncoderEvent {
            detents: self.decoder.poll(clk, dt),
            switch_pressed: button.pressed,
            switch_long_pressed: button.long_pressed,
        }
    }
    pub fn reset_button(&mut self) {
        self.button.reset();
    }
}

fn read_state() -> (bool, bool) {
    unsafe {
        (
            sys::gpio_get_level(hardware::KY040_PIN_CLK) != 0,
            sys::gpio_get_level(hardware::KY040_PIN_DT) != 0,
        )
    }
}

impl Default for Encoder {
    fn default() -> Self {
        Self::new()
    }
}
