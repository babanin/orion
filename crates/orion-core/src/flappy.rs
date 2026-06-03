use crate::rng::Rng;
use crate::store::HighScoreStore;

pub const FLAPPY_OBSTACLE_COUNT: usize = 3;
pub const FLAPPY_PLAYER_X: i16 = 58;
pub const FLAPPY_PLAYER_W: i16 = 20;
pub const FLAPPY_PLAYER_H: i16 = 20;
pub const FLAPPY_PLAY_TOP: i16 = 24;
pub const FLAPPY_FLOOR_Y: i16 = 224;
pub const FLAPPY_OBSTACLE_W: i16 = 24;
pub const FLAPPY_GAP_H: i16 = 78;
pub const FLAPPY_OBSTACLE_SPACING: i16 = 112;
pub const FLAPPY_TICK_US: i64 = 33_000;
pub const FLAPPY_INITIAL_LIVES: u32 = 3;
pub const FLAPPY_INVINCIBLE_TICKS: u8 = 45;

const FP_SHIFT: i32 = 8;
const FP_ONE: i32 = 1 << FP_SHIFT;
const START_Y: i16 = 112;
const START_VEL: i32 = 0;
const GRAVITY: i32 = 42;
const FLAP_VELOCITY: i32 = -760;
const BASE_SCROLL_SPEED: i16 = 2;
const SCORE_PER_SPEED_STEP: u32 = 10;
const EXTRA_LIFE_SCORE_STEP: u32 = 20;
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlappyTickOutcome {
    pub life_lost: bool,
}

#[derive(Debug, Clone)]
pub struct FlappyGame {
    mode: FlappyMode,
    player_y_fp: i32,
    velocity_fp: i32,
    score: u32,
    best_score: u32,
    lives: u32,
    next_extra_life_score: u32,
    invincible_ticks_remaining: u8,
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
            lives: FLAPPY_INITIAL_LIVES,
            next_extra_life_score: EXTRA_LIFE_SCORE_STEP,
            invincible_ticks_remaining: 0,
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
        self.lives = FLAPPY_INITIAL_LIVES;
        self.next_extra_life_score = EXTRA_LIFE_SCORE_STEP;
        self.invincible_ticks_remaining = 0;
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
        self.lives = FLAPPY_INITIAL_LIVES;
        self.next_extra_life_score = EXTRA_LIFE_SCORE_STEP;
        self.invincible_ticks_remaining = 0;
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

