use crate::config::Direction;
use crate::rng::Rng;

pub const TETRIS_COLS: usize = 10;
pub const TETRIS_ROWS: usize = 20;
pub const TETRIS_CELLS: usize = TETRIS_COLS * TETRIS_ROWS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TetrisMode {
    Choosing,
    Playing,
    Paused,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TetrisPauseAction {
    Continue,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TetrisChoosingAction {
    Start,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tetromino {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Tetromino {
    pub const fn from_index(index: usize) -> Self {
        match index {
            0 => Self::I,
            1 => Self::O,
            2 => Self::T,
            3 => Self::S,
            4 => Self::Z,
            5 => Self::J,
            _ => Self::L,
        }
    }

    pub const fn cell_value(self) -> u8 {
        match self {
            Self::I => 1,
            Self::O => 2,
            Self::T => 3,
            Self::S => 4,
            Self::Z => 5,
            Self::J => 6,
            Self::L => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TetrisBlock {
    pub x: i8,
    pub y: i8,
}

impl TetrisBlock {
    pub const fn new(x: i8, y: i8) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TetrisPiece {
    pub kind: Tetromino,
    pub rotation: u8,
    pub x: i8,
    pub y: i8,
}

impl TetrisPiece {
    pub const fn cells(self) -> [TetrisBlock; 4] {
        let local = tetromino_cells(self.kind, self.rotation);
        [
            TetrisBlock::new(self.x + local[0].x, self.y + local[0].y),
            TetrisBlock::new(self.x + local[1].x, self.y + local[1].y),
            TetrisBlock::new(self.x + local[2].x, self.y + local[2].y),
            TetrisBlock::new(self.x + local[3].x, self.y + local[3].y),
        ]
    }
}

#[derive(Debug, Clone)]
pub struct TetrisGame {
    board: [u8; TETRIS_CELLS],
    active: TetrisPiece,
    next: Tetromino,
    mode: TetrisMode,
    pause_action: TetrisPauseAction,
    choosing_action: TetrisChoosingAction,
    score: u32,
    lines: u32,
    last_tick_us: i64,
}

impl Default for TetrisGame {
    fn default() -> Self {
        Self {
            board: [0; TETRIS_CELLS],
            active: TetrisPiece {
                kind: Tetromino::I,
                rotation: 0,
                x: 3,
                y: 0,
            },
            next: Tetromino::O,
            mode: TetrisMode::Choosing,
            pause_action: TetrisPauseAction::Continue,
            choosing_action: TetrisChoosingAction::Start,
            score: 0,
            lines: 0,
            last_tick_us: 0,
        }
    }
}

impl TetrisGame {
    pub fn enter_choosing(&mut self) {
        self.mode = TetrisMode::Choosing;
        self.pause_action = TetrisPauseAction::Continue;
        self.choosing_action = TetrisChoosingAction::Start;
    }

    pub fn press_switch(&mut self, rng: &mut impl Rng, now_us: i64) -> bool {
        match self.mode {
            TetrisMode::Choosing | TetrisMode::GameOver => {
                self.start_new_game(rng, now_us);
                true
            }
            TetrisMode::Playing => {
                self.mode = TetrisMode::Paused;
                self.pause_action = TetrisPauseAction::Continue;
                true
            }
            TetrisMode::Paused => {
                match self.pause_action {
                    TetrisPauseAction::Continue => {
                        self.mode = TetrisMode::Playing;
                        self.last_tick_us = now_us;
                    }
                    TetrisPauseAction::Exit => self.enter_choosing(),
                }
                true
            }
        }
    }

    pub fn cycle_pause_action(&mut self) {
        self.pause_action = match self.pause_action {
            TetrisPauseAction::Continue => TetrisPauseAction::Exit,
            TetrisPauseAction::Exit => TetrisPauseAction::Continue,
        };
    }

    pub fn move_active(&mut self, direction: Direction) -> bool {
        if self.mode != TetrisMode::Playing {
            return false;
        }
        match direction {
            Direction::Left => self.try_offset(-1, 0),
            Direction::Right => self.try_offset(1, 0),
            Direction::Down => self.soft_drop(),
            Direction::Up => self.rotate_clockwise(),
        }
    }

    pub fn soft_drop(&mut self) -> bool {
        if self.try_offset(0, 1) {
            self.score = self.score.saturating_add(1);
            true
        } else {
            false
        }
    }

    pub fn tick(&mut self, rng: &mut impl Rng) {
        if self.mode != TetrisMode::Playing {
            return;
        }
        if !self.try_offset(0, 1) {
            self.lock_active();
            let cleared = self.clear_completed_lines();
            self.add_line_score(cleared);
            self.spawn_next(rng);
        }
    }

    pub fn due_for_tick(&self, now_us: i64) -> bool {
        self.mode == TetrisMode::Playing && now_us - self.last_tick_us >= self.tick_us()
    }

    pub fn mark_ticked(&mut self, now_us: i64) {
        self.last_tick_us = now_us;
    }

    pub const fn mode(&self) -> TetrisMode {
        self.mode
    }

    pub const fn pause_action(&self) -> TetrisPauseAction {
        self.pause_action
    }

    pub const fn choosing_action(&self) -> TetrisChoosingAction {
        self.choosing_action
    }

    pub fn cycle_choosing_action(&mut self) {
        self.choosing_action = match self.choosing_action {
            TetrisChoosingAction::Start => TetrisChoosingAction::Exit,
            TetrisChoosingAction::Exit => TetrisChoosingAction::Start,
        };
    }

    pub const fn board(&self) -> &[u8; TETRIS_CELLS] {
        &self.board
    }

    pub const fn active(&self) -> TetrisPiece {
        self.active
    }

    pub const fn next(&self) -> Tetromino {
        self.next
    }

    pub const fn score(&self) -> u32 {
        self.score
    }

    pub const fn lines(&self) -> u32 {
        self.lines
    }

    pub const fn level(&self) -> u32 {
        self.lines / 10
    }

    pub fn cell(&self, row: usize, col: usize) -> u8 {
        self.board[row * TETRIS_COLS + col]
    }

    pub fn occupied_cell(&self, row: usize, col: usize) -> u8 {
        let x = col as i8;
        let y = row as i8;
        for block in self.active.cells() {
            if block.x == x && block.y == y {
                return self.active.kind.cell_value();
            }
        }
        self.cell(row, col)
    }

    fn start_new_game(&mut self, rng: &mut impl Rng, now_us: i64) {
        self.board = [0; TETRIS_CELLS];
        self.score = 0;
        self.lines = 0;
        self.mode = TetrisMode::Playing;
        self.pause_action = TetrisPauseAction::Continue;
        self.active = spawn_piece(random_kind(rng));
        self.next = random_kind(rng);
        self.last_tick_us = now_us;
        if self.collides(self.active) {
            self.mode = TetrisMode::GameOver;
        }
    }

    fn spawn_next(&mut self, rng: &mut impl Rng) {
        self.active = spawn_piece(self.next);
        self.next = random_kind(rng);
        if self.collides(self.active) {
            self.mode = TetrisMode::GameOver;
        }
    }

    fn try_offset(&mut self, dx: i8, dy: i8) -> bool {
        let candidate = TetrisPiece {
            x: self.active.x + dx,
            y: self.active.y + dy,
            ..self.active
        };
        if self.collides(candidate) {
            false
        } else {
            self.active = candidate;
            true
        }
    }

    fn rotate_clockwise(&mut self) -> bool {
        if self.active.kind == Tetromino::O {
            return false;
        }
        let rotated = TetrisPiece {
            rotation: (self.active.rotation + 1) % 4,
            ..self.active
        };
        for kick_x in [0, -1, 1, -2, 2] {
            let candidate = TetrisPiece {
                x: rotated.x + kick_x,
                ..rotated
            };
            if !self.collides(candidate) {
                self.active = candidate;
                return true;
            }
        }
        false
    }

    fn collides(&self, piece: TetrisPiece) -> bool {
        for block in piece.cells() {
            if block.x < 0 || block.x >= TETRIS_COLS as i8 || block.y >= TETRIS_ROWS as i8 {
                return true;
            }
            if block.y >= 0 && self.board[block.y as usize * TETRIS_COLS + block.x as usize] != 0 {
                return true;
            }
        }
        false
    }

    fn lock_active(&mut self) {
        for block in self.active.cells() {
            if block.y < 0 {
                self.mode = TetrisMode::GameOver;
                return;
            }
            let index = block.y as usize * TETRIS_COLS + block.x as usize;
            self.board[index] = self.active.kind.cell_value();
        }
    }

    fn clear_completed_lines(&mut self) -> u32 {
        let mut cleared = 0u32;
        let mut write_row = TETRIS_ROWS as isize - 1;
        for read_row in (0..TETRIS_ROWS).rev() {
            if row_complete(&self.board, read_row) {
                cleared += 1;
                continue;
            }
            if write_row as usize != read_row {
                copy_row(&mut self.board, read_row, write_row as usize);
            }
            write_row -= 1;
        }
        while write_row >= 0 {
            clear_row(&mut self.board, write_row as usize);
            write_row -= 1;
        }
        self.lines += cleared;
        cleared
    }

    fn add_line_score(&mut self, cleared: u32) {
        let base = match cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        };
        self.score = self.score.saturating_add(base * (self.level() + 1));
    }

    fn tick_us(&self) -> i64 {
        let tick_ms = 700u32.saturating_sub(self.level().min(12) * 45).max(120);
        tick_ms as i64 * 1000
    }
}

fn spawn_piece(kind: Tetromino) -> TetrisPiece {
    TetrisPiece {
        kind,
        rotation: 0,
        x: 3,
        y: 0,
    }
}

fn random_kind(rng: &mut impl Rng) -> Tetromino {
    Tetromino::from_index(rng.index(7))
}

fn row_complete(board: &[u8; TETRIS_CELLS], row: usize) -> bool {
    for col in 0..TETRIS_COLS {
        if board[row * TETRIS_COLS + col] == 0 {
            return false;
        }
    }
    true
}

fn copy_row(board: &mut [u8; TETRIS_CELLS], from: usize, to: usize) {
    for col in 0..TETRIS_COLS {
        board[to * TETRIS_COLS + col] = board[from * TETRIS_COLS + col];
    }
}

fn clear_row(board: &mut [u8; TETRIS_CELLS], row: usize) {
    for col in 0..TETRIS_COLS {
        board[row * TETRIS_COLS + col] = 0;
    }
}

pub const fn tetromino_cells(kind: Tetromino, rotation: u8) -> [TetrisBlock; 4] {
    let r = rotation % 4;
    match kind {
        Tetromino::I => match r {
            0 => [
                TetrisBlock::new(0, 1),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(2, 1),
                TetrisBlock::new(3, 1),
            ],
            1 => [
                TetrisBlock::new(2, 0),
                TetrisBlock::new(2, 1),
                TetrisBlock::new(2, 2),
                TetrisBlock::new(2, 3),
            ],
            2 => [
                TetrisBlock::new(0, 2),
                TetrisBlock::new(1, 2),
                TetrisBlock::new(2, 2),
                TetrisBlock::new(3, 2),
            ],
            _ => [
                TetrisBlock::new(1, 0),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(1, 2),
                TetrisBlock::new(1, 3),
            ],
        },
        Tetromino::O => [
            TetrisBlock::new(1, 0),
            TetrisBlock::new(2, 0),
            TetrisBlock::new(1, 1),
            TetrisBlock::new(2, 1),
        ],
        Tetromino::T => match r {
            0 => [
                TetrisBlock::new(1, 0),
                TetrisBlock::new(0, 1),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(2, 1),
            ],
            1 => [
                TetrisBlock::new(1, 0),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(2, 1),
                TetrisBlock::new(1, 2),
            ],
            2 => [
                TetrisBlock::new(0, 1),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(2, 1),
                TetrisBlock::new(1, 2),
            ],
            _ => [
                TetrisBlock::new(1, 0),
                TetrisBlock::new(0, 1),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(1, 2),
            ],
        },
        Tetromino::S => match r {
            0 | 2 => [
                TetrisBlock::new(1, 0),
                TetrisBlock::new(2, 0),
                TetrisBlock::new(0, 1),
                TetrisBlock::new(1, 1),
            ],
            _ => [
                TetrisBlock::new(1, 0),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(2, 1),
                TetrisBlock::new(2, 2),
            ],
        },
        Tetromino::Z => match r {
            0 | 2 => [
                TetrisBlock::new(0, 0),
                TetrisBlock::new(1, 0),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(2, 1),
            ],
            _ => [
                TetrisBlock::new(2, 0),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(2, 1),
                TetrisBlock::new(1, 2),
            ],
        },
        Tetromino::J => match r {
            0 => [
                TetrisBlock::new(0, 0),
                TetrisBlock::new(0, 1),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(2, 1),
            ],
            1 => [
                TetrisBlock::new(1, 0),
                TetrisBlock::new(2, 0),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(1, 2),
            ],
            2 => [
                TetrisBlock::new(0, 1),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(2, 1),
                TetrisBlock::new(2, 2),
            ],
            _ => [
                TetrisBlock::new(1, 0),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(0, 2),
                TetrisBlock::new(1, 2),
            ],
        },
        Tetromino::L => match r {
            0 => [
                TetrisBlock::new(2, 0),
                TetrisBlock::new(0, 1),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(2, 1),
            ],
            1 => [
                TetrisBlock::new(1, 0),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(1, 2),
                TetrisBlock::new(2, 2),
            ],
            2 => [
                TetrisBlock::new(0, 1),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(2, 1),
                TetrisBlock::new(0, 2),
            ],
            _ => [
                TetrisBlock::new(0, 0),
                TetrisBlock::new(1, 0),
                TetrisBlock::new(1, 1),
                TetrisBlock::new(1, 2),
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::ScriptedRng;

    #[test]
    fn switch_starts_game_with_deterministic_piece_queue() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([2, 6]);

        assert!(game.press_switch(&mut rng, 10));

        assert_eq!(game.mode(), TetrisMode::Playing);
        assert_eq!(game.active().kind, Tetromino::T);
        assert_eq!(game.next(), Tetromino::L);
    }

    #[test]
    fn piece_moves_left_and_right_within_well() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([1, 1]);
        game.press_switch(&mut rng, 0);

        assert!(game.move_active(Direction::Left));
        assert_eq!(game.active().x, 2);
        assert!(game.move_active(Direction::Right));
        assert_eq!(game.active().x, 3);
    }

    #[test]
    fn rotation_changes_piece_orientation() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([0, 1]);
        game.press_switch(&mut rng, 0);

        assert!(game.move_active(Direction::Up));
        assert_eq!(game.active().rotation, 1);
    }

    #[test]
    fn tick_locks_piece_at_bottom_and_spawns_next() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([1, 2, 3]);
        game.press_switch(&mut rng, 0);

        for _ in 0..TETRIS_ROWS {
            game.tick(&mut rng);
        }

        assert_eq!(game.active().kind, Tetromino::T);
        assert!(game
            .board()
            .iter()
            .any(|&cell| cell == Tetromino::O.cell_value()));
    }

    #[test]
    fn clearing_line_adds_score_and_lines() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([0, 0]);
        game.press_switch(&mut rng, 0);
        for col in 0..TETRIS_COLS {
            game.board[(TETRIS_ROWS - 1) * TETRIS_COLS + col] = 1;
        }

        let cleared = game.clear_completed_lines();
        game.add_line_score(cleared);

        assert_eq!(game.lines(), 1);
        assert_eq!(game.score(), 100);
        assert_eq!(
            game.board()[(TETRIS_ROWS - 1) * TETRIS_COLS..TETRIS_ROWS * TETRIS_COLS],
            [0; TETRIS_COLS]
        );
    }

    #[test]
    fn spawn_collision_enters_game_over() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([1, 1, 1]);
        game.press_switch(&mut rng, 0);
        for col in 0..TETRIS_COLS {
            game.board[col] = 1;
        }

        game.spawn_next(&mut rng);

        assert_eq!(game.mode(), TetrisMode::GameOver);
    }

    #[test]
    fn press_switch_from_game_over_starts_new_game() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([1, 2]);
        game.press_switch(&mut rng, 0);
        game.mode = TetrisMode::GameOver;
        assert!(game.press_switch(&mut rng, 100));
        assert_eq!(game.mode(), TetrisMode::Playing);
    }

    #[test]
    fn press_switch_paused_continue_resumes() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([1, 2]);
        game.press_switch(&mut rng, 0);
        game.mode = TetrisMode::Paused;
        game.pause_action = TetrisPauseAction::Continue;
        assert!(game.press_switch(&mut rng, 100));
        assert_eq!(game.mode(), TetrisMode::Playing);
    }

    #[test]
    fn press_switch_paused_exit_enters_choosing() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([1, 2]);
        game.press_switch(&mut rng, 0);
        game.mode = TetrisMode::Paused;
        game.pause_action = TetrisPauseAction::Exit;
        assert!(game.press_switch(&mut rng, 100));
        assert_eq!(game.mode(), TetrisMode::Choosing);
    }

    #[test]
    fn press_switch_playing_enters_paused() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([1, 2]);
        game.press_switch(&mut rng, 0);
        assert!(game.press_switch(&mut rng, 100));
        assert_eq!(game.mode(), TetrisMode::Paused);
    }

    #[test]
    fn press_switch_choosing_starts_game() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([0, 1]);
        assert!(game.press_switch(&mut rng, 0));
        assert_eq!(game.mode(), TetrisMode::Playing);
    }

    #[test]
    fn cycle_pause_action_toggles() {
        let mut game = TetrisGame::default();
        assert_eq!(game.pause_action(), TetrisPauseAction::Continue);
        game.cycle_pause_action();
        assert_eq!(game.pause_action(), TetrisPauseAction::Exit);
        game.cycle_pause_action();
        assert_eq!(game.pause_action(), TetrisPauseAction::Continue);
    }

    #[test]
    fn cycle_choosing_action_toggles() {
        let mut game = TetrisGame::default();
        assert_eq!(game.choosing_action(), TetrisChoosingAction::Start);
        game.cycle_choosing_action();
        assert_eq!(game.choosing_action(), TetrisChoosingAction::Exit);
        game.cycle_choosing_action();
        assert_eq!(game.choosing_action(), TetrisChoosingAction::Start);
    }

    #[test]
    fn move_active_returns_false_when_not_playing() {
        let mut game = TetrisGame::default();
        assert!(!game.move_active(Direction::Left));
    }

    #[test]
    fn soft_drop_adds_score() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([1, 1]);
        game.press_switch(&mut rng, 0);
        let initial_score = game.score();
        assert!(game.soft_drop());
        assert!(game.score() > initial_score);
    }

    #[test]
    fn tick_does_nothing_when_not_playing() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([1]);
        game.tick(&mut rng);
        assert_eq!(game.score(), 0);
    }

    #[test]
    fn due_for_tick_returns_false_when_not_playing() {
        let game = TetrisGame::default();
        assert!(!game.due_for_tick(1_000_000));
    }

    #[test]
    fn add_line_score_all_combinations() {
        let mut game = TetrisGame::default();
        game.add_line_score(1);
        assert_eq!(game.score(), 100);
        game.add_line_score(2);
        assert_eq!(game.score(), 400);
        game.add_line_score(3);
        assert_eq!(game.score(), 900);
        game.add_line_score(4);
        assert_eq!(game.score(), 1700);
    }

    #[test]
    fn tetromino_from_index_covers_all() {
        assert_eq!(Tetromino::from_index(0), Tetromino::I);
        assert_eq!(Tetromino::from_index(1), Tetromino::O);
        assert_eq!(Tetromino::from_index(2), Tetromino::T);
        assert_eq!(Tetromino::from_index(3), Tetromino::S);
        assert_eq!(Tetromino::from_index(4), Tetromino::Z);
        assert_eq!(Tetromino::from_index(5), Tetromino::J);
        assert_eq!(Tetromino::from_index(6), Tetromino::L);
        assert_eq!(Tetromino::from_index(7), Tetromino::L);
    }

    #[test]
    fn tetromino_cell_value_covers_all() {
        assert_eq!(Tetromino::I.cell_value(), 1);
        assert_eq!(Tetromino::O.cell_value(), 2);
        assert_eq!(Tetromino::T.cell_value(), 3);
        assert_eq!(Tetromino::S.cell_value(), 4);
        assert_eq!(Tetromino::Z.cell_value(), 5);
        assert_eq!(Tetromino::J.cell_value(), 6);
        assert_eq!(Tetromino::L.cell_value(), 7);
    }

    #[test]
    fn rotation_o_does_not_rotate() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([1, 1]);
        game.press_switch(&mut rng, 0);
        game.active.kind = Tetromino::O;
        assert!(!game.move_active(Direction::Up));
    }

    #[test]
    fn level_increases_with_lines() {
        let mut game = TetrisGame::default();
        assert_eq!(game.level(), 0);
        game.lines = 10;
        assert_eq!(game.level(), 1);
        game.lines = 25;
        assert_eq!(game.level(), 2);
    }

    #[test]
    fn tick_us_decreases_with_level() {
        let game0 = TetrisGame::default();
        let tick0 = game0.tick_us();
        let mut game10 = TetrisGame::default();
        game10.lines = 100;
        let tick10 = game10.tick_us();
        assert!(tick10 < tick0);
    }

    #[test]
    fn occupied_cell_returns_active_piece_value() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([0, 1]);
        game.press_switch(&mut rng, 0);
        let active = game.active();
        let first_block = active.cells()[0];
        let value = game.occupied_cell(first_block.y as usize, first_block.x as usize);
        assert_eq!(value, Tetromino::I.cell_value());
    }

    #[test]
    fn enter_choosing_resets_state() {
        let mut game = TetrisGame::default();
        let mut rng = ScriptedRng::new([0, 1]);
        game.press_switch(&mut rng, 0);
        assert_eq!(game.mode(), TetrisMode::Playing);
        game.enter_choosing();
        assert_eq!(game.mode(), TetrisMode::Choosing);
        assert_eq!(game.choosing_action(), TetrisChoosingAction::Start);
    }
}
