use crate::rng::Rng;
use crate::store::HighScoreStore;

pub const FLAPPY_OBSTACLE_COUNT: usize = 3;
pub const FLAPPY_PLAYER_X: i16 = 58;
pub const FLAPPY_PLAYER_W: i16 = 18;
pub const FLAPPY_PLAYER_H: i16 = 18;
pub const FLAPPY_PLAY_TOP: i16 = 24;
pub const FLAPPY_FLOOR_Y: i16 = 224;
pub const FLAPPY_OBSTACLE_W: i16 = 24;
pub const FLAPPY_GAP_H: i16 = 78;
pub const FLAPPY_OBSTACLE_SPACING: i16 = 112;
pub const FLAPPY_TICK_US: i64 = 33_000;

const FP_SHIFT: i32 = 8;
const FP_ONE: i32 = 1 << FP_SHIFT;
const START_Y: i16 = 112;
const START_VEL: i32 = 0;
const GRAVITY: i32 = 42;
const FLAP_VELOCITY: i32 = -760;
const SCROLL_SPEED: i16 = 2;
const GAP_MIN_Y: i16 = 44;
const GAP_STEP: i16 = 16;
const GAP_CHOICES: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlappyMode {
    Ready,
    Playing,
    Paused,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlappyPauseAction {
    Continue,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlappyObstacle {
    pub x: i16,
    pub gap_y: i16,
    scored: bool,
}

impl FlappyObstacle {
    pub const fn new(x: i16, gap_y: i16) -> Self {
        Self {
            x,
            gap_y,
            scored: false,
        }
    }

    pub const fn scored(&self) -> bool {
        self.scored
    }
}

#[derive(Debug, Clone)]
pub struct FlappyGame {
    mode: FlappyMode,
    player_y_fp: i32,
    velocity_fp: i32,
    score: u32,
    best_score: u32,
    last_tick_us: i64,
    pause_action: FlappyPauseAction,
    obstacles: [FlappyObstacle; FLAPPY_OBSTACLE_COUNT],
}

impl FlappyGame {
    pub const fn new() -> Self {
        Self {
            mode: FlappyMode::Ready,
            player_y_fp: (START_Y as i32) * FP_ONE,
            velocity_fp: START_VEL,
            score: 0,
            best_score: 0,
            last_tick_us: 0,
            pause_action: FlappyPauseAction::Continue,
            obstacles: [
                FlappyObstacle::new(340, 80),
                FlappyObstacle::new(452, 112),
                FlappyObstacle::new(564, 64),
            ],
        }
    }

    pub fn enter(&mut self, high_scores: &impl HighScoreStore) {
        self.mode = FlappyMode::Ready;
        self.player_y_fp = (START_Y as i32) * FP_ONE;
        self.velocity_fp = START_VEL;
        self.score = 0;
        self.best_score = high_scores.flappy_best_score();
        self.last_tick_us = 0;
        self.pause_action = FlappyPauseAction::Continue;
        self.reset_obstacles();
    }

    pub fn start(&mut self, high_scores: &impl HighScoreStore, rng: &mut impl Rng, now_us: i64) {
        self.mode = FlappyMode::Playing;
        self.player_y_fp = (START_Y as i32) * FP_ONE;
        self.velocity_fp = FLAP_VELOCITY / 2;
        self.score = 0;
        self.best_score = high_scores.flappy_best_score();
        self.last_tick_us = now_us;
        self.pause_action = FlappyPauseAction::Continue;
        self.seed_obstacles(rng);
    }

    pub fn flap(&mut self) -> bool {
        if self.mode != FlappyMode::Playing {
            return false;
        }
        self.velocity_fp = FLAP_VELOCITY;
        true
    }

    pub fn pause(&mut self) {
        if self.mode == FlappyMode::Playing {
            self.mode = FlappyMode::Paused;
            self.pause_action = FlappyPauseAction::Continue;
        }
    }

    pub fn resume(&mut self, now_us: i64) {
        if self.mode == FlappyMode::Paused {
            self.mode = FlappyMode::Playing;
            self.last_tick_us = now_us;
        }
    }

    pub fn cycle_pause_action(&mut self) {
        self.pause_action = match self.pause_action {
            FlappyPauseAction::Continue => FlappyPauseAction::Exit,
            FlappyPauseAction::Exit => FlappyPauseAction::Continue,
        };
    }

    pub fn due_for_tick(&self, now_us: i64) -> bool {
        self.mode == FlappyMode::Playing && now_us - self.last_tick_us >= FLAPPY_TICK_US
    }

    pub fn mark_ticked(&mut self, now_us: i64) {
        self.last_tick_us = now_us;
    }

    pub fn tick(&mut self, high_scores: &mut impl HighScoreStore, rng: &mut impl Rng) {
        if self.mode != FlappyMode::Playing {
            return;
        }

        self.velocity_fp += GRAVITY;
        self.player_y_fp += self.velocity_fp;

        for obstacle in &mut self.obstacles {
            obstacle.x -= SCROLL_SPEED;
        }

        self.recycle_obstacles(rng);
        self.update_score(high_scores);

        if self.collides() {
            self.mode = FlappyMode::GameOver;
            self.update_best_score(high_scores);
        }
    }

    pub fn update_best_score(&mut self, high_scores: &mut impl HighScoreStore) {
        if self.score > self.best_score {
            self.best_score = self.score;
            high_scores.update_flappy_best_score(self.score);
        }
    }

    pub const fn mode(&self) -> FlappyMode {
        self.mode
    }

    pub const fn player_y(&self) -> i16 {
        (self.player_y_fp / FP_ONE) as i16
    }

    pub const fn velocity_fp(&self) -> i32 {
        self.velocity_fp
    }

    pub const fn score(&self) -> u32 {
        self.score
    }

    pub const fn best_score(&self) -> u32 {
        self.best_score
    }

    pub const fn pause_action(&self) -> FlappyPauseAction {
        self.pause_action
    }

    pub const fn obstacles(&self) -> &[FlappyObstacle; FLAPPY_OBSTACLE_COUNT] {
        &self.obstacles
    }

    #[cfg(test)]
    pub fn set_player_y_for_test(&mut self, y: i16) {
        self.player_y_fp = y as i32 * FP_ONE;
    }

    #[cfg(test)]
    pub fn set_obstacle_for_test(&mut self, index: usize, obstacle: FlappyObstacle) {
        self.obstacles[index] = obstacle;
    }

    #[cfg(test)]
    pub fn force_playing_for_test(&mut self, high_scores: &impl HighScoreStore) {
        self.enter(high_scores);
        self.mode = FlappyMode::Playing;
    }

    fn reset_obstacles(&mut self) {
        let mut x = 340;
        for obstacle in &mut self.obstacles {
            obstacle.x = x;
            obstacle.gap_y = 80;
            obstacle.scored = false;
            x += FLAPPY_OBSTACLE_SPACING;
        }
    }

    fn seed_obstacles(&mut self, rng: &mut impl Rng) {
        let mut x = 340;
        for obstacle in &mut self.obstacles {
            obstacle.x = x;
            obstacle.gap_y = random_gap_y(rng);
            obstacle.scored = false;
            x += FLAPPY_OBSTACLE_SPACING;
        }
    }

    fn recycle_obstacles(&mut self, rng: &mut impl Rng) {
        let mut max_x = self.obstacles[0].x;
        for obstacle in &self.obstacles[1..] {
            max_x = max_x.max(obstacle.x);
        }

        for obstacle in &mut self.obstacles {
            if obstacle.x + FLAPPY_OBSTACLE_W < 0 {
                obstacle.x = max_x + FLAPPY_OBSTACLE_SPACING;
                obstacle.gap_y = random_gap_y(rng);
                obstacle.scored = false;
                max_x = obstacle.x;
            }
        }
    }

    fn update_score(&mut self, high_scores: &mut impl HighScoreStore) {
        for obstacle in &mut self.obstacles {
            if !obstacle.scored && FLAPPY_PLAYER_X > obstacle.x + FLAPPY_OBSTACLE_W {
                obstacle.scored = true;
                self.score += 1;
            }
        }
        if self.score > self.best_score {
            self.best_score = self.score;
            high_scores.update_flappy_best_score(self.score);
        }
    }

    fn collides(&self) -> bool {
        let player_y = self.player_y();
        if player_y < FLAPPY_PLAY_TOP || player_y + FLAPPY_PLAYER_H > FLAPPY_FLOOR_Y {
            return true;
        }

        let player_left = FLAPPY_PLAYER_X;
        let player_right = FLAPPY_PLAYER_X + FLAPPY_PLAYER_W;
        let player_top = player_y;
        let player_bottom = player_y + FLAPPY_PLAYER_H;

        for obstacle in &self.obstacles {
            let obstacle_left = obstacle.x;
            let obstacle_right = obstacle.x + FLAPPY_OBSTACLE_W;
            if player_right <= obstacle_left || player_left >= obstacle_right {
                continue;
            }
            let gap_top = obstacle.gap_y;
            let gap_bottom = obstacle.gap_y + FLAPPY_GAP_H;
            if player_top < gap_top || player_bottom > gap_bottom {
                return true;
            }
        }
        false
    }
}

impl Default for FlappyGame {
    fn default() -> Self {
        Self::new()
    }
}

fn random_gap_y(rng: &mut impl Rng) -> i16 {
    GAP_MIN_Y + rng.index(GAP_CHOICES) as i16 * GAP_STEP
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::ScriptedRng;
    use crate::store::MemoryHighScoreStore;

    #[test]
    fn enter_loads_best_score() {
        let mut store = MemoryHighScoreStore::new();
        store.update_flappy_best_score(7);
        let mut game = FlappyGame::new();
        game.enter(&store);
        assert_eq!(game.mode(), FlappyMode::Ready);
        assert_eq!(game.best_score(), 7);
    }

    #[test]
    fn start_seeds_deterministic_gaps() {
        let store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 6]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 10);
        assert_eq!(game.mode(), FlappyMode::Playing);
        assert_eq!(game.obstacles()[0].gap_y, GAP_MIN_Y);
        assert_eq!(game.obstacles()[1].gap_y, GAP_MIN_Y + 2 * GAP_STEP);
        assert_eq!(game.obstacles()[2].gap_y, GAP_MIN_Y + 6 * GAP_STEP);
    }

    #[test]
    fn flap_changes_velocity_only_during_play() {
        let store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        assert!(!game.flap());
        game.start(&store, &mut rng, 0);
        assert!(game.flap());
        assert_eq!(game.velocity_fp(), FLAP_VELOCITY);
    }

    #[test]
    fn gravity_tick_moves_player_down_after_start_velocity() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        let y = game.player_y();
        game.mark_ticked(FLAPPY_TICK_US);
        game.tick(&mut store, &mut rng);
        assert!(game.player_y() < y);
        for _ in 0..24 {
            game.mark_ticked(FLAPPY_TICK_US);
            game.tick(&mut store, &mut rng);
        }
        assert!(game.velocity_fp() > 0);
    }

    #[test]
    fn obstacle_recycles_with_new_gap() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1, 2, 3, 4]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        game.set_obstacle_for_test(0, FlappyObstacle::new(-FLAPPY_OBSTACLE_W - 1, 80));
        game.tick(&mut store, &mut rng);
        assert_eq!(
            game.obstacles()[0].x,
            564 + FLAPPY_OBSTACLE_SPACING - SCROLL_SPEED
        );
        assert_eq!(game.obstacles()[0].gap_y, GAP_MIN_Y + 4 * GAP_STEP);
    }

    #[test]
    fn scores_once_when_obstacle_is_passed() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        game.set_obstacle_for_test(
            0,
            FlappyObstacle::new(FLAPPY_PLAYER_X - FLAPPY_OBSTACLE_W - 1, 80),
        );
        game.tick(&mut store, &mut rng);
        assert_eq!(game.score(), 1);
        game.tick(&mut store, &mut rng);
        assert_eq!(game.score(), 1);
    }

    #[test]
    fn candle_collision_ends_game() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        game.set_player_y_for_test(50);
        game.set_obstacle_for_test(0, FlappyObstacle::new(FLAPPY_PLAYER_X, 80));
        game.tick(&mut store, &mut rng);
        assert_eq!(game.mode(), FlappyMode::GameOver);
    }

    #[test]
    fn jelly_collision_ends_game() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        game.set_player_y_for_test(160);
        game.set_obstacle_for_test(0, FlappyObstacle::new(FLAPPY_PLAYER_X, 80));
        game.tick(&mut store, &mut rng);
        assert_eq!(game.mode(), FlappyMode::GameOver);
    }

    #[test]
    fn floor_and_ceiling_collisions_end_game() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut floor_game = FlappyGame::new();
        floor_game.start(&store, &mut rng, 0);
        floor_game.set_player_y_for_test(FLAPPY_FLOOR_Y);
        floor_game.tick(&mut store, &mut rng);
        assert_eq!(floor_game.mode(), FlappyMode::GameOver);

        let mut ceiling_game = FlappyGame::new();
        ceiling_game.start(&store, &mut rng, 0);
        ceiling_game.set_player_y_for_test(FLAPPY_PLAY_TOP - 1);
        ceiling_game.tick(&mut store, &mut rng);
        assert_eq!(ceiling_game.mode(), FlappyMode::GameOver);
    }

    #[test]
    fn updates_persisted_best_score() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        game.set_obstacle_for_test(
            0,
            FlappyObstacle::new(FLAPPY_PLAYER_X - FLAPPY_OBSTACLE_W - 1, 80),
        );
        game.tick(&mut store, &mut rng);
        assert_eq!(game.best_score(), 1);
        assert_eq!(store.flappy_best_score(), 1);
    }

    #[test]
    fn pause_continue_round_trip() {
        let store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        game.pause();
        assert_eq!(game.mode(), FlappyMode::Paused);
        game.resume(10);
        assert_eq!(game.mode(), FlappyMode::Playing);
    }
}
