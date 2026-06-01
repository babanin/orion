pub const fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    let color = (((r as u16) >> 3) << 11) | (((g as u16) >> 2) << 5) | ((b as u16) >> 3);
    color.rotate_left(8)
}

pub fn rgb565_blend(bg: u16, fg: u16, alpha: u8) -> u16 {
    match alpha {
        0 => bg,
        4 => fg,
        a => {
            let bg_std = bg.rotate_right(8);
            let fg_std = fg.rotate_right(8);
            let bg_r = (bg_std >> 11) & 0x1F;
            let bg_g = (bg_std >> 5) & 0x3F;
            let bg_b = bg_std & 0x1F;
            let fg_r = (fg_std >> 11) & 0x1F;
            let fg_g = (fg_std >> 5) & 0x3F;
            let fg_b = fg_std & 0x1F;
            let t = a as u16;
            let r = (bg_r * (4 - t) + fg_r * t + 2) / 4;
            let g = (bg_g * (4 - t) + fg_g * t + 2) / 4;
            let b = (bg_b * (4 - t) + fg_b * t + 2) / 4;
            ((r << 11) | (g << 5) | b).rotate_left(8)
        }
    }
}

pub const BG: u16 = rgb565(9, 12, 16);
pub const HUD: u16 = rgb565(21, 27, 35);
pub const GRID: u16 = rgb565(14, 19, 25);
pub const TEXT: u16 = rgb565(230, 237, 243);
pub const MUTED: u16 = rgb565(139, 148, 158);
pub const ACCENT: u16 = rgb565(88, 166, 255);
pub const GOOD: u16 = rgb565(46, 160, 67);
pub const BAD: u16 = rgb565(218, 54, 51);
pub const SNAKE: u16 = rgb565(38, 166, 74);
pub const SNAKE_MARK: u16 = rgb565(18, 111, 48);
pub const HEAD: u16 = rgb565(80, 200, 104);
pub const HEAD_MARK: u16 = rgb565(26, 137, 61);
pub const EYE: u16 = rgb565(3, 7, 10);
pub const APPLE: u16 = rgb565(220, 20, 36);
pub const APPLE_DARK: u16 = rgb565(140, 12, 28);
pub const APPLE_HIGHLIGHT: u16 = rgb565(255, 225, 220);
pub const STEM: u16 = rgb565(112, 64, 28);
pub const LEAF: u16 = rgb565(42, 170, 76);
pub const OVERLAY: u16 = rgb565(31, 41, 55);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_matches_cpp_byte_swap() {
        assert_eq!(rgb565(255, 0, 0), 0x00f8);
        assert_eq!(rgb565(0, 255, 0), 0xe007);
        assert_eq!(rgb565(0, 0, 255), 0x1f00);
    }

    #[test]
    fn rgb565_blend_returns_bg_at_alpha_zero() {
        let bg = rgb565(10, 20, 30);
        let fg = rgb565(200, 210, 220);
        assert_eq!(rgb565_blend(bg, fg, 0), bg);
    }

    #[test]
    fn rgb565_blend_returns_fg_at_alpha_four() {
        let bg = rgb565(10, 20, 30);
        let fg = rgb565(200, 210, 220);
        assert_eq!(rgb565_blend(bg, fg, 4), fg);
    }

    #[test]
    fn rgb565_blend_interpolates() {
        let bg = rgb565(0, 0, 0);
        let fg = rgb565(255, 255, 255);
        let mid = rgb565_blend(bg, fg, 2);
        let mid_std = mid.rotate_right(8);
        let r = (mid_std >> 11) & 0x1F;
        let g = (mid_std >> 5) & 0x3F;
        let b = mid_std & 0x1F;
        assert!(r > 6 && r < 18);
        assert!(g > 14 && g < 50);
        assert!(b > 6 && b < 18);
    }
}
