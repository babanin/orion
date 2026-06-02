use crate::theme;

pub const OM_NOM_W: i16 = 18;
pub const OM_NOM_H: i16 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteSpan {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub palette: u8,
}

pub const OM_NOM_PALETTE: [u16; 6] = [
    theme::rgb565(43, 192, 68),
    theme::rgb565(77, 225, 92),
    theme::rgb565(12, 75, 28),
    theme::rgb565(248, 255, 240),
    theme::rgb565(4, 9, 8),
    theme::rgb565(214, 35, 44),
];

pub const OM_NOM_SPANS: &[SpriteSpan] = &[
    SpriteSpan {
        x: 7,
        y: 0,
        w: 4,
        palette: 2,
    },
    SpriteSpan {
        x: 5,
        y: 1,
        w: 8,
        palette: 2,
    },
    SpriteSpan {
        x: 4,
        y: 2,
        w: 10,
        palette: 0,
    },
    SpriteSpan {
        x: 3,
        y: 3,
        w: 12,
        palette: 0,
    },
    SpriteSpan {
        x: 2,
        y: 4,
        w: 14,
        palette: 0,
    },
    SpriteSpan {
        x: 1,
        y: 5,
        w: 16,
        palette: 0,
    },
    SpriteSpan {
        x: 1,
        y: 6,
        w: 16,
        palette: 1,
    },
    SpriteSpan {
        x: 0,
        y: 7,
        w: 18,
        palette: 1,
    },
    SpriteSpan {
        x: 0,
        y: 8,
        w: 18,
        palette: 1,
    },
    SpriteSpan {
        x: 0,
        y: 9,
        w: 18,
        palette: 1,
    },
    SpriteSpan {
        x: 0,
        y: 10,
        w: 18,
        palette: 1,
    },
    SpriteSpan {
        x: 1,
        y: 11,
        w: 16,
        palette: 0,
    },
    SpriteSpan {
        x: 1,
        y: 12,
        w: 16,
        palette: 0,
    },
    SpriteSpan {
        x: 2,
        y: 13,
        w: 14,
        palette: 0,
    },
    SpriteSpan {
        x: 3,
        y: 14,
        w: 12,
        palette: 0,
    },
    SpriteSpan {
        x: 4,
        y: 15,
        w: 10,
        palette: 0,
    },
    SpriteSpan {
        x: 5,
        y: 16,
        w: 8,
        palette: 2,
    },
    SpriteSpan {
        x: 7,
        y: 17,
        w: 4,
        palette: 2,
    },
    SpriteSpan {
        x: 4,
        y: 6,
        w: 4,
        palette: 3,
    },
    SpriteSpan {
        x: 11,
        y: 6,
        w: 4,
        palette: 3,
    },
    SpriteSpan {
        x: 5,
        y: 7,
        w: 2,
        palette: 4,
    },
    SpriteSpan {
        x: 12,
        y: 7,
        w: 2,
        palette: 4,
    },
    SpriteSpan {
        x: 7,
        y: 10,
        w: 5,
        palette: 4,
    },
    SpriteSpan {
        x: 6,
        y: 11,
        w: 7,
        palette: 5,
    },
    SpriteSpan {
        x: 7,
        y: 12,
        w: 5,
        palette: 5,
    },
    SpriteSpan {
        x: 8,
        y: 13,
        w: 3,
        palette: 3,
    },
];
