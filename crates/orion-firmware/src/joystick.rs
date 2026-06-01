use core::ptr;

use esp_idf_sys as sys;
use orion_core::{joystick_direction, DebouncedButton, JoystickEvent, FLAGS_PRACTICE_EXIT_HOLD_MS};

use crate::hardware;

pub struct Joystick {
    adc: sys::adc_oneshot_unit_handle_t,
    center_x: i32,
    center_y: i32,
    button: DebouncedButton,
}

impl Joystick {
    pub const fn new() -> Self {
        Self {
            adc: ptr::null_mut(),
            center_x: 0,
            center_y: 0,
            button: DebouncedButton::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), sys::EspError> {
        let unit_config = sys::adc_oneshot_unit_init_cfg_t {
            unit_id: hardware::JOY_ADC_UNIT,
            ..Default::default()
        };
        unsafe {
            sys::esp!(sys::adc_oneshot_new_unit(&unit_config, &mut self.adc))?;
        }

        let channel_config = sys::adc_oneshot_chan_cfg_t {
            atten: sys::adc_atten_t_ADC_ATTEN_DB_12,
            bitwidth: sys::adc_bitwidth_t_ADC_BITWIDTH_DEFAULT,
        };
        unsafe {
            sys::esp!(sys::adc_oneshot_config_channel(
                self.adc,
                hardware::JOY_X_ADC_CHANNEL,
                &channel_config
            ))?;
            sys::esp!(sys::adc_oneshot_config_channel(
                self.adc,
                hardware::JOY_Y_ADC_CHANNEL,
                &channel_config
            ))?;
        }

        let switch_config = sys::gpio_config_t {
            pin_bit_mask: 1_u64 << hardware::JOY_PIN_SW,
            mode: sys::gpio_mode_t_GPIO_MODE_INPUT,
            pull_up_en: sys::gpio_pullup_t_GPIO_PULLUP_ENABLE,
            pull_down_en: sys::gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
            intr_type: sys::gpio_int_type_t_GPIO_INTR_DISABLE,
        };
        unsafe {
            sys::esp!(sys::gpio_config(&switch_config))?;
        }
        self.calibrate_center();
        Ok(())
    }

    pub fn poll(&mut self, now_us: i64) -> JoystickEvent {
        let raw_pressed = unsafe { sys::gpio_get_level(hardware::JOY_PIN_SW) == 0 };
        let button = self.button.poll(
            raw_pressed,
            (now_us / 1000) as u64,
            FLAGS_PRACTICE_EXIT_HOLD_MS,
        );

        let mut raw_x = 0;
        let mut raw_y = 0;
        unsafe {
            let _ = sys::adc_oneshot_read(self.adc, hardware::JOY_X_ADC_CHANNEL, &mut raw_x);
            let _ = sys::adc_oneshot_read(self.adc, hardware::JOY_Y_ADC_CHANNEL, &mut raw_y);
        }
        let direction = joystick_direction(
            raw_x,
            raw_y,
            self.center_x,
            self.center_y,
            hardware::JOY_INVERT_X,
            hardware::JOY_INVERT_Y,
        );

        JoystickEvent {
            has_direction: direction.is_some(),
            direction,
            switch_pressed: button.pressed,
            switch_long_pressed: button.long_pressed,
        }
    }

    fn calibrate_center(&mut self) {
        let mut sum_x = 0;
        let mut sum_y = 0;
        for _ in 0..hardware::JOYSTICK_CALIBRATION_SAMPLES {
            let mut x = 0;
            let mut y = 0;
            unsafe {
                let _ = sys::adc_oneshot_read(self.adc, hardware::JOY_X_ADC_CHANNEL, &mut x);
                let _ = sys::adc_oneshot_read(self.adc, hardware::JOY_Y_ADC_CHANNEL, &mut y);
                sys::esp_rom_delay_us(1000);
            }
            sum_x += x;
            sum_y += y;
        }
        self.center_x = sum_x / hardware::JOYSTICK_CALIBRATION_SAMPLES as i32;
        self.center_y = sum_y / hardware::JOYSTICK_CALIBRATION_SAMPLES as i32;
    }
}