    pub fn tick(
        &mut self,
        high_scores: &mut impl HighScoreStore,
        rng: &mut impl Rng,
    ) -> FlappyTickOutcome {
        if self.mode != FlappyMode::Playing {
            return FlappyTickOutcome::default();
        }

        let invincible_before_tick = self.invincible_ticks_remaining;
        let mut outcome = FlappyTickOutcome::default();

        self.velocity_fp += GRAVITY;
        self.player_y_fp += self.velocity_fp;

        let scroll_speed = scroll_speed_for_score(self.score);
        for obstacle in &mut self.obstacles {
            obstacle.x -= scroll_speed;
        }

        self.recycle_obstacles(rng);
        self.update_score(high_scores);

        if self.collides() {
            outcome.life_lost = self.handle_collision(high_scores);
        }
        if self.mode == FlappyMode::Playing && invincible_before_tick > 0 {
            self.invincible_ticks_remaining = self.invincible_ticks_remaining.saturating_sub(1);
        }
        outcome
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

    pub const fn lives(&self) -> u32 {
        self.lives
    }

    pub const fn invincible_ticks_remaining(&self) -> u8 {
        self.invincible_ticks_remaining
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
    pub fn set_score_for_test(&mut self, score: u32) {
        self.score = score;
    }

    #[cfg(test)]
    pub fn set_motion_for_test(&mut self, y: i16, velocity_fp: i32) {
        self.player_y_fp = y as i32 * FP_ONE;
        self.velocity_fp = velocity_fp;
    }

    #[cfg(test)]
    pub fn set_lives_for_test(&mut self, lives: u32) {
        self.lives = lives;
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
        let mut scored = false;
        for obstacle in &mut self.obstacles {
            if !obstacle.scored && FLAPPY_PLAYER_X > obstacle.x + FLAPPY_OBSTACLE_W {
                obstacle.scored = true;
                self.score += 1;
                scored = true;
            }
        }
        if scored {
            self.award_extra_lives();
        }
        if self.score > self.best_score {
            self.best_score = self.score;
            high_scores.update_flappy_best_score(self.score);
        }
    }

    fn award_extra_lives(&mut self) {
        while self.score >= self.next_extra_life_score {
            self.lives = self.lives.saturating_add(1);
            if let Some(next_score) = self
                .next_extra_life_score
                .checked_add(EXTRA_LIFE_SCORE_STEP)
            {
                self.next_extra_life_score = next_score;
            } else {
                break;
            }
        }
    }

    fn handle_collision(&mut self, high_scores: &mut impl HighScoreStore) -> bool {
        if self.invincible_ticks_remaining > 0 {
            return false;
        }

        self.lives = self.lives.saturating_sub(1);
        if self.lives == 0 {
            self.mode = FlappyMode::GameOver;
            self.update_best_score(high_scores);
        } else {
            self.reset_player_after_hit();
            self.invincible_ticks_remaining = FLAPPY_INVINCIBLE_TICKS;
        }
        true
    }

    fn reset_player_after_hit(&mut self) {
        self.player_y_fp = (START_Y as i32) * FP_ONE;
        self.velocity_fp = FLAP_VELOCITY / 2;
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

fn scroll_speed_for_score(score: u32) -> i16 {
    let speed_bonus = (score / SCORE_PER_SPEED_STEP).min((i16::MAX - BASE_SCROLL_SPEED) as u32);
    BASE_SCROLL_SPEED + speed_bonus as i16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::ScriptedRng;
    use crate::store::MemoryHighScoreStore;

    fn score_one(game: &mut FlappyGame, store: &mut MemoryHighScoreStore, rng: &mut ScriptedRng) {
        game.set_motion_for_test(100, 0);
        game.set_obstacle_for_test(
            0,
            FlappyObstacle::new(FLAPPY_PLAYER_X - FLAPPY_OBSTACLE_W - 1, 80),
        );
        game.set_obstacle_for_test(1, FlappyObstacle::new(240, 80));
        game.set_obstacle_for_test(2, FlappyObstacle::new(300, 80));
        game.tick(store, rng);
    }

    #[test]
    fn enter_loads_best_score() {
        let mut store = MemoryHighScoreStore::new();
        store.update_flappy_best_score(7);
        let mut game = FlappyGame::new();
        game.enter(&store);
        assert_eq!(game.mode(), FlappyMode::Ready);
        assert_eq!(game.best_score(), 7);
        assert_eq!(game.lives(), FLAPPY_INITIAL_LIVES);
    }

    #[test]
    fn start_resets_lives_and_seeds_deterministic_gaps() {
        let store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 6]);
        let mut game = FlappyGame::new();
        game.set_lives_for_test(9);
        game.start(&store, &mut rng, 10);
        assert_eq!(game.mode(), FlappyMode::Playing);
        assert_eq!(game.lives(), FLAPPY_INITIAL_LIVES);
        assert_eq!(game.invincible_ticks_remaining(), 0);
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
            564 + FLAPPY_OBSTACLE_SPACING - scroll_speed_for_score(0)
        );
        assert_eq!(game.obstacles()[0].gap_y, GAP_MIN_Y + 4 * GAP_STEP);
    }

    #[test]
    fn scroll_speed_increases_every_ten_scores() {
        assert_eq!(scroll_speed_for_score(0), 2);
        assert_eq!(scroll_speed_for_score(9), 2);
        assert_eq!(scroll_speed_for_score(10), 3);
        assert_eq!(scroll_speed_for_score(19), 3);
        assert_eq!(scroll_speed_for_score(20), 4);
    }

    #[test]
    fn tick_moves_obstacles_faster_after_ten_scores() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        game.set_score_for_test(10);
        game.set_obstacle_for_test(0, FlappyObstacle::new(200, 80));

        game.tick(&mut store, &mut rng);

        assert_eq!(game.obstacles()[0].x, 200 - scroll_speed_for_score(10));
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
    fn extra_life_awards_at_twenty_and_forty_scores() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);

        for _ in 0..19 {
            score_one(&mut game, &mut store, &mut rng);
        }
        assert_eq!(game.score(), 19);
        assert_eq!(game.lives(), 3);

        score_one(&mut game, &mut store, &mut rng);
        assert_eq!(game.score(), 20);
        assert_eq!(game.lives(), 4);

