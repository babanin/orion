use crate::config::Direction;
use crate::rng::Rng;
use crate::store::HighScoreStore;

pub const MAX_GRID_SIZE: usize = 5;
pub const MAX_CELLS: usize = MAX_GRID_SIZE * MAX_GRID_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridSize {
    Small,
    Classic,
    Large,
}

impl GridSize {
    pub const fn size(self) -> usize {
        match self {
            Self::Small => 3,
            Self::Classic => 4,
            Self::Large => 5,
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::Small => 0,
            Self::Classic => 1,
            Self::Large => 2,
        }
    }

    pub const fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Small,
            1 => Self::Classic,
            _ => Self::Large,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Small => "3X3",
            Self::Classic => "4X4",
            Self::Large => "5X5",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Game2048Mode {
    Choosing,
    Playing,
    Paused,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseAction {
    Continue,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Game2048ChoosingField {
    Size,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Game2048GameOverAction {
    Restart,
    Exit,
}

#[derive(Debug, Clone)]
pub struct Game2048 {
    pub(crate) grid: [u16; MAX_CELLS],
    pub(crate) score: u32,
    best_score: u32,
    grid_size: GridSize,
    pub(crate) mode: Game2048Mode,
    pub(crate) pause_action: PauseAction,
    pub(crate) choosing_field: Game2048ChoosingField,
    pub(crate) game_over_action: Game2048GameOverAction,
}

impl Default for Game2048 {
    fn default() -> Self {
        Self {
            grid: [0; MAX_CELLS],
            score: 0,
            best_score: 0,
            grid_size: GridSize::Classic,
            mode: Game2048Mode::Choosing,
            pause_action: PauseAction::Continue,
            choosing_field: Game2048ChoosingField::Size,
            game_over_action: Game2048GameOverAction::Restart,
        }
    }
}

impl Game2048 {
    pub fn enter_choosing(&mut self, high_scores: &impl HighScoreStore) {
        self.mode = Game2048Mode::Choosing;
        self.refresh_best_score(high_scores);
        self.pause_action = PauseAction::Continue;
        self.choosing_field = Game2048ChoosingField::Size;
        self.game_over_action = Game2048GameOverAction::Restart;
    }

    pub fn refresh_best_score(&mut self, high_scores: &impl HighScoreStore) {
        self.best_score = high_scores.game2048_best_score(self.grid_size);
    }

    pub fn mode(&self) -> Game2048Mode {
        self.mode
    }

    pub fn grid_size(&self) -> GridSize {
        self.grid_size
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn best_score(&self) -> u32 {
        self.best_score
    }

    pub fn pause_action(&self) -> PauseAction {
        self.pause_action
    }

    pub fn choosing_field(&self) -> Game2048ChoosingField {
        self.choosing_field
    }

    pub fn game_over_action(&self) -> Game2048GameOverAction {
        self.game_over_action
    }

    pub fn grid(&self) -> &[u16; MAX_CELLS] {
        &self.grid
    }

    pub fn cell(&self, row: usize, col: usize) -> u16 {
        self.grid[row * MAX_GRID_SIZE + col]
    }

    pub fn adjust_grid_size(&mut self, delta: i32) -> bool {
        let sizes = [GridSize::Small, GridSize::Classic, GridSize::Large];
        let current_index = self.grid_size.index() as i32;
        let new_index = (current_index + delta).clamp(0, sizes.len() as i32 - 1) as usize;
        let new_size = sizes[new_index];
        if new_size == self.grid_size {
            return false;
        }
        self.grid_size = new_size;
        true
    }

    pub fn press_switch(
        &mut self,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
    ) -> bool {
        match self.mode {
            Game2048Mode::Choosing => {
                self.start_new_game(high_scores, rng);
                true
            }
            Game2048Mode::Playing => {
                self.mode = Game2048Mode::Paused;
                self.pause_action = PauseAction::Continue;
                true
            }
            Game2048Mode::Paused => {
                match self.pause_action {
                    PauseAction::Continue => {
                        self.mode = Game2048Mode::Playing;
                    }
                    PauseAction::Exit => {
                        self.enter_choosing(high_scores);
                    }
                }
                true
            }
            Game2048Mode::GameOver => {
                self.enter_choosing(high_scores);
                true
            }
        }
    }

    pub fn cycle_pause_action(&mut self) {
        self.pause_action = match self.pause_action {
            PauseAction::Continue => PauseAction::Exit,
            PauseAction::Exit => PauseAction::Continue,
        };
    }

    pub fn cycle_choosing_field(&mut self) {
        self.choosing_field = match self.choosing_field {
            Game2048ChoosingField::Size => Game2048ChoosingField::Exit,
            Game2048ChoosingField::Exit => Game2048ChoosingField::Size,
        };
    }

    pub fn cycle_game_over_action(&mut self) {
        self.game_over_action = match self.game_over_action {
            Game2048GameOverAction::Restart => Game2048GameOverAction::Exit,
            Game2048GameOverAction::Exit => Game2048GameOverAction::Restart,
        };
    }

    pub fn select_next_choosing_field(&mut self) -> bool {
        if self.choosing_field == Game2048ChoosingField::Size {
            self.choosing_field = Game2048ChoosingField::Exit;
            true
        } else {
            false
        }
    }

    pub fn select_previous_choosing_field(&mut self) -> bool {
        if self.choosing_field == Game2048ChoosingField::Exit {
            self.choosing_field = Game2048ChoosingField::Size;
            true
        } else {
            false
        }
    }

    pub fn slide(&mut self, direction: Direction) -> bool {
        if self.mode != Game2048Mode::Playing {
            return false;
        }

        let size = self.grid_size.size();
        let old_grid = self.grid;
        let mut total_score = 0u32;

        match direction {
            Direction::Left => {
                for row in 0..size {
                    let mut line = extract_row_left(self.grid, row, size);
                    total_score += slide_line(&mut line, size);
                    write_row_left(&mut self.grid, row, size, &line);
                }
            }
            Direction::Right => {
                for row in 0..size {
                    let mut line = extract_row_right(self.grid, row, size);
                    total_score += slide_line(&mut line, size);
                    write_row_right(&mut self.grid, row, size, &line);
                }
            }
            Direction::Up => {
                for col in 0..size {
                    let mut line = extract_col_up(self.grid, col, size);
                    total_score += slide_line(&mut line, size);
                    write_col_up(&mut self.grid, col, size, &line);
                }
            }
            Direction::Down => {
                for col in 0..size {
                    let mut line = extract_col_down(self.grid, col, size);
                    total_score += slide_line(&mut line, size);
                    write_col_down(&mut self.grid, col, size, &line);
                }
            }
        }

        if self.grid != old_grid {
            self.score += total_score;
            true
        } else {
            false
        }
    }

    pub fn place_random_tile(&mut self, rng: &mut impl Rng) {
        let size = self.grid_size.size();
        let mut empty_count = 0usize;
        let mut empty_indices = [0usize; MAX_CELLS];

        for row in 0..size {
            for col in 0..size {
                let idx = row * MAX_GRID_SIZE + col;
                if self.grid[idx] == 0 {
                    empty_indices[empty_count] = idx;
                    empty_count += 1;
                }
            }
        }

        if empty_count == 0 {
            return;
        }

        let pos = empty_indices[rng.index(empty_count)];
        let value = if rng.index(10) == 0 { 4 } else { 2 };
        self.grid[pos] = value;
    }

    pub fn is_game_over(&self) -> bool {
        let size = self.grid_size.size();
        for row in 0..size {
            for col in 0..size {
                let idx = row * MAX_GRID_SIZE + col;
                if self.grid[idx] == 0 {
                    return false;
                }
                if col + 1 < size && self.grid[idx] == self.grid[row * MAX_GRID_SIZE + col + 1] {
                    return false;
                }
                if row + 1 < size && self.grid[idx] == self.grid[(row + 1) * MAX_GRID_SIZE + col] {
                    return false;
                }
            }
        }
        true
    }

    pub fn enter_game_over(&mut self, high_scores: &mut impl HighScoreStore) {
        self.mode = Game2048Mode::GameOver;
        if self.score > self.best_score {
            self.best_score = self.score;
        }
        high_scores.update_game2048_best_score(self.score, self.grid_size);
    }

    pub fn update_best_score(&mut self, high_scores: &mut impl HighScoreStore) {
        if self.score > self.best_score {
            self.best_score = self.score;
        }
        high_scores.update_game2048_best_score(self.score, self.grid_size);
    }

    fn start_new_game(&mut self, high_scores: &mut impl HighScoreStore, rng: &mut impl Rng) {
        self.grid = [0; MAX_CELLS];
        self.score = 0;
        self.mode = Game2048Mode::Playing;
        self.best_score = high_scores.game2048_best_score(self.grid_size);
        self.place_random_tile(rng);
        self.place_random_tile(rng);
    }
}

fn slide_line(line: &mut [u16; MAX_GRID_SIZE], size: usize) -> u32 {
    let mut compacted = [0u16; MAX_GRID_SIZE];
    let mut compact_len = 0usize;
    for value in line.iter().take(size) {
        if *value != 0 {
            compacted[compact_len] = *value;
            compact_len += 1;
        }
    }

    let mut result = [0u16; MAX_GRID_SIZE];
    let mut result_len = 0usize;
    let mut score = 0u32;
    let mut i = 0;
    while i < compact_len {
        if i + 1 < compact_len && compacted[i] == compacted[i + 1] {
            let merged = compacted[i] * 2;
            result[result_len] = merged;
            score += merged as u32;
            i += 2;
        } else {
            result[result_len] = compacted[i];
            i += 1;
        }
        result_len += 1;
    }

    line[..size].copy_from_slice(&result[..size]);
    score
}

fn extract_row_left(grid: [u16; MAX_CELLS], row: usize, size: usize) -> [u16; MAX_GRID_SIZE] {
    let mut line = [0u16; MAX_GRID_SIZE];
    for col in 0..size {
        line[col] = grid[row * MAX_GRID_SIZE + col];
    }
    line
}

fn write_row_left(
    grid: &mut [u16; MAX_CELLS],
    row: usize,
    size: usize,
    line: &[u16; MAX_GRID_SIZE],
) {
    for col in 0..size {
        grid[row * MAX_GRID_SIZE + col] = line[col];
    }
}

fn extract_row_right(grid: [u16; MAX_CELLS], row: usize, size: usize) -> [u16; MAX_GRID_SIZE] {
    let mut line = [0u16; MAX_GRID_SIZE];
    for col in 0..size {
        line[col] = grid[row * MAX_GRID_SIZE + (size - 1 - col)];
    }
    line
}

fn write_row_right(
    grid: &mut [u16; MAX_CELLS],
    row: usize,
    size: usize,
    line: &[u16; MAX_GRID_SIZE],
) {
    for col in 0..size {
        grid[row * MAX_GRID_SIZE + (size - 1 - col)] = line[col];
    }
}

fn extract_col_up(grid: [u16; MAX_CELLS], col: usize, size: usize) -> [u16; MAX_GRID_SIZE] {
    let mut line = [0u16; MAX_GRID_SIZE];
    for row in 0..size {
        line[row] = grid[row * MAX_GRID_SIZE + col];
    }
    line
}

fn write_col_up(grid: &mut [u16; MAX_CELLS], col: usize, size: usize, line: &[u16; MAX_GRID_SIZE]) {
    for row in 0..size {
        grid[row * MAX_GRID_SIZE + col] = line[row];
    }
}

fn extract_col_down(grid: [u16; MAX_CELLS], col: usize, size: usize) -> [u16; MAX_GRID_SIZE] {
    let mut line = [0u16; MAX_GRID_SIZE];
    for row in 0..size {
        line[row] = grid[(size - 1 - row) * MAX_GRID_SIZE + col];
    }
    line
}

fn write_col_down(
    grid: &mut [u16; MAX_CELLS],
    col: usize,
    size: usize,
    line: &[u16; MAX_GRID_SIZE],
) {
    for row in 0..size {
        grid[(size - 1 - row) * MAX_GRID_SIZE + col] = line[row];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::ScriptedRng;
    use crate::store::MemoryHighScoreStore;

    fn make_grid(size: usize, values: &[u16]) -> [u16; MAX_CELLS] {
        let mut grid = [0u16; MAX_CELLS];
        for row in 0..size {
            for col in 0..size {
                grid[row * MAX_GRID_SIZE + col] = values[row * size + col];
            }
        }
        grid
    }

    #[test]
    fn slide_left_merges_adjacent() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.mode = Game2048Mode::Playing;
        game.grid = make_grid(4, &[2, 2, 0, 0, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        let moved = game.slide(Direction::Left);
        assert!(moved);
        assert_eq!(game.cell(0, 0), 4);
        assert_eq!(game.cell(0, 1), 0);
        assert_eq!(game.cell(1, 0), 8);
    }

    #[test]
    fn slide_left_no_move_returns_false() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.grid = make_grid(4, &[2, 4, 8, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        game.mode = Game2048Mode::Playing;

        let moved = game.slide(Direction::Left);
        assert!(!moved);
    }

    #[test]
    fn slide_right_shifts_tiles() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.mode = Game2048Mode::Playing;
        game.grid = make_grid(4, &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        let moved = game.slide(Direction::Right);
        assert!(moved);
        assert_eq!(game.cell(0, 3), 2);
        assert_eq!(game.cell(0, 0), 0);
    }

    #[test]
    fn slide_merges_only_once_per_move() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.mode = Game2048Mode::Playing;
        game.grid = make_grid(4, &[2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        game.slide(Direction::Left);
        assert_eq!(game.cell(0, 0), 4);
        assert_eq!(game.cell(0, 1), 4);
        assert_eq!(game.cell(0, 2), 0);
        assert_eq!(game.cell(0, 3), 0);
    }

    #[test]
    fn slide_adds_score_on_merge() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.grid = make_grid(4, &[2, 2, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        game.mode = Game2048Mode::Playing;

        game.slide(Direction::Left);
        assert_eq!(game.score, 12);
    }

    #[test]
    fn slide_down_merges_column() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.mode = Game2048Mode::Playing;
        game.grid = make_grid(4, &[2, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        game.slide(Direction::Down);
        assert_eq!(game.cell(3, 0), 4);
        assert_eq!(game.cell(2, 0), 0);
    }

    #[test]
    fn slide_up_merges_column() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.mode = Game2048Mode::Playing;
        game.grid = make_grid(4, &[0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0]);

        game.slide(Direction::Up);
        assert_eq!(game.cell(0, 0), 4);
        assert_eq!(game.cell(1, 0), 0);
    }

    #[test]
    fn is_game_over_detects_full_board_no_merges() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.grid = make_grid(4, &[2, 4, 2, 4, 4, 2, 4, 2, 2, 4, 8, 16, 32, 64, 128, 256]);
        assert!(game.is_game_over());
    }

    #[test]
    fn is_game_over_false_with_merge_available() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.grid = make_grid(
            4,
            &[
                2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 2, 8192, 16384, 32768, 2,
            ],
        );
        assert!(!game.is_game_over());
    }

    #[test]
    fn is_game_over_false_with_empty_cell() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.grid = make_grid(
            4,
            &[
                2, 4, 8, 16, 32, 64, 128, 256, 0, 1024, 2048, 4096, 8192, 16384, 2, 4,
            ],
        );
        assert!(!game.is_game_over());
    }

    #[test]
    fn place_random_tile_fills_empty_cell() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.mode = Game2048Mode::Playing;
        let mut rng = ScriptedRng::new([0, 0]);

        game.place_random_tile(&mut rng);

        let non_zero_count = game.grid[..MAX_GRID_SIZE * 4]
            .iter()
            .filter(|&&v| v != 0)
            .count();
        assert_eq!(non_zero_count, 1);
        let value = game.grid.iter().find(|&&v| v != 0).unwrap();
        assert!(*value == 2 || *value == 4);
    }

    #[test]
    fn press_switch_starts_game_from_choosing() {
        let mut game = Game2048::default();
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 5, 3, 7]);

        game.press_switch(&mut scores, &mut rng);
        assert_eq!(game.mode(), Game2048Mode::Playing);
    }

    #[test]
    fn grid_size_cycling() {
        let mut game = Game2048::default();
        assert_eq!(game.grid_size(), GridSize::Classic);
        assert!(game.adjust_grid_size(1));
        assert_eq!(game.grid_size(), GridSize::Large);
        assert!(!game.adjust_grid_size(1));
        assert_eq!(game.grid_size(), GridSize::Large);
        assert!(game.adjust_grid_size(-1));
        assert_eq!(game.grid_size(), GridSize::Classic);
        assert!(game.adjust_grid_size(-1));
        assert_eq!(game.grid_size(), GridSize::Small);
        assert!(!game.adjust_grid_size(-1));
        assert_eq!(game.grid_size(), GridSize::Small);
    }

    #[test]
    fn small_grid_slide() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Small;
        game.grid = make_grid(3, &[2, 2, 0, 0, 4, 0, 0, 0, 2]);
        game.mode = Game2048Mode::Playing;

        let moved = game.slide(Direction::Left);
        assert!(moved);
        assert_eq!(game.cell(0, 0), 4);
        assert_eq!(game.cell(1, 0), 4);
        assert_eq!(game.cell(2, 0), 2);
    }

    #[test]
    fn update_best_score_persists() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.score = 500;
        let mut scores = MemoryHighScoreStore::new();

        game.update_best_score(&mut scores);

        assert_eq!(scores.game2048_best_score(GridSize::Classic), 500);
        assert_eq!(game.best_score(), 500);
    }

    #[test]
    fn enter_game_over_sets_mode_and_persists_score() {
        let mut game = Game2048::default();
        game.grid_size = GridSize::Classic;
        game.grid = make_grid(4, &[2, 4, 2, 4, 4, 2, 4, 2, 2, 4, 8, 16, 32, 64, 128, 256]);
        game.mode = Game2048Mode::Playing;
        game.score = 100;
        let mut scores = MemoryHighScoreStore::new();

        assert!(game.is_game_over());
        game.enter_game_over(&mut scores);
        assert_eq!(game.mode(), Game2048Mode::GameOver);
        assert_eq!(scores.game2048_best_score(GridSize::Classic), 100);
    }
}
