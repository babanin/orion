use crate::config::{
    on_board, wrap_index, wrapped_cell, Cell, Direction, BOARD_CELLS, BOARD_COLS, BOARD_ROWS,
};
use crate::rng::Rng;
use crate::store::HighScoreStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Choosing,
    Playing,
    Paused,
    Over,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeedTier {
    Slow,
    Normal,
    Fast,
    Expert,
}

impl SpeedTier {
    pub const fn index(self) -> usize {
        match self {
            Self::Slow => 0,
            Self::Normal => 1,
            Self::Fast => 2,
            Self::Expert => 3,
        }
    }

    pub const fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Slow,
            1 => Self::Normal,
            2 => Self::Fast,
            3 => Self::Expert,
            _ => Self::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderMode {
    Borders,
    Wrap,
}

impl BorderMode {
    pub const fn index(self) -> usize {
        match self {
            Self::Borders => 0,
            Self::Wrap => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionField {
    Speed,
    Border,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeedConfig {
    pub name: &'static str,
    pub tick_ms: u32,
    pub points: u32,
}

pub const fn speed_config(tier: SpeedTier) -> SpeedConfig {
    match tier {
        SpeedTier::Slow => SpeedConfig {
            name: "SLOW",
            tick_ms: 320,
            points: 1,
        },
        SpeedTier::Normal => SpeedConfig {
            name: "NORMAL",
            tick_ms: 180,
            points: 2,
        },
        SpeedTier::Fast => SpeedConfig {
            name: "FAST",
            tick_ms: 120,
            points: 3,
        },
        SpeedTier::Expert => SpeedConfig {
            name: "EXPERT",
            tick_ms: 80,
            points: 4,
        },
    }
}

pub const fn border_mode_name(mode: BorderMode) -> &'static str {
    match mode {
        BorderMode::Borders => "BORDERS",
        BorderMode::Wrap => "WRAP",
    }
}

#[derive(Debug, Clone)]
pub struct SnakeGame {
    snake: [Cell; BOARD_CELLS],
    length: usize,
    food: Cell,
    direction: Direction,
    pending_direction: Direction,
    mode: GameMode,
    speed_tier: SpeedTier,
    border_mode: BorderMode,
    selected_field: SelectionField,
    score: u32,
    best_score: u32,
    tick_ms: u32,
    last_tick_us: i64,
}

impl Default for SnakeGame {
    fn default() -> Self {
        Self {
            snake: [Cell::new(0, 0); BOARD_CELLS],
            length: 0,
            food: Cell::new(-1, -1),
            direction: Direction::Right,
            pending_direction: Direction::Right,
            mode: GameMode::Choosing,
            speed_tier: SpeedTier::Normal,
            border_mode: BorderMode::Borders,
            selected_field: SelectionField::Speed,
            score: 0,
            best_score: 0,
            tick_ms: speed_config(SpeedTier::Normal).tick_ms,
            last_tick_us: 0,
        }
    }
}

impl SnakeGame {
    pub fn enter_choosing(&mut self, high_scores: &impl HighScoreStore) {
        self.mode = GameMode::Choosing;
        self.selected_field = SelectionField::Speed;
        self.score = 0;
        self.length = 0;
        self.food = Cell::new(-1, -1);
        self.refresh_best_score(high_scores);
    }

    pub fn reset(&mut self, high_scores: &impl HighScoreStore, rng: &mut impl Rng, now_us: i64) {
        self.length = 4;
        self.direction = Direction::Right;
        self.pending_direction = Direction::Right;
        self.mode = GameMode::Playing;
        self.score = 0;
        self.refresh_best_score(high_scores);
        self.tick_ms = speed_config(self.speed_tier).tick_ms;
        self.last_tick_us = now_us;

        let start_x = BOARD_COLS / 2;
        let start_y = BOARD_ROWS / 2;
        for i in 0..self.length {
            self.snake[i] = Cell::new(start_x - i as i16, start_y);
        }
        self.place_food(rng);
    }

    pub fn press_switch(
        &mut self,
        high_scores: &impl HighScoreStore,
        rng: &mut impl Rng,
        now_us: i64,
    ) -> bool {
        match self.mode {
            GameMode::Choosing => {
                self.reset(high_scores, rng, now_us);
                true
            }
            GameMode::Over => {
                self.enter_choosing(high_scores);
                true
            }
            GameMode::Paused => {
                self.mode = GameMode::Playing;
                self.last_tick_us = now_us;
                true
            }
            GameMode::Playing => {
                self.mode = GameMode::Paused;
                true
            }
        }
    }

    pub fn request_direction(&mut self, direction: Direction) {
        if !self.direction.is_opposite(direction) {
            self.pending_direction = direction;
        }
    }

    pub fn select_next_field(&mut self) -> bool {
        let next = match self.selected_field {
            SelectionField::Speed => SelectionField::Border,
            SelectionField::Border => SelectionField::Speed,
        };
        let changed = next != self.selected_field;
        self.selected_field = next;
        changed
    }

    pub fn select_previous_field(&mut self) -> bool {
        self.select_next_field()
    }

    pub fn adjust_selected_value(
        &mut self,
        high_scores: &impl HighScoreStore,
        detents: i32,
    ) -> bool {
        if self.mode != GameMode::Choosing || detents == 0 {
            return false;
        }

        let changed = match self.selected_field {
            SelectionField::Speed => {
                self.set_speed_tier(cycle_speed_tier(self.speed_tier, detents))
            }
            SelectionField::Border => {
                let next = cycle_border_mode(self.border_mode, detents);
                if next == self.border_mode {
                    false
                } else {
                    self.border_mode = next;
                    true
                }
            }
        };
        if changed {
            self.refresh_best_score(high_scores);
        }
        changed
    }

    pub fn adjust_speed(&mut self, high_scores: &impl HighScoreStore, detents: i32) -> bool {
        if self.mode != GameMode::Playing || detents == 0 {
            return false;
        }
        let changed = self.set_speed_tier(cycle_speed_tier(self.speed_tier, detents));
        if changed {
            self.refresh_best_score(high_scores);
        }
        changed
    }

    pub fn tick(&mut self, high_scores: &mut impl HighScoreStore, rng: &mut impl Rng) {
        if self.mode != GameMode::Playing {
            return;
        }

        self.direction = self.pending_direction;
        let mut new_head = self.next_head_for(self.direction);
        if self.border_mode == BorderMode::Wrap {
            new_head = wrapped_cell(new_head);
        } else if !on_board(new_head) {
            self.mode = GameMode::Over;
            return;
        }

        let eating = new_head == self.food;
        let collision_limit = if eating {
            self.length
        } else {
            self.length.saturating_sub(1)
        };
        if self.contains(new_head, collision_limit) {
            self.mode = GameMode::Over;
            return;
        }

        let new_length = (self.length + usize::from(eating)).min(BOARD_CELLS);
        for i in (1..new_length).rev() {
            self.snake[i] = self.snake[i - 1];
        }
        self.snake[0] = new_head;
        self.length = new_length;

        if eating {
            self.score += speed_config(self.speed_tier).points;
            high_scores.update_if_better(self.score, self.speed_tier, self.border_mode);
            self.refresh_best_score(high_scores);
            self.place_food(rng);
        }
    }

    pub fn due_for_tick(&self, now_us: i64) -> bool {
        self.mode == GameMode::Playing && now_us - self.last_tick_us >= self.tick_ms as i64 * 1000
    }

    pub fn mark_ticked(&mut self, now_us: i64) {
        self.last_tick_us = now_us;
    }

    pub const fn mode(&self) -> GameMode {
        self.mode
    }

    pub const fn score(&self) -> u32 {
        self.score
    }

    pub const fn best_score(&self) -> u32 {
        self.best_score
    }

    pub const fn length(&self) -> usize {
        self.length
    }

    pub const fn head(&self) -> Cell {
        self.snake[0]
    }

    pub const fn tail(&self) -> Cell {
        self.snake[self.length - 1]
    }

    pub const fn snake_at(&self, index: usize) -> Cell {
        self.snake[index]
    }

    pub const fn direction(&self) -> Direction {
        self.direction
    }

    pub const fn food(&self) -> Cell {
        self.food
    }

    pub const fn speed_tier(&self) -> SpeedTier {
        self.speed_tier
    }

    pub const fn border_mode(&self) -> BorderMode {
        self.border_mode
    }

    pub const fn selected_field(&self) -> SelectionField {
        self.selected_field
    }

    pub fn set_options(
        &mut self,
        speed: SpeedTier,
        border: BorderMode,
        high_scores: &impl HighScoreStore,
    ) {
        self.speed_tier = speed;
        self.border_mode = border;
        self.tick_ms = speed_config(speed).tick_ms;
        self.refresh_best_score(high_scores);
    }

    fn next_head_for(&self, direction: Direction) -> Cell {
        let mut head = self.snake[0];
        match direction {
            Direction::Up => head.y -= 1,
            Direction::Down => head.y += 1,
            Direction::Left => head.x -= 1,
            Direction::Right => head.x += 1,
        }
        head
    }

    fn set_speed_tier(&mut self, tier: SpeedTier) -> bool {
        if tier == self.speed_tier {
            return false;
        }
        self.speed_tier = tier;
        self.tick_ms = speed_config(tier).tick_ms;
        true
    }

    fn refresh_best_score(&mut self, high_scores: &impl HighScoreStore) {
        self.best_score = high_scores.best_score(self.speed_tier, self.border_mode);
    }

    fn contains(&self, cell: Cell, limit: usize) -> bool {
        self.snake[..limit].contains(&cell)
    }

    fn place_food(&mut self, rng: &mut impl Rng) {
        if self.length >= BOARD_CELLS {
            self.food = Cell::new(-1, -1);
            return;
        }

        loop {
            let food = Cell::new(
                rng.index(BOARD_COLS as usize) as i16,
                rng.index(BOARD_ROWS as usize) as i16,
            );
            if !self.contains(food, self.length) {
                self.food = food;
                return;
            }
        }
    }
}

pub const fn cycle_speed_tier(tier: SpeedTier, detents: i32) -> SpeedTier {
    SpeedTier::from_index(wrap_index(tier.index() as i32 + detents, 4) as usize)
}

pub const fn cycle_border_mode(mode: BorderMode, detents: i32) -> BorderMode {
    if detents == 0 {
        return mode;
    }
    match wrap_index(mode.index() as i32 + detents, 2) {
        0 => BorderMode::Borders,
        _ => BorderMode::Wrap,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::ScriptedRng;
    use crate::store::MemoryHighScoreStore;

    #[test]
    fn reset_starts_in_middle_with_food_off_snake() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 12, 7]);
        let mut game = SnakeGame::default();
        game.reset(&scores, &mut rng, 10);

        assert_eq!(game.mode(), GameMode::Playing);
        assert_eq!(game.length(), 4);
        assert_eq!(game.head(), Cell::new(BOARD_COLS / 2, BOARD_ROWS / 2));
        assert!(!game.snake[..game.length()].contains(&game.food()));
    }

    #[test]
    fn ignores_immediate_reverse_direction() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 12, 7]);
        let mut game = SnakeGame::default();
        game.reset(&scores, &mut rng, 0);
        game.request_direction(Direction::Left);
        game.tick(&mut MemoryHighScoreStore::new(), &mut rng);
        assert_eq!(game.head(), Cell::new(BOARD_COLS / 2 + 1, BOARD_ROWS / 2));
    }

    #[test]
    fn eating_scores_by_current_speed_and_persists_best() {
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([11, 7, 0, 0]);
        let mut game = SnakeGame::default();
        game.set_options(SpeedTier::Expert, BorderMode::Borders, &scores);
        game.reset(&scores, &mut rng, 0);
        game.tick(&mut scores, &mut rng);

        assert_eq!(game.score(), 4);
        assert_eq!(scores.best_score(SpeedTier::Expert, BorderMode::Borders), 4);
    }

    #[test]
    fn wrap_mode_crosses_board_edge() {
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([12, 6, 0, 0]);
        let mut game = SnakeGame::default();
        game.set_options(SpeedTier::Normal, BorderMode::Wrap, &scores);
        game.reset(&scores, &mut rng, 0);
        for _ in 0..11 {
            game.tick(&mut scores, &mut rng);
        }
        assert_eq!(game.mode(), GameMode::Playing);
        assert_eq!(game.head().x, 1);
    }
}