        for _ in 20..39 {
            score_one(&mut game, &mut store, &mut rng);
        }
        score_one(&mut game, &mut store, &mut rng);
        assert_eq!(game.score(), 40);
        assert_eq!(game.lives(), 5);
    }

    #[test]
    fn extra_life_does_not_duplicate_at_same_milestone() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);

        for _ in 0..20 {
            score_one(&mut game, &mut store, &mut rng);
        }
        assert_eq!(game.lives(), 4);

        game.set_motion_for_test(100, 0);
        game.set_obstacle_for_test(0, FlappyObstacle::new(200, 80));
        game.tick(&mut store, &mut rng);
        assert_eq!(game.score(), 20);
        assert_eq!(game.lives(), 4);
    }

    #[test]
    fn extra_lives_continue_without_low_cap() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);

        for _ in 0..100 {
            score_one(&mut game, &mut store, &mut rng);
        }

        assert_eq!(game.score(), 100);
        assert_eq!(game.lives(), 8);
    }

    #[test]
    fn candle_collision_costs_one_life_and_continues() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        game.set_player_y_for_test(50);
        game.set_obstacle_for_test(0, FlappyObstacle::new(FLAPPY_PLAYER_X, 80));
        let outcome = game.tick(&mut store, &mut rng);
        assert_eq!(outcome, FlappyTickOutcome { life_lost: true });
        assert_eq!(game.mode(), FlappyMode::Playing);
        assert_eq!(game.lives(), 2);
        assert_eq!(game.player_y(), START_Y);
        assert_eq!(game.velocity_fp(), FLAP_VELOCITY / 2);
        assert_eq!(game.invincible_ticks_remaining(), FLAPPY_INVINCIBLE_TICKS);
    }

    #[test]
    fn jelly_collision_costs_one_life_and_continues() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        game.set_player_y_for_test(160);
        game.set_obstacle_for_test(0, FlappyObstacle::new(FLAPPY_PLAYER_X, 80));
        game.tick(&mut store, &mut rng);
        assert_eq!(game.mode(), FlappyMode::Playing);
        assert_eq!(game.lives(), 2);
    }

    #[test]
    fn floor_and_ceiling_collisions_cost_one_life() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut floor_game = FlappyGame::new();
        floor_game.start(&store, &mut rng, 0);
        floor_game.set_player_y_for_test(FLAPPY_FLOOR_Y);
        floor_game.tick(&mut store, &mut rng);
        assert_eq!(floor_game.mode(), FlappyMode::Playing);
        assert_eq!(floor_game.lives(), 2);

        let mut ceiling_game = FlappyGame::new();
        ceiling_game.start(&store, &mut rng, 0);
        ceiling_game.set_player_y_for_test(FLAPPY_PLAY_TOP - 1);
        ceiling_game.tick(&mut store, &mut rng);
        assert_eq!(ceiling_game.mode(), FlappyMode::Playing);
        assert_eq!(ceiling_game.lives(), 2);
    }

    #[test]
    fn non_final_collision_keeps_score_and_scrolled_obstacles() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        game.set_score_for_test(7);
        game.set_player_y_for_test(FLAPPY_FLOOR_Y);
        let before = *game.obstacles();

        game.tick(&mut store, &mut rng);

        assert_eq!(game.score(), 7);
        assert_eq!(
            game.obstacles()[0].x,
            before[0].x - scroll_speed_for_score(7)
        );
        assert_eq!(game.obstacles()[0].gap_y, before[0].gap_y);
        assert_eq!(game.mode(), FlappyMode::Playing);
    }

    #[test]
    fn invincibility_prevents_repeated_life_loss() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        game.set_player_y_for_test(FLAPPY_FLOOR_Y);
        game.tick(&mut store, &mut rng);
        assert_eq!(game.lives(), 2);

        game.set_player_y_for_test(FLAPPY_FLOOR_Y);
        let outcome = game.tick(&mut store, &mut rng);

        assert_eq!(outcome, FlappyTickOutcome { life_lost: false });
        assert_eq!(game.lives(), 2);
        assert_eq!(
            game.invincible_ticks_remaining(),
            FLAPPY_INVINCIBLE_TICKS - 1
        );
    }

    #[test]
    fn final_collision_enters_game_over() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        game.set_lives_for_test(1);
        game.set_player_y_for_test(FLAPPY_FLOOR_Y);

        let outcome = game.tick(&mut store, &mut rng);

        assert_eq!(outcome, FlappyTickOutcome { life_lost: true });
        assert_eq!(game.mode(), FlappyMode::GameOver);
        assert_eq!(game.lives(), 0);
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
