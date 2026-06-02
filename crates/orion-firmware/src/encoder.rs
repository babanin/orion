use esp_idf_sys as sys;
use orion_core::{
    EncoderDecoder, EncoderEvent, ENCODER_STEPS_PER_DETENT, FLAGS_PRACTICE_EXIT_HOLD_MS,
};
use std::ffi::c_void;
use std::sync::atomic::{AtomicI32, AtomicU8, Ordering};

use crate::hardware;

const KEY_DEBOUNCE_MS: u64 = 120;
const KEY_LOCKOUT_AFTER_ROTATION_MS: u64 = 250;
const MOVEMENT_TABLE: [i8; 16] = [0, -1, 1, 0, 1, 0, 0, -1, -1, 0, 0, 1, 0, 1, -1, 0];

static ENCODER_LAST: AtomicU8 = AtomicU8::new(0);
static ENCODER_STEPS: AtomicI32 = AtomicI32::new(0);
static ENCODER_DETENTS: AtomicI32 = AtomicI32::new(0);

pub struct Encoder {
    decoder: EncoderDecoder,
    interrupt_driven: bool,
    last_encoded: u8,
    last_key_reading: bool,
    stable_key_pressed: bool,
    ignore_key_until_release: bool,
    last_key_change_ms: u64,
    last_encoder_change_ms: u64,
    press_start_ms: u64,
    long_reported: bool,
}

impl Encoder {
    pub fn new() -> Self {
        Self {
            decoder: EncoderDecoder::new(false, false),
            interrupt_driven: false,
            last_encoded: 0,
            last_key_reading: false,
            stable_key_pressed: false,
            ignore_key_until_release: false,
            last_key_change_ms: 0,
            last_encoder_change_ms: 0,
            press_start_ms: 0,
            long_reported: false,
        }
    }

    pub fn init(&mut self) -> Result<(), sys::EspError> {
        if !hardware::KY040_ENABLED {
            return Ok(());
        }
        let encoder_pin_mask =
            (1_u64 << hardware::KY040_PIN_CLK) | (1_u64 << hardware::KY040_PIN_DT);
        let encoder_config = sys::gpio_config_t {
            pin_bit_mask: encoder_pin_mask,
            mode: sys::gpio_mode_t_GPIO_MODE_INPUT,
            pull_up_en: sys::gpio_pullup_t_GPIO_PULLUP_ENABLE,
            pull_down_en: sys::gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
            intr_type: sys::gpio_int_type_t_GPIO_INTR_ANYEDGE,
        };
        let switch_config = sys::gpio_config_t {
            pin_bit_mask: 1_u64 << hardware::KY040_PIN_SW,
            mode: sys::gpio_mode_t_GPIO_MODE_INPUT,
            pull_up_en: sys::gpio_pullup_t_GPIO_PULLUP_ENABLE,
            pull_down_en: sys::gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
            intr_type: sys::gpio_int_type_t_GPIO_INTR_DISABLE,
        };
        unsafe {
            sys::esp!(sys::gpio_config(&encoder_config))?;
            sys::esp!(sys::gpio_config(&switch_config))?;
        }
        let (clk, dt) = read_state();
        self.decoder = EncoderDecoder::new(clk, dt);
        self.last_encoded = encode_state(clk, dt);
        reset_interrupt_state(self.last_encoded);
        self.interrupt_driven = init_interrupts()?;
        let key_pressed = read_key_pressed();
        self.last_key_reading = key_pressed;
        self.stable_key_pressed = key_pressed;
        self.ignore_key_until_release = false;
        self.last_key_change_ms = 0;
        self.last_encoder_change_ms = 0;
        self.press_start_ms = 0;
        self.long_reported = false;
        Ok(())
    }

    pub fn poll(&mut self, now_us: i64) -> EncoderEvent {
        if !hardware::KY040_ENABLED {
            return EncoderEvent::default();
        }
        let now_ms = (now_us / 1000) as u64;
        let key_pressed = read_key_pressed();
        let (clk, dt) = read_state();
        let encoded = encode_state(clk, dt);
        if encoded != self.last_encoded {
            self.last_encoder_change_ms = now_ms;
            if key_pressed {
                self.ignore_key_until_release = true;
            }
            self.last_encoded = encoded;
        }
        let detents = if self.interrupt_driven {
            ENCODER_DETENTS.swap(0, Ordering::Relaxed)
        } else {
            self.decoder.poll(clk, dt)
        };
        if detents != 0 {
            self.last_encoder_change_ms = now_ms;
            if key_pressed {
                self.ignore_key_until_release = true;
            }
        }

        let (switch_pressed, switch_long_pressed) = self.poll_button(key_pressed, now_ms);
        EncoderEvent {
            detents,
            switch_pressed,
            switch_long_pressed,
        }
    }

