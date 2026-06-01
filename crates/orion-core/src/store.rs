use crate::game2048::GridSize;
use crate::snake::{BorderMode, SpeedTier};

pub const HIGH_SCORE_BUCKET_COUNT: usize = 8;
pub const GAME2048_SCORE_BUCKET_COUNT: usize = 3;

pub trait HighScoreStore {
    fn best_score(&self, speed: SpeedTier, border: BorderMode) -> u32;
    fn update_if_better(&mut self, score: u32, speed: SpeedTier, border: BorderMode);
    fn flags_death_match_best_score(&self) -> u32;
    fn update_flags_death_match_best_score(&mut self, score: u32);
    fn game2048_best_score(&self, grid_size: GridSize) -> u32;
    fn update_game2048_best_score(&mut self, score: u32, grid_size: GridSize);
}

#[derive(Debug, Clone)]
pub struct MemoryHighScoreStore {
    snake: [u32; HIGH_SCORE_BUCKET_COUNT],
    flags_death_match: u32,
    game2048: [u32; GAME2048_SCORE_BUCKET_COUNT],
}

impl MemoryHighScoreStore {
    pub const fn new() -> Self {
        Self {
            snake: [0; HIGH_SCORE_BUCKET_COUNT],
            flags_death_match: 0,
            game2048: [0; GAME2048_SCORE_BUCKET_COUNT],
        }
    }
}

impl Default for MemoryHighScoreStore {
    fn default() -> Self {
        Self::new()
    }
}

impl HighScoreStore for MemoryHighScoreStore {
    fn best_score(&self, speed: SpeedTier, border: BorderMode) -> u32 {
        self.snake[high_score_index(speed, border)]
    }

    fn update_if_better(&mut self, score: u32, speed: SpeedTier, border: BorderMode) {
        let index = high_score_index(speed, border);
        if score > self.snake[index] {
            self.snake[index] = score;
        }
    }

    fn flags_death_match_best_score(&self) -> u32 {
        self.flags_death_match
    }

    fn update_flags_death_match_best_score(&mut self, score: u32) {
        if score > self.flags_death_match {
            self.flags_death_match = score;
        }
    }

    fn game2048_best_score(&self, grid_size: GridSize) -> u32 {
        self.game2048[grid_size.index()]
    }

    fn update_game2048_best_score(&mut self, score: u32, grid_size: GridSize) {
        if score > self.game2048[grid_size.index()] {
            self.game2048[grid_size.index()] = score;
        }
    }
}

pub const fn high_score_index(speed: SpeedTier, border: BorderMode) -> usize {
    speed.index() * 2 + border.index()
}

pub const fn high_score_key(index: usize) -> &'static str {
    match index {
        0 => "bs_slow_b",
        1 => "bs_slow_w",
        2 => "bs_norm_b",
        3 => "bs_norm_w",
        4 => "bs_fast_b",
        5 => "bs_fast_w",
        6 => "bs_expr_b",
        7 => "bs_expr_w",
        _ => "",
    }
}

pub const fn game2048_score_key(index: usize) -> &'static str {
    match index {
        0 => "best_3x3",
        1 => "best_4x4",
        2 => "best_5x5",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game2048::GridSize;
    use crate::snake::{BorderMode, SpeedTier};

    #[test]
    fn keys_match_existing_nvs_contract() {
        let keys: Vec<_> = (0..HIGH_SCORE_BUCKET_COUNT).map(high_score_key).collect();
        assert_eq!(
            keys,
            vec![
                "bs_slow_b",
                "bs_slow_w",
                "bs_norm_b",
                "bs_norm_w",
                "bs_fast_b",
                "bs_fast_w",
                "bs_expr_b",
                "bs_expr_w"
            ]
        );
    }

    #[test]
    fn game2048_keys() {
        assert_eq!(game2048_score_key(0), "best_3x3");
        assert_eq!(game2048_score_key(1), "best_4x4");
        assert_eq!(game2048_score_key(2), "best_5x5");
    }

    #[test]
    fn update_only_keeps_better_scores() {
        let mut store = MemoryHighScoreStore::new();
        store.update_if_better(4, SpeedTier::Fast, BorderMode::Wrap);
        store.update_if_better(3, SpeedTier::Fast, BorderMode::Wrap);
        assert_eq!(store.best_score(SpeedTier::Fast, BorderMode::Wrap), 4);
    }

    #[test]
    fn game2048_best_score_per_grid_size() {
        let mut store = MemoryHighScoreStore::new();
        store.update_game2048_best_score(100, GridSize::Classic);
        store.update_game2048_best_score(50, GridSize::Small);
        assert_eq!(store.game2048_best_score(GridSize::Classic), 100);
        assert_eq!(store.game2048_best_score(GridSize::Small), 50);
        assert_eq!(store.game2048_best_score(GridSize::Large), 0);

        store.update_game2048_best_score(200, GridSize::Classic);
        assert_eq!(store.game2048_best_score(GridSize::Classic), 200);

        store.update_game2048_best_score(150, GridSize::Classic);
        assert_eq!(store.game2048_best_score(GridSize::Classic), 200);
    }
}
