use core::ffi::c_void;
use core::ptr;
use core::slice;

use esp_idf_sys as sys;
use orion_core::{theme, DisplaySink, DrawCommand, Rect};

use crate::hardware;

const SPI_TRANS_QUEUE_DEPTH: usize = 10;
const FILL_BUF_PIXELS: usize = hardware::TFT_H_RES as usize * hardware::TFT_DRAW_BUF_LINES;
const BITMAP_BUF_BYTES: usize = 160 * 104 * 2;
const FLAG_BYTES: &[u8] = include_bytes!("../../../main/flags.bin");

pub struct Display {
    io_handle: sys::esp_lcd_panel_io_handle_t,
    panel: sys::esp_lcd_panel_handle_t,
    fill_bufs: DmaBuffer<u16>,
    fill_buf_index: usize,
    bitmap_buf: DmaBuffer<u8>,
    bitmap_buf_busy: bool,
}

impl Display {
    pub const fn new() -> Self {
        Self {
            io_handle: ptr::null_mut(),
            panel: ptr::null_mut(),
            fill_bufs: DmaBuffer::new(),
            fill_buf_index: 0,
            bitmap_buf: DmaBuffer::new(),
            bitmap_buf_busy: false,
        }
    }

    pub fn init(&mut self) -> Result<(), sys::EspError> {
        self.fill_bufs
            .alloc(FILL_BUF_PIXELS * SPI_TRANS_QUEUE_DEPTH)?;
        self.bitmap_buf.alloc(BITMAP_BUF_BYTES)?;

        let bus_config = sys::spi_bus_config_t {
            __bindgen_anon_1: sys::spi_bus_config_t__bindgen_ty_1 {
                mosi_io_num: hardware::TFT_PIN_MOSI,
            },
            __bindgen_anon_2: sys::spi_bus_config_t__bindgen_ty_2 {
                miso_io_num: hardware::TFT_PIN_MISO,
            },
            sclk_io_num: hardware::TFT_PIN_SCLK,
            __bindgen_anon_3: sys::spi_bus_config_t__bindgen_ty_3 { quadwp_io_num: -1 },
            __bindgen_anon_4: sys::spi_bus_config_t__bindgen_ty_4 { quadhd_io_num: -1 },
            max_transfer_sz: (FILL_BUF_PIXELS.max(BITMAP_BUF_BYTES / 2) * 2) as i32,
            ..Default::default()
        };
        unsafe {
            sys::esp!(sys::spi_bus_initialize(
                hardware::TFT_SPI_HOST,
                &bus_config,
                sys::spi_common_dma_t_SPI_DMA_CH_AUTO
            ))?;
        }

        let io_config = sys::esp_lcd_panel_io_spi_config_t {
            cs_gpio_num: hardware::TFT_PIN_CS,
            dc_gpio_num: hardware::TFT_PIN_DC,
            spi_mode: 0,
            pclk_hz: hardware::TFT_PIXEL_CLOCK_HZ,
            trans_queue_depth: SPI_TRANS_QUEUE_DEPTH,
            lcd_cmd_bits: 8,
            lcd_param_bits: 8,
            ..Default::default()
        };
        unsafe {
            sys::esp!(sys::esp_lcd_new_panel_io_spi(
                hardware::TFT_SPI_HOST as sys::esp_lcd_spi_bus_handle_t,
                &io_config,
                &mut self.io_handle
            ))?;
        }

        let panel_config = sys::esp_lcd_panel_dev_config_t {
            reset_gpio_num: hardware::TFT_PIN_RST,
            __bindgen_anon_1: sys::esp_lcd_panel_dev_config_t__bindgen_ty_1 {
                rgb_ele_order: sys::lcd_rgb_element_order_t_LCD_RGB_ELEMENT_ORDER_RGB,
            },
            bits_per_pixel: 16,
            ..Default::default()
        };
        unsafe {
            sys::esp!(sys::esp_lcd_new_panel_st7789(
                self.io_handle,
                &panel_config,
                &mut self.panel
            ))?;
            sys::esp!(sys::esp_lcd_panel_reset(self.panel))?;
            sys::esp!(sys::esp_lcd_panel_init(self.panel))?;
            sys::esp!(sys::esp_lcd_panel_swap_xy(self.panel, true))?;
            sys::esp!(sys::esp_lcd_panel_mirror(self.panel, true, false))?;
            sys::esp!(sys::esp_lcd_panel_set_gap(
                self.panel,
                hardware::TFT_X_GAP,
                hardware::TFT_Y_GAP
            ))?;
            sys::esp!(sys::esp_lcd_panel_disp_on_off(self.panel, true))?;
        }

        if hardware::TFT_BL_ENABLED && hardware::TFT_PIN_BL >= 0 {
            let bl_config = sys::gpio_config_t {
                pin_bit_mask: 1_u64 << hardware::TFT_PIN_BL,
                mode: sys::gpio_mode_t_GPIO_MODE_OUTPUT,
                pull_up_en: sys::gpio_pullup_t_GPIO_PULLUP_DISABLE,
                pull_down_en: sys::gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
                intr_type: sys::gpio_int_type_t_GPIO_INTR_DISABLE,
            };
            unsafe {
                sys::esp!(sys::gpio_config(&bl_config))?;
                sys::esp!(sys::gpio_set_level(hardware::TFT_PIN_BL, 1))?;
            }
        }

        self.clear(theme::BG);
        self.flush();
        Ok(())
    }