    pub fn reset_button(&mut self) {
        self.last_key_reading = false;
        self.stable_key_pressed = false;
        self.ignore_key_until_release = false;
        self.last_key_change_ms = 0;
        self.press_start_ms = 0;
        self.long_reported = false;
    }

    fn poll_button(&mut self, key_pressed: bool, now_ms: u64) -> (bool, bool) {
        if key_pressed != self.last_key_reading {
            self.last_key_reading = key_pressed;
            self.last_key_change_ms = now_ms;
            if key_pressed
                && now_ms.saturating_sub(self.last_encoder_change_ms)
                    < KEY_LOCKOUT_AFTER_ROTATION_MS
            {
                self.ignore_key_until_release = true;
            }
        }

        if !key_pressed {
            self.ignore_key_until_release = false;
        }

        if now_ms.saturating_sub(self.last_encoder_change_ms) < KEY_LOCKOUT_AFTER_ROTATION_MS {
            self.last_key_change_ms = now_ms;
            return (false, false);
        }

        if self.ignore_key_until_release {
            return (false, false);
        }

        let mut switch_pressed = false;
        let mut switch_long_pressed = false;
        if key_pressed != self.stable_key_pressed
            && now_ms.saturating_sub(self.last_key_change_ms) >= KEY_DEBOUNCE_MS
        {
            self.stable_key_pressed = key_pressed;
            if key_pressed {
                self.press_start_ms = now_ms;
                self.long_reported = false;
                switch_pressed = true;
            }
        }

        if self.stable_key_pressed
            && !self.long_reported
            && now_ms.saturating_sub(self.press_start_ms) >= FLAGS_PRACTICE_EXIT_HOLD_MS
        {
            self.long_reported = true;
            switch_long_pressed = true;
        }

        (switch_pressed, switch_long_pressed)
    }
}

fn reset_interrupt_state(encoded: u8) {
    ENCODER_LAST.store(encoded, Ordering::Relaxed);
    ENCODER_STEPS.store(0, Ordering::Relaxed);
    ENCODER_DETENTS.store(0, Ordering::Relaxed);
}

fn init_interrupts() -> Result<bool, sys::EspError> {
    unsafe {
        let install_result = sys::gpio_install_isr_service(0);
        if install_result != sys::ESP_OK && install_result != sys::ESP_ERR_INVALID_STATE {
            return Err(sys::EspError::from(install_result).unwrap());
        }
        sys::esp!(sys::gpio_isr_handler_add(
            hardware::KY040_PIN_CLK,
            Some(encoder_gpio_isr),
            core::ptr::null_mut()
        ))?;
        sys::esp!(sys::gpio_isr_handler_add(
            hardware::KY040_PIN_DT,
            Some(encoder_gpio_isr),
            core::ptr::null_mut()
        ))?;
    }
    Ok(true)
}

extern "C" fn encoder_gpio_isr(_arg: *mut c_void) {
    let (clk, dt) = read_state();
    let encoded = encode_state(clk, dt);
    let previous = ENCODER_LAST.swap(encoded, Ordering::Relaxed);
    if encoded == previous {
        return;
    }

    let movement = MOVEMENT_TABLE[((previous << 2) | encoded) as usize] as i32;
    if movement == 0 {
        ENCODER_STEPS.store(0, Ordering::Relaxed);
        return;
    }

    let steps = ENCODER_STEPS.load(Ordering::Relaxed) + movement;
    let threshold = ENCODER_STEPS_PER_DETENT as i32;
    if steps >= threshold {
        ENCODER_DETENTS.fetch_add(1, Ordering::Relaxed);
        ENCODER_STEPS.store(0, Ordering::Relaxed);
    } else if steps <= -threshold {
        ENCODER_DETENTS.fetch_sub(1, Ordering::Relaxed);
        ENCODER_STEPS.store(0, Ordering::Relaxed);
    } else {
        ENCODER_STEPS.store(steps, Ordering::Relaxed);
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

fn encode_state(clk: bool, dt: bool) -> u8 {
    ((clk as u8) << 1) | dt as u8
}

fn read_key_pressed() -> bool {
    unsafe { sys::gpio_get_level(hardware::KY040_PIN_SW) == 0 }
}

impl Default for Encoder {
    fn default() -> Self {
        Self::new()
    }
}
