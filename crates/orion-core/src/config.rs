#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub const fn is_opposite(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Up, Self::Down)
                | (Self::Down, Self::Up)
                | (Self::Left, Self::Right)
                | (Self::Right, Self::Left)
        )
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub x: i16,
    pub y: i16,
}

impl Cell {
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

pub const TFT_H_RES: i16 = 320;
pub const TFT_V_RES: i16 = 240;
pub const HUD_HEIGHT: i16 = 16;
pub const CELL_SIZE: i16 = 16;
pub const BOARD_COLS: i16 = 20;
pub const BOARD_ROWS: i16 = (TFT_V_RES - HUD_HEIGHT) / CELL_SIZE;
pub const BOARD_CELLS: usize = (BOARD_COLS as usize) * (BOARD_ROWS as usize);
pub const JOYSTICK_DEADZONE: i32 = 450;
pub const JOYSTICK_THRESHOLD: i32 = 700;
pub const MENU_COLS: usize = 2;
pub const BUTTON_DEBOUNCE_MS: u64 = 80;
pub const FLAGS_PRACTICE_EXIT_HOLD_MS: u64 = 700;
pub const ENCODER_STEPS_PER_DETENT: i8 = 4;

pub const fn on_board(cell: Cell) -> bool {
    cell.x >= 0 && cell.y >= 0 && cell.x < BOARD_COLS && cell.y < BOARD_ROWS
}

pub const fn wrapped_cell(cell: Cell) -> Cell {
    let mut x = cell.x;
    let mut y = cell.y;
    if x < 0 {
        x = BOARD_COLS - 1;
    } else if x >= BOARD_COLS {
        x = 0;
    }
    if y < 0 {
        y = BOARD_ROWS - 1;
    } else if y >= BOARD_ROWS {
        y = 0;
    }
    Cell { x, y }
}

pub const fn wrap_index(index: i32, count: i32) -> i32 {
    ((index % count) + count) % count
}