    pub fn clear(&mut self, color: u16) {
        self.fill_rect(
            Rect {
                x: 0,
                y: 0,
                w: hardware::TFT_H_RES,
                h: hardware::TFT_V_RES,
            },
            color,
        );
    }

    fn fill_rect(&mut self, mut rect: Rect, color: u16) {
        if rect.x < 0 {
            rect.w += rect.x;
            rect.x = 0;
        }
        if rect.y < 0 {
            rect.h += rect.y;
            rect.y = 0;
        }
        if rect.x + rect.w > hardware::TFT_H_RES {
            rect.w = hardware::TFT_H_RES - rect.x;
        }
        if rect.y + rect.h > hardware::TFT_V_RES {
            rect.h = hardware::TFT_V_RES - rect.y;
        }
        if rect.w <= 0 || rect.h <= 0 || self.panel.is_null() {
            return;
        }

        let mut remaining = rect.h;
        let mut draw_y = rect.y;
        while remaining > 0 {
            let max_lines = 1.max(FILL_BUF_PIXELS as i16 / rect.w);
            let lines = remaining.min(max_lines);
            if self.fill_buf_index >= SPI_TRANS_QUEUE_DEPTH {
                self.flush();
            }
            let start = self.fill_buf_index * FILL_BUF_PIXELS;
            let fill_buf = &mut self.fill_bufs.as_mut_slice()[start..start + FILL_BUF_PIXELS];
            self.fill_buf_index += 1;
            fill_buf[..rect.w as usize * lines as usize].fill(color);
            unsafe {
                let _ = sys::esp_lcd_panel_draw_bitmap(
                    self.panel,
                    rect.x as i32,
                    draw_y as i32,
                    (rect.x + rect.w) as i32,
                    (draw_y + lines) as i32,
                    fill_buf.as_ptr() as *const c_void,
                );
            }
            draw_y += lines;
            remaining -= lines;
        }
    }

    fn draw_bitmap(&mut self, x: i16, y: i16, w: u16, h: u16, offset: u32) {
        let byte_count = w as usize * h as usize * 2;
        let offset = offset as usize;
        if byte_count == 0
            || byte_count > self.bitmap_buf.len()
            || offset
                .checked_add(byte_count)
                .map_or(true, |end| end > FLAG_BYTES.len())
            || self.panel.is_null()
        {
            return;
        }
        if self.bitmap_buf_busy {
            self.flush();
        }
        self.bitmap_buf.as_mut_slice()[..byte_count]
            .copy_from_slice(&FLAG_BYTES[offset..offset + byte_count]);
        self.bitmap_buf_busy = true;
        unsafe {
            let _ = sys::esp_lcd_panel_draw_bitmap(
                self.panel,
                x as i32,
                y as i32,
                x as i32 + w as i32,
                y as i32 + h as i32,
                self.bitmap_buf.as_ptr() as *const c_void,
            );
        }
    }

    pub fn flush(&mut self) {
        if !self.io_handle.is_null() {
            unsafe {
                let _ = sys::esp_lcd_panel_io_tx_param(self.io_handle, -1, ptr::null(), 0);
            }
        }
        self.fill_buf_index = 0;
        self.bitmap_buf_busy = false;
    }
}

impl DisplaySink for Display {
    fn push(&mut self, command: DrawCommand) {
        match command {
            DrawCommand::Fill { rect, color } => self.fill_rect(rect, color),
            DrawCommand::Bitmap { x, y, w, h, offset } => self.draw_bitmap(x, y, w, h, offset),
            DrawCommand::Flush => self.flush(),
        }
    }
}

impl Default for Display {
    fn default() -> Self {
        Self::new()
    }
}

struct DmaBuffer<T> {
    ptr: *mut T,
    len: usize,
}

impl<T> DmaBuffer<T> {
    const fn new() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
        }
    }

    fn alloc(&mut self, len: usize) -> Result<(), sys::EspError> {
        if !self.ptr.is_null() && self.len >= len {
            return Ok(());
        }
        if !self.ptr.is_null() {
            unsafe {
                sys::heap_caps_free(self.ptr.cast());
            }
        }
        let bytes = len * core::mem::size_of::<T>();
        let ptr =
            unsafe { sys::heap_caps_malloc(bytes, sys::MALLOC_CAP_DMA | sys::MALLOC_CAP_INTERNAL) }
                .cast::<T>();
        if ptr.is_null() {
            return Err(sys::EspError::from(sys::ESP_ERR_NO_MEM).unwrap());
        }
        self.ptr = ptr;
        self.len = len;
        Ok(())
    }

    fn len(&self) -> usize {
        self.len
    }

    fn as_ptr(&self) -> *const T {
        self.ptr
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T> Drop for DmaBuffer<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                sys::heap_caps_free(self.ptr.cast());
            }
        }
    }
}
