use esp_idf_sys as sys;

pub const TFT_SPI_HOST: sys::spi_host_device_t = sys::spi_host_device_t_SPI2_HOST;
pub const TFT_PIXEL_CLOCK_HZ: u32 = 40 * 1000 * 1000;
pub const TFT_H_RES: i16 = 320;
pub const TFT_V_RES: i16 = 240;
pub const TFT_DRAW_BUF_LINES: usize = 16;

pub const TFT_PIN_MOSI: sys::gpio_num_t = sys::gpio_num_t_GPIO_NUM_11;
pub const TFT_PIN_MISO: sys::gpio_num_t = sys::gpio_num_t_GPIO_NUM_13;
pub const TFT_PIN_SCLK: sys::gpio_num_t = sys::gpio_num_t_GPIO_NUM_12;
pub const TFT_PIN_CS: sys::gpio_num_t = sys::gpio_num_t_GPIO_NUM_10;
pub const TFT_PIN_DC: sys::gpio_num_t = sys::gpio_num_t_GPIO_NUM_9;
pub const TFT_PIN_RST: sys::gpio_num_t = sys::gpio_num_t_GPIO_NUM_8;
pub const TFT_PIN_BL: sys::gpio_num_t = sys::gpio_num_t_GPIO_NUM_NC;
pub const TFT_BL_ENABLED: bool = false;
pub const TFT_X_GAP: i32 = 0;
pub const TFT_Y_GAP: i32 = 0;

pub const JOY_ADC_UNIT: sys::adc_unit_t = sys::adc_unit_t_ADC_UNIT_1;
pub const JOY_X_ADC_CHANNEL: sys::adc_channel_t = sys::adc_channel_t_ADC_CHANNEL_0;
pub const JOY_Y_ADC_CHANNEL: sys::adc_channel_t = sys::adc_channel_t_ADC_CHANNEL_1;
pub const JOY_PIN_SW: sys::gpio_num_t = sys::gpio_num_t_GPIO_NUM_4;
pub const JOY_INVERT_X: bool = false;
pub const JOY_INVERT_Y: bool = false;
pub const JOYSTICK_CALIBRATION_SAMPLES: usize = 64;

pub const KY040_ENABLED: bool = true;
pub const KY040_PIN_CLK: sys::gpio_num_t = sys::gpio_num_t_GPIO_NUM_5;
pub const KY040_PIN_DT: sys::gpio_num_t = sys::gpio_num_t_GPIO_NUM_6;
pub const KY040_PIN_SW: sys::gpio_num_t = sys::gpio_num_t_GPIO_NUM_7;
