use core::fmt::{self, Write};

use crate::render::{fill_rect, DisplaySink};

pub struct TextBuffer<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> TextBuffer<N> {
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

impl<const N: usize> Write for TextBuffer<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let remaining = N.saturating_sub(self.len);
        let bytes = s.as_bytes();
        let copy_len = remaining.min(bytes.len());
        self.bytes[self.len..self.len + copy_len].copy_from_slice(&bytes[..copy_len]);
        self.len += copy_len;
        if copy_len == bytes.len() {
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

pub fn draw_char(display: &mut impl DisplaySink, x: i16, y: i16, c: char, color: u16, scale: i16) {
    let scale = if scale <= 1 { 1 } else { 2 };
    let cols = font_columns(c);
    for (col, bits) in cols.iter().enumerate() {
        for row in 0..7 {
            if bits & (1 << row) != 0 {
                fill_rect(
                    display,
                    x + col as i16 * scale,
                    y + row as i16 * scale,
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
}

pub fn draw_text(
    display: &mut impl DisplaySink,
    mut x: i16,
    y: i16,
    text: &str,
    color: u16,
    scale: i16,
) {
    let step = 6 * if scale <= 1 { 1 } else { 2 };
    for c in text.chars() {
        draw_char(display, x, y, c, color, scale);
        x += step;
    }
}

pub fn draw_text_limited(
    display: &mut impl DisplaySink,
    mut x: i16,
    y: i16,
    text: &str,
    color: u16,
    scale: i16,
    max_chars: usize,
) {
    let step = 6 * if scale <= 1 { 1 } else { 2 };
    for c in text.chars().take(max_chars) {
        draw_char(display, x, y, c, color, scale);
        x += step;
    }
}

pub fn draw_text_aa(
    display: &mut impl DisplaySink,
    mut x: i16,
    y: i16,
    text: &str,
    fg: u16,
    bg: u16,
    scale: i16,
) {
    let char_w = 5 * scale;
    let gap = scale;
    for c in text.chars() {
        draw_char_aa(display, x, y, c, fg, bg, scale);
        x += char_w + gap;
    }
}

pub fn draw_centered_text_aa(
    display: &mut impl DisplaySink,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
    text: &str,
    fg: u16,
    bg: u16,
    scale: i16,
) {
    let len = text.chars().count() as i16;
    let char_w = 5 * scale;
    let gap = scale;
    let text_width = len * char_w + (len - 1).max(0) * gap;
    let text_height = 7 * scale;
    let start_x = x + 0.max((w - text_width) / 2);
    let start_y = y + 0.max((h - text_height) / 2);
    draw_text_aa(display, start_x, start_y, text, fg, bg, scale);
}

pub fn draw_wrapped_text(
    display: &mut impl DisplaySink,
    x: i16,
    mut y: i16,
    text: &str,
    color: u16,
    scale: i16,
    max_chars_per_line: usize,
    max_lines: usize,
) {
    let mut cursor = text;
    for _ in 0..max_lines {
        cursor = cursor.trim_start_matches(' ');
        if cursor.is_empty() {
            break;
        }

        let mut len = 0;
        let mut last_space = None;
        for (index, c) in cursor.char_indices() {
            if len >= max_chars_per_line {
                break;
            }
            if c == ' ' {
                last_space = Some(index);
            }
            len += 1;
        }

        let mut byte_len = cursor.len();
        let mut char_count = 0;
        for (index, _) in cursor.char_indices().take(max_chars_per_line) {
            byte_len = index;
            char_count += 1;
        }
        if char_count < max_chars_per_line {
            byte_len = cursor.len();
        } else if let Some(space) = last_space {
            if space > 0 {
                byte_len = space;
            }
        }

        let line = &cursor[..byte_len];
        draw_text_limited(display, x, y, line, color, scale, max_chars_per_line);
        cursor = &cursor[byte_len..];
        y += 10 * if scale <= 1 { 1 } else { 2 };
    }
}

pub fn draw_centered_text(
    display: &mut impl DisplaySink,
    x: i16,
    y: i16,
    w: i16,
    text: &str,
    color: u16,
    scale: i16,
) {
    let scale = if scale <= 1 { 1 } else { 2 };
    let len = text.chars().count() as i16;
    let text_width = len * 6 * scale - scale;
    draw_text(
        display,
        x + 0.max((w - text_width) / 2),
        y,
        text,
        color,
        scale,
    );
}

fn font_bit_val(cols: &[u8; 5], col: i16, row: i16) -> u16 {
    if col < 0 || col >= 5 || row < 0 || row >= 7 {
        return 0;
    }
    if cols[col as usize] & (1 << row as usize) != 0 {
        1
    } else {
        0
    }
}

fn compute_aa_coverage(cols: &[u8; 5], ox: usize, oy: usize, scale: usize) -> u8 {
    let w = 2 * scale as u16;
    let cx = (2 * ox as u16 + 1) as u16;
    let cy = (2 * oy as u16 + 1) as u16;
    let col = (cx / w) as i16;
    let row = (cy / w) as i16;
    let dx = cx % w;
    let dy = cy % w;
    let wx0 = w - dx;
    let wx1 = dx;
    let wy0 = w - dy;
    let wy1 = dy;
    let coverage = font_bit_val(cols, col, row) * wx0 * wy0
        + font_bit_val(cols, col + 1, row) * wx1 * wy0
        + font_bit_val(cols, col, row + 1) * wx0 * wy1
        + font_bit_val(cols, col + 1, row + 1) * wx1 * wy1;
    let max_val = w * w;
    let alpha = ((coverage * 4 + max_val / 2) / max_val) as u8;
    if alpha == 0 {
        0
    } else {
        (alpha + 1).min(4)
    }
}

pub fn draw_char_aa(
    display: &mut impl DisplaySink,
    x: i16,
    y: i16,
    c: char,
    fg: u16,
    bg: u16,
    scale: i16,
) {
    let scale = scale as usize;
    let char_w = 5 * scale;
    let char_h = 7 * scale;
    let cols = font_columns(c);
    for oy in 0..char_h {
        let mut run_x = x;
        let mut run_len: i16 = 0;
        let mut run_color = 0u16;
        for ox in 0..char_w {
            let alpha = compute_aa_coverage(&cols, ox, oy, scale);
            let color = crate::theme::rgb565_blend(bg, fg, alpha);
            if run_len > 0 && color == run_color {
                run_len += 1;
            } else {
                if run_len > 0 {
                    fill_rect(display, run_x, y + oy as i16, run_len, 1, run_color);
                }
                run_x = x + ox as i16;
                run_len = 1;
                run_color = color;
            }
        }
        if run_len > 0 {
            fill_rect(display, run_x, y + oy as i16, run_len, 1, run_color);
        }
    }
}

const fn font_columns(c: char) -> [u8; 5] {
    match c {
        '0' => [0x3E, 0x51, 0x49, 0x45, 0x3E],
        '1' => [0x00, 0x42, 0x7F, 0x40, 0x00],
        '2' => [0x42, 0x61, 0x51, 0x49, 0x46],
        '3' => [0x21, 0x41, 0x45, 0x4B, 0x31],
        '4' => [0x18, 0x14, 0x12, 0x7F, 0x10],
        '5' => [0x27, 0x45, 0x45, 0x45, 0x39],
        '6' => [0x3C, 0x4A, 0x49, 0x49, 0x30],
        '7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        '8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        '9' => [0x06, 0x49, 0x49, 0x29, 0x1E],
        ':' => [0x00, 0x36, 0x36, 0x00, 0x00],
        'A' | 'a' => [0x7E, 0x11, 0x11, 0x11, 0x7E],
        'B' | 'b' => [0x7F, 0x49, 0x49, 0x49, 0x36],
        'C' | 'c' => [0x3E, 0x41, 0x41, 0x41, 0x22],
        'D' | 'd' => [0x7F, 0x41, 0x41, 0x22, 0x1C],
        'E' | 'e' => [0x7F, 0x49, 0x49, 0x49, 0x41],
        'F' | 'f' => [0x7F, 0x09, 0x09, 0x09, 0x01],
        'G' | 'g' => [0x3E, 0x41, 0x49, 0x49, 0x7A],
        'H' | 'h' => [0x7F, 0x08, 0x08, 0x08, 0x7F],
        'I' | 'i' => [0x00, 0x41, 0x7F, 0x41, 0x00],
        'J' | 'j' => [0x20, 0x40, 0x41, 0x3F, 0x01],
        'K' | 'k' => [0x7F, 0x08, 0x14, 0x22, 0x41],
        'L' | 'l' => [0x7F, 0x40, 0x40, 0x40, 0x40],
        'M' | 'm' => [0x7F, 0x02, 0x0C, 0x02, 0x7F],
        'N' | 'n' => [0x7F, 0x04, 0x08, 0x10, 0x7F],
        'O' | 'o' => [0x3E, 0x41, 0x41, 0x41, 0x3E],
        'P' | 'p' => [0x7F, 0x09, 0x09, 0x09, 0x06],
        'Q' | 'q' => [0x3E, 0x41, 0x51, 0x21, 0x5E],
        'R' | 'r' => [0x7F, 0x09, 0x19, 0x29, 0x46],
        'S' | 's' => [0x46, 0x49, 0x49, 0x49, 0x31],
        'T' | 't' => [0x01, 0x01, 0x7F, 0x01, 0x01],
        'U' | 'u' => [0x3F, 0x40, 0x40, 0x40, 0x3F],
        'V' | 'v' => [0x1F, 0x20, 0x40, 0x20, 0x1F],
        'W' | 'w' => [0x7F, 0x20, 0x18, 0x20, 0x7F],
        'X' | 'x' => [0x63, 0x14, 0x08, 0x14, 0x63],
        'Y' | 'y' => [0x07, 0x08, 0x70, 0x08, 0x07],
        'Z' | 'z' => [0x61, 0x51, 0x49, 0x45, 0x43],
        _ => [0, 0, 0, 0, 0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::RecordingDisplay;

    #[test]
    fn draws_scaled_glyph_pixels() {
        let mut display = RecordingDisplay::new();
        draw_char(&mut display, 1, 2, '1', 0xffff, 2);
        assert!(display.commands().iter().any(|command| matches!(
            command,
            crate::render::DrawCommand::Fill { rect, color: 0xffff }
                if rect.w == 2 && rect.h == 2 && rect.x >= 1 && rect.y >= 2
        )));
    }
}
