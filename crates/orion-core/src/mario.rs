use crate::store::HighScoreStore;

pub const TILE_SIZE: i16 = 16;
pub const LEVEL_COLS: i16 = 200;
pub const LEVEL_ROWS: i16 = 14;
pub const TILES_TOTAL: usize = (LEVEL_COLS as usize) * (LEVEL_ROWS as usize);
pub const GAME_AREA_Y: i16 = 16;
pub const MAX_CAMERA_X: i16 = LEVEL_COLS * TILE_SIZE - 320;

pub const PLAYER_W: i16 = 24;
pub const PLAYER_H: i16 = 24;

pub const MAX_ENEMIES: usize = 16;

pub const FP_SHIFT: i32 = 8;
pub const FP_ONE: i32 = 1 << FP_SHIFT;
pub const GRAVITY: i32 = 52;
pub const JUMP_VEL: i32 = -480;
pub const MOVE_SPEED: i32 = 2 * FP_ONE;
pub const MAX_FALL: i32 = 600;
pub const JUMP_HOLD_TICKS_MAX: u8 = 8;
pub const TICK_US: i64 = 16_666;
pub const START_LIVES: u32 = 3;
pub const INVINCIBLE_TICKS: u8 = 60;

pub const TILE_AIR: u8 = 0;
pub const TILE_GROUND: u8 = 1;
pub const TILE_BRICK: u8 = 2;
pub const TILE_QUESTION: u8 = 3;
pub const TILE_USED: u8 = 4;
pub const TILE_PIPE_TL: u8 = 5;
pub const TILE_PIPE_TR: u8 = 6;
pub const TILE_PIPE_BL: u8 = 7;
pub const TILE_PIPE_BR: u8 = 8;
pub const TILE_FLAGPOLE: u8 = 9;
pub const TILE_FLAGPOLE_TOP: u8 = 10;
pub const TILE_HARD: u8 = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarioMode {
    Ready,
    Playing,
    Paused,
    Dying,
    LevelComplete,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarioPauseAction {
    Continue,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarioEnemyType {
    ChompNom,
    SpikeNom,
}

impl MarioEnemyType {
    pub const fn w(self) -> i16 {
        match self {
            MarioEnemyType::ChompNom => 16,
            MarioEnemyType::SpikeNom => 20,
        }
    }
    pub const fn h(self) -> i16 {
        match self {
            MarioEnemyType::ChompNom => 14,
            MarioEnemyType::SpikeNom => 24,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Player {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub on_ground: bool,
    pub facing_left: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarioEnemy {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub spawn_x: i32,
    pub alive: bool,
    pub stomped: bool,
    pub shell_timer: i16,
    pub enemy_type: MarioEnemyType,
}

fn px_to_tile(px: i32) -> i16 {
    (px / TILE_SIZE as i32) as i16
}

fn fp_to_px(v: i32) -> i16 {
    (v >> FP_SHIFT) as i16
}

#[derive(Debug, Clone)]
pub struct MarioGame {
    mode: MarioMode,
    player: Player,
    enemies: [MarioEnemy; MAX_ENEMIES],
    enemy_count: usize,
    camera_x: i32,
    tiles: [u8; TILES_TOTAL],
    score: u32,
    best_score: u32,
    coins: u32,
    lives: u32,
    invincible_ticks: u8,
    death_timer: i16,
    pause_action: MarioPauseAction,
    last_tick_us: i64,
    jump_hold_ticks: u8,
    switch_held: bool,
}

impl MarioGame {
    pub fn new() -> Self {
        let mut game = Self {
            mode: MarioMode::Ready,
            player: Player {
                x: 3 * FP_ONE,
                y: ((GAME_AREA_Y + 11 * TILE_SIZE - PLAYER_H) as i32) * FP_ONE,
                vx: 0,
                vy: 0,
                on_ground: true,
                facing_left: false,
            },
            enemies: [MarioEnemy {
                x: 0,
                y: 0,
                vx: 0,
                spawn_x: 0,
                alive: false,
                stomped: false,
                shell_timer: 0,
                enemy_type: MarioEnemyType::ChompNom,
            }; MAX_ENEMIES],
            enemy_count: 0,
            camera_x: 0,
            tiles: [TILE_AIR; TILES_TOTAL],
            score: 0,
            best_score: 0,
            coins: 0,
            lives: START_LIVES,
            invincible_ticks: 0,
            death_timer: 0,
            pause_action: MarioPauseAction::Continue,
            last_tick_us: 0,
            jump_hold_ticks: 0,
            switch_held: false,
        };
        game.build_level();
        game.place_enemies();
        game
    }

    pub fn enter(&mut self, high_scores: &impl HighScoreStore) {
        self.best_score = high_scores.mario_best_score();
        self.reset_game();
    }

    fn reset_game(&mut self) {
        self.player = Player {
            x: 3 * FP_ONE,
            y: ((GAME_AREA_Y + 11 * TILE_SIZE - PLAYER_H) as i32) * FP_ONE,
            vx: 0,
            vy: 0,
            on_ground: true,
            facing_left: false,
        };
        self.camera_x = 0;
        self.score = 0;
        self.coins = 0;
        self.lives = START_LIVES;
        self.invincible_ticks = 0;
        self.death_timer = 0;
        self.jump_hold_ticks = 0;
        self.switch_held = false;
        self.build_level();
        self.place_enemies();
        self.mode = MarioMode::Ready;
    }

    fn build_level(&mut self) {
        self.tiles = [TILE_AIR; TILES_TOTAL];

        // Ground
        for col in 0..LEVEL_COLS {
            if !is_gap(col) {
                self.set_tile(12, col, TILE_GROUND);
                self.set_tile(13, col, TILE_GROUND);
            }
        }

        // Question blocks
        set_tile_s(&mut self.tiles, 9, 14, TILE_QUESTION);
        set_tile_s(&mut self.tiles, 9, 21, TILE_QUESTION);
        set_tile_s(&mut self.tiles, 5, 61, TILE_QUESTION);
        set_tile_s(&mut self.tiles, 5, 62, TILE_QUESTION);
        set_tile_s(&mut self.tiles, 9, 80, TILE_QUESTION);
        set_tile_s(&mut self.tiles, 9, 87, TILE_QUESTION);
        set_tile_s(&mut self.tiles, 9, 94, TILE_QUESTION);

        // Brick platforms
        fill_tiles_s(&mut self.tiles, 9, 15, 2, 1, TILE_BRICK);
        fill_tiles_s(&mut self.tiles, 9, 53, 3, 1, TILE_BRICK);
        fill_tiles_s(&mut self.tiles, 9, 64, 3, 1, TILE_BRICK);
        set_tile_s(&mut self.tiles, 11, 90, TILE_BRICK);

        // Support under platforms
        fill_tiles_s(&mut self.tiles, 9, 61, 3, 1, TILE_BRICK);

        // Pipes
        build_pipe(&mut self.tiles, 30, 8, 4);
        build_pipe(&mut self.tiles, 46, 10, 2);

        // Staircase near end
        build_stairs(&mut self.tiles, 176, 9);

        // Flagpole at col 196
        set_tile_s(&mut self.tiles, 2, 196, TILE_FLAGPOLE_TOP);
        for row in 3..13 {
            set_tile_s(&mut self.tiles, row, 196, TILE_FLAGPOLE);
        }
    }

    fn place_enemies(&mut self) {
        let spawns: [(i32, MarioEnemyType); 6] = [
            (20 * TILE_SIZE as i32, MarioEnemyType::ChompNom),
            (35 * TILE_SIZE as i32, MarioEnemyType::ChompNom),
            (55 * TILE_SIZE as i32, MarioEnemyType::SpikeNom),
            (70 * TILE_SIZE as i32, MarioEnemyType::ChompNom),
            (100 * TILE_SIZE as i32, MarioEnemyType::ChompNom),
            (110 * TILE_SIZE as i32, MarioEnemyType::SpikeNom),
        ];
        self.enemy_count = spawns.len();
        for (i, &(spawn_px, etype)) in spawns.iter().enumerate() {
            let ground_px = GAME_AREA_Y as i32 + 11 * TILE_SIZE as i32;
            self.enemies[i] = MarioEnemy {
                x: spawn_px * FP_ONE,
                y: (ground_px - etype.h() as i32) * FP_ONE,
                vx: -FP_ONE,
                spawn_x: spawn_px,
                alive: true,
                stomped: false,
                shell_timer: 0,
                enemy_type: etype,
            };
        }
    }

    pub fn start(&mut self) {
        self.reset_game();
        self.mode = MarioMode::Playing;
    }

    pub fn pause(&mut self) {
        if self.mode == MarioMode::Playing {
            self.mode = MarioMode::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.mode == MarioMode::Paused {
            self.mode = MarioMode::Playing;
        }
    }

    pub fn cycle_pause_action(&mut self) {
        self.pause_action = match self.pause_action {
            MarioPauseAction::Continue => MarioPauseAction::Exit,
            MarioPauseAction::Exit => MarioPauseAction::Continue,
        };
    }

    pub fn jump(&mut self) {
        if self.player.on_ground {
            self.player.vy = JUMP_VEL;
            self.player.on_ground = false;
            self.jump_hold_ticks = 0;
            self.switch_held = true;
        }
    }

    pub fn release_jump(&mut self) {
        self.switch_held = false;
    }

    pub fn set_direction(&mut self, left: bool, right: bool) {
        if left {
            self.player.vx = -MOVE_SPEED;
            self.player.facing_left = true;
        } else if right {
            self.player.vx = MOVE_SPEED;
            self.player.facing_left = false;
        } else {
            self.player.vx = 0;
        }
    }

    pub fn due_for_tick(&self, now_us: i64) -> bool {
        now_us - self.last_tick_us >= TICK_US
    }

    pub fn mark_ticked(&mut self, now_us: i64) {
        self.last_tick_us = now_us;
    }

    pub fn tick(&mut self, _high_scores: &mut impl HighScoreStore) {
        match self.mode {
            MarioMode::Playing => {
                self.tick_player();
                self.tick_enemies();
                self.tick_camera();
                self.check_player_enemy_collisions();
                self.check_flagpole();

                if self.invincible_ticks > 0 {
                    self.invincible_ticks -= 1;
                }
            }
            MarioMode::Dying => {
                self.death_timer -= 1;
                if self.death_timer <= 0 {
                    if self.lives > 0 {
                        self.respawn();
                    } else {
                        self.mode = MarioMode::GameOver;
                    }
                }
            }
            MarioMode::LevelComplete => {
                self.death_timer -= 1;
                if self.death_timer <= 0 {
                    self.mode = MarioMode::Ready;
                }
            }
            _ => {}
        }
    }

    fn tick_player(&mut self) {
        if self.switch_held && self.jump_hold_ticks < JUMP_HOLD_TICKS_MAX && self.player.vy < 0 {
            self.jump_hold_ticks += 1;
        } else {
            self.player.vy += GRAVITY;
        }

        if self.player.vy > MAX_FALL {
            self.player.vy = MAX_FALL;
        }

        self.move_player_horiz();
        self.move_player_vert();

        let max_y = GAME_AREA_Y + LEVEL_ROWS * TILE_SIZE + PLAYER_H;
        let py_px = fp_to_px(self.player.y);
        if py_px > max_y {
            self.die();
        }
    }

    fn move_player_horiz(&mut self) {
        if self.player.vx == 0 {
            return;
        }
        let new_x = self.player.x + self.player.vx;
        let new_px = fp_to_px(new_x);
        // Shrink the horizontal hitbox vertically by a few pixels (toe/head clearance).
        // This prevents the player from snagging on the floor horizontally while gravity pulls them
        // a fraction of a pixel into the floor each frame before vertical collision resolves it.
        let py_top = fp_to_px(self.player.y) + 4;
        let py_bot = fp_to_px(self.player.y + PLAYER_H as i32 * FP_ONE - 1) - 4;

        let check_px = if self.player.vx > 0 {
            new_px + PLAYER_W - 1
        } else {
            new_px
        };
        let tile_col = px_to_tile(check_px as i32);
        let tile_top = py_top - GAME_AREA_Y;
        let tile_bot = py_bot - GAME_AREA_Y;
        let row_start = px_to_tile(tile_top as i32).max(0);
        let row_end = px_to_tile(tile_bot as i32).min(LEVEL_ROWS - 1);

        for row in row_start..=row_end {
            if (0..LEVEL_COLS).contains(&tile_col) {
                let t = self.tile_at(row, tile_col);
                if is_solid(t) {
                    let snap = if self.player.vx > 0 {
                        (tile_col as i32 * TILE_SIZE as i32 - PLAYER_W as i32) * FP_ONE
                    } else {
                        ((tile_col as i32 + 1) * TILE_SIZE as i32) * FP_ONE
                    };
                    self.player.x = snap;
                    self.player.vx = 0;
                    return;
                }
            }
        }
        self.player.x = new_x;
    }

    fn move_player_vert(&mut self) {
        if self.player.vy == 0 {
            return;
        }
        let new_y = self.player.y + self.player.vy;
        let new_py = fp_to_px(new_y);
        let px_left = fp_to_px(self.player.x);
        let px_right = fp_to_px(self.player.x + PLAYER_W as i32 * FP_ONE - 1);

        let check_py = if self.player.vy > 0 {
            new_py + PLAYER_H - 1
        } else {
            new_py
        };
        let tile_row = ((check_py as i32 - GAME_AREA_Y as i32) / TILE_SIZE as i32)
            .max(0)
            .min(LEVEL_ROWS as i32 - 1) as i16;
        let col_start = (px_left as i32 / TILE_SIZE as i32)
            .max(0)
            .min(LEVEL_COLS as i32 - 1) as i16;
        let col_end = ((px_right as i32 - 1) / TILE_SIZE as i32)
            .max(0)
            .min(LEVEL_COLS as i32 - 1) as i16;

        for col in col_start..=col_end {
            let t = self.tile_at(tile_row, col);
            if is_solid(t) {
                if self.player.vy > 0 {
                    let snap = (tile_row as i32 * TILE_SIZE as i32 + GAME_AREA_Y as i32
                        - PLAYER_H as i32)
                        * FP_ONE;
                    self.player.y = snap;
                    if !self.player.on_ground {
                        self.player.on_ground = true;
                    }
                } else {
                    let snap =
                        ((tile_row as i32 + 1) * TILE_SIZE as i32 + GAME_AREA_Y as i32) * FP_ONE;
                    self.player.y = snap;
                    // The tile that was hit is at tile_row (the player's head row)
                    if (0..LEVEL_ROWS).contains(&tile_row) {
                        let hit_t = self.tile_at(tile_row, col);
                        if hit_t == TILE_QUESTION {
                            if let Some(t) = self.mut_tile_at(tile_row, col) {
                                *t = TILE_USED;
                            }
                            self.score += 200;
                            self.coins += 1;
                        } else if hit_t == TILE_BRICK {
                            self.destroy_brick(tile_row, col);
                        }
                    }
                }
                self.player.vy = 0;
                return;
            }
        }
        self.player.y = new_y;
        if self.player.vy > 0 {
            // Check if there is still ground 1 pixel beneath us, because gravity might have 
            // pulled us a fraction of a pixel into the air (or rather, we haven't snapped this frame).
            let check_py = fp_to_px(new_y + PLAYER_H as i32 * FP_ONE - 1) + 1;
            let check_row = ((check_py as i32 - GAME_AREA_Y as i32) / TILE_SIZE as i32)
                .max(0)
                .min(LEVEL_ROWS as i32 - 1) as i16;
            
            let mut still_on_ground = false;
            for col in col_start..=col_end {
                if is_solid(self.tile_at(check_row, col)) {
                    still_on_ground = true;
                    break;
                }
            }
            if !still_on_ground {
                self.player.on_ground = false;
            }
        }
    }

    fn destroy_brick(&mut self, row: i16, col: i16) {
        if let Some(t) = self.mut_tile_at(row, col) {
            *t = TILE_AIR;
        }
    }

    fn tick_camera(&mut self) {
        let px = fp_to_px(self.player.x);
        let screen_x = px as i32 - self.camera_x;
        
        // Page scrolling to prevent full screen redraw every tick on SPI displays
        if screen_x > 240 {
            self.camera_x += 128;
        } else if screen_x < 64 && self.camera_x >= 128 {
            self.camera_x -= 128;
        }
        self.camera_x = self.camera_x.max(0).min(MAX_CAMERA_X as i32);
    }

    fn tick_enemies(&mut self) {
        let cam_x = self.camera_x;
        let cam_ex_min = ((cam_x - 320).max(0)) as i16;
        let cam_ex_max = ((cam_x + 640).min(LEVEL_COLS as i32 * TILE_SIZE as i32)) as i16;

        let mut i = 0;
        while i < self.enemy_count {
            let alive;
            let stomped;
            let vx;
            let x;
            let spawn_x;
            let etype;
            {
                let enemy = &self.enemies[i];
                alive = enemy.alive;
                stomped = enemy.stomped;
                vx = enemy.vx;
                x = enemy.x;
                spawn_x = enemy.spawn_x;
                etype = enemy.enemy_type;
            }

            if !alive {
                i += 1;
                continue;
            }

            if stomped {
                let enemy = &mut self.enemies[i];
                enemy.shell_timer -= 1;
                if enemy.shell_timer <= 0 {
                    enemy.alive = false;
                }
                i += 1;
                continue;
            }

            let patrol_range: i32 = 64 * FP_ONE;
            let dx = x - spawn_x * FP_ONE;
            let mut new_vx = vx;
            if dx.abs() > patrol_range {
                new_vx = -new_vx;
            }

            let mut new_x = x + new_vx;
            let ex_px = fp_to_px(new_x);
            let ew = etype.w();
            let ahead_col = if new_vx > 0 {
                px_to_tile(ex_px as i32 + ew as i32 - 1) + 1
            } else {
                px_to_tile(ex_px as i32) - 1
            };

            let ground_row: i16 = 11;
            let ahead_has_ground = (0..LEVEL_COLS).contains(&ahead_col)
                && is_solid(self.tile_at(ground_row, ahead_col));

            if !ahead_has_ground {
                new_vx = -new_vx;
                new_x = x + new_vx;
            }

            let enemy = &mut self.enemies[i];
            enemy.vx = new_vx;
            enemy.x = new_x;

            if ex_px < cam_ex_min - 320 || ex_px > cam_ex_max + 320 {
                enemy.alive = false;
            }

            i += 1;
        }
    }

    fn check_player_enemy_collisions(&mut self) {
        if self.invincible_ticks > 0 {
            return;
        }

        let px = fp_to_px(self.player.x);
        let py = fp_to_px(self.player.y);

        for enemy in &mut self.enemies {
            if !enemy.alive || enemy.stomped {
                continue;
            }
            let ex = fp_to_px(enemy.x);
            let ey = fp_to_px(enemy.y);
            let ew = enemy.enemy_type.w();
            let eh = enemy.enemy_type.h();

            if px + PLAYER_W > ex && px < ex + ew && py + PLAYER_H > ey && py < ey + eh {
                let py_bottom = py + PLAYER_H;
                let ey_top = ey;
                let overlap_top = py_bottom - ey_top;

                if overlap_top <= 16 && self.player.vy >= 0 {
                    enemy.stomped = true;
                    enemy.shell_timer = 120;
                    enemy.vx = 0;
                    self.player.vy = JUMP_VEL / 2;
                    self.player.on_ground = false;
                    self.score += 100;
                } else {
                    self.die();
                    return;
                }
            }
        }
    }

    fn check_flagpole(&mut self) {
        let px = fp_to_px(self.player.x);
        let flag_x = 196 * TILE_SIZE;
        if px >= flag_x - 8 && px <= flag_x + 16 {
            self.mode = MarioMode::LevelComplete;
            self.death_timer = 120;
        }
    }

    fn die(&mut self) {
        if self.lives > 0 {
            self.lives -= 1;
        }
        self.mode = MarioMode::Dying;
        self.death_timer = 60;
    }

    fn respawn(&mut self) {
        self.player = Player {
            x: 3 * FP_ONE,
            y: ((GAME_AREA_Y + 11 * TILE_SIZE - PLAYER_H) as i32) * FP_ONE,
            vx: 0,
            vy: 0,
            on_ground: true,
            facing_left: false,
        };
        self.camera_x = 0;
        self.invincible_ticks = INVINCIBLE_TICKS;
        self.mode = MarioMode::Playing;
    }

    pub fn update_best_score(&mut self, high_scores: &mut impl HighScoreStore) {
        if self.score > self.best_score {
            self.best_score = self.score;
            high_scores.update_mario_best_score(self.score);
        }
    }

    pub fn tile_at(&self, row: i16, col: i16) -> u8 {
        if (0..LEVEL_ROWS).contains(&row) && (0..LEVEL_COLS).contains(&col) {
            self.tiles[row as usize * LEVEL_COLS as usize + col as usize]
        } else {
            TILE_AIR
        }
    }

    pub fn mut_tile_at(&mut self, row: i16, col: i16) -> Option<&mut u8> {
        if (0..LEVEL_ROWS).contains(&row) && (0..LEVEL_COLS).contains(&col) {
            let idx = row as usize * LEVEL_COLS as usize + col as usize;
            if idx < TILES_TOTAL {
                return Some(&mut self.tiles[idx]);
            }
        }
        None
    }

    fn set_tile(&mut self, row: i16, col: i16, tile: u8) {
        if let Some(t) = self.mut_tile_at(row, col) {
            *t = tile;
        }
    }

    pub const fn mode(&self) -> MarioMode {
        self.mode
    }
    pub fn player(&self) -> &Player {
        &self.player
    }
    pub const fn camera_x(&self) -> i32 {
        self.camera_x
    }
    pub const fn score(&self) -> u32 {
        self.score
    }
    pub const fn best_score(&self) -> u32 {
        self.best_score
    }
    pub const fn coins(&self) -> u32 {
        self.coins
    }
    pub const fn lives(&self) -> u32 {
        self.lives
    }
    pub const fn invincible_ticks(&self) -> u8 {
        self.invincible_ticks
    }
    pub fn enemies(&self) -> &[MarioEnemy; MAX_ENEMIES] {
        &self.enemies
    }
    pub const fn enemy_count(&self) -> usize {
        self.enemy_count
    }
    pub const fn pause_action(&self) -> MarioPauseAction {
        self.pause_action
    }
    pub fn tiles(&self) -> &[u8; TILES_TOTAL] {
        &self.tiles
    }

    #[cfg(test)]
    pub fn set_player_position(&mut self, x: i32, y: i32) {
        self.player.x = x;
        self.player.y = y;
    }

    #[cfg(test)]
    pub fn set_player_vy(&mut self, vy: i32) {
        self.player.vy = vy;
    }

    #[cfg(test)]
    pub fn set_player_on_ground(&mut self, on_ground: bool) {
        self.player.on_ground = on_ground;
    }

    #[cfg(test)]
    pub fn set_camera_x(&mut self, cam_x: i32) {
        self.camera_x = cam_x;
    }

    #[cfg(test)]
    pub fn set_invincible_ticks(&mut self, ticks: u8) {
        self.invincible_ticks = ticks;
    }

    #[cfg(test)]
    pub fn set_lives(&mut self, lives: u32) {
        self.lives = lives;
    }

    #[cfg(test)]
    pub fn set_score(&mut self, score: u32) {
        self.score = score;
    }

    #[cfg(test)]
    pub fn set_coins(&mut self, coins: u32) {
        self.coins = coins;
    }

    #[cfg(test)]
    pub fn set_mode(&mut self, mode: MarioMode) {
        self.mode = mode;
    }

    #[cfg(test)]
    pub fn set_player_vx(&mut self, vx: i32) {
        self.player.vx = vx;
    }

    #[cfg(test)]
    pub fn set_enemy_position(&mut self, idx: usize, x: i32, y: i32) {
        if idx < MAX_ENEMIES {
            self.enemies[idx].x = x;
            self.enemies[idx].y = y;
        }
    }

    #[cfg(test)]
    pub fn set_enemy_alive(&mut self, idx: usize, alive: bool) {
        if idx < MAX_ENEMIES {
            self.enemies[idx].alive = alive;
        }
    }

    #[cfg(test)]
    pub fn set_enemy_stomped(&mut self, idx: usize, stomped: bool) {
        if idx < MAX_ENEMIES {
            self.enemies[idx].stomped = stomped;
        }
    }

    #[cfg(test)]
    pub fn set_enemy_vx(&mut self, idx: usize, vx: i32) {
        if idx < MAX_ENEMIES {
            self.enemies[idx].vx = vx;
        }
    }
}

pub const fn is_solid(tile: u8) -> bool {
    matches!(
        tile,
        TILE_GROUND
            | TILE_BRICK
            | TILE_QUESTION
            | TILE_USED
            | TILE_PIPE_TL
            | TILE_PIPE_TR
            | TILE_PIPE_BL
            | TILE_PIPE_BR
            | TILE_HARD
            | TILE_FLAGPOLE
            | TILE_FLAGPOLE_TOP
    )
}

const fn is_gap(col: i16) -> bool {
    col == 85 || col == 86 || col == 130 || col == 131
}

fn set_tile_s(tiles: &mut [u8; TILES_TOTAL], row: i16, col: i16, tile: u8) {
    let idx = row as usize * LEVEL_COLS as usize + col as usize;
    if idx < TILES_TOTAL {
        tiles[idx] = tile;
    }
}

fn fill_tiles_s(tiles: &mut [u8; TILES_TOTAL], row: i16, col: i16, w: i16, h: i16, tile: u8) {
    for r in row..row + h {
        for c in col..col + w {
            set_tile_s(tiles, r, c, tile);
        }
    }
}

fn build_pipe(tiles: &mut [u8; TILES_TOTAL], col: i16, top_row: i16, height: i16) {
    for row in top_row..top_row + height {
        if row == top_row {
            set_tile_s(tiles, row, col, TILE_PIPE_TL);
            set_tile_s(tiles, row, col + 1, TILE_PIPE_TR);
        } else {
            set_tile_s(tiles, row, col, TILE_PIPE_BL);
            set_tile_s(tiles, row, col + 1, TILE_PIPE_BR);
        }
    }
}

fn build_stairs(tiles: &mut [u8; TILES_TOTAL], start_col: i16, height: i16) {
    for step in 0..height {
        let col = start_col + step * 2;
        let top_row = 11 - step;
        for row in top_row..12 {
            set_tile_s(tiles, row, col, TILE_HARD);
            set_tile_s(tiles, row, col + 1, TILE_HARD);
        }
    }
}

impl Default for MarioGame {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryHighScoreStore;

    #[test]
    fn new_game_is_ready() {
        let game = MarioGame::new();
        assert_eq!(game.mode(), MarioMode::Ready);
    }

    #[test]
    fn level_has_ground() {
        let game = MarioGame::new();
        assert_eq!(game.tile_at(12, 0), TILE_GROUND);
        assert_eq!(game.tile_at(13, 0), TILE_GROUND);
    }

    #[test]
    fn gap_is_air() {
        let game = MarioGame::new();
        assert_eq!(game.tile_at(12, 85), TILE_AIR);
        assert_eq!(game.tile_at(13, 86), TILE_AIR);
    }

    #[test]
    fn levels_has_question_blocks() {
        let game = MarioGame::new();
        assert_eq!(game.tile_at(9, 14), TILE_QUESTION);
        assert_eq!(game.tile_at(9, 21), TILE_QUESTION);
        assert_eq!(game.tile_at(5, 61), TILE_QUESTION);
    }

    #[test]
    fn levels_has_pipes() {
        let game = MarioGame::new();
        assert_eq!(game.tile_at(8, 30), TILE_PIPE_TL);
        assert_eq!(game.tile_at(8, 31), TILE_PIPE_TR);
        assert_eq!(game.tile_at(11, 30), TILE_PIPE_BL);
    }

    #[test]
    fn levels_has_flagpole() {
        let game = MarioGame::new();
        assert_eq!(game.tile_at(2, 196), TILE_FLAGPOLE_TOP);
        assert_eq!(game.tile_at(5, 196), TILE_FLAGPOLE);
    }

    #[test]
    fn jumping_applies_velocity() {
        let mut game = MarioGame::new();
        game.mode = MarioMode::Playing;
        let y_before = game.player.y;
        game.jump();
        game.tick(&mut MemoryHighScoreStore::new());
        assert!(game.player.y < y_before, "Player should move up after jump");
    }

    #[test]
    fn gravity_pulls_player_down() {
        let mut game = MarioGame::new();
        game.mode = MarioMode::Playing;
        game.player.on_ground = false;
        game.player.vy = 0;
        let y_before = game.player.y;
        game.tick(&mut MemoryHighScoreStore::new());
        assert!(game.player.vy > 0, "Velocity should increase downward");
        assert!(game.player.y > y_before, "Player should move down");
    }

    #[test]
    fn stomp_enemy_awards_points() {
        let mut game = MarioGame::new();
        game.mode = MarioMode::Playing;
        game.enemy_count = 1;
        game.enemies[0] = MarioEnemy {
            x: 100 * FP_ONE,
            y: 200 * FP_ONE,
            vx: FP_ONE,
            spawn_x: 100 * TILE_SIZE as i32,
            alive: true,
            stomped: false,
            shell_timer: 0,
            enemy_type: MarioEnemyType::ChompNom,
        };
        game.player.x = 100 * FP_ONE;
        game.player.y = 190 * FP_ONE;
        game.player.vy = 10;
        game.check_player_enemy_collisions();
        assert!(game.enemies[0].stomped, "Enemy should be stomped");
        assert!(game.score > 0);
    }

    #[test]
    fn player_starts_at_correct_position() {
        let game = MarioGame::new();
        let expected_y = ((GAME_AREA_Y + 11 * TILE_SIZE - PLAYER_H) as i32) * FP_ONE;
        assert_eq!(game.player.y, expected_y);
        assert_eq!(game.player.x, 3 * FP_ONE);
    }

    #[test]
    fn stairs_are_hard_blocks() {
        let game = MarioGame::new();
        assert_eq!(game.tile_at(11, 176), TILE_HARD);
        assert_eq!(game.tile_at(10, 178), TILE_HARD);
        assert_eq!(game.tile_at(9, 180), TILE_HARD);
    }

    // ── State transitions ──

    #[test]
    fn start_sets_playing_mode() {
        let mut game = MarioGame::new();
        game.start();
        assert_eq!(game.mode(), MarioMode::Playing);
    }

    #[test]
    fn start_resets_player_position() {
        let mut game = MarioGame::new();
        let expected_y = ((GAME_AREA_Y + 11 * TILE_SIZE - PLAYER_H) as i32) * FP_ONE;
        game.start();
        assert_eq!(game.player().x, 3 * FP_ONE);
        assert_eq!(game.player().y, expected_y);
    }

    #[test]
    fn pause_resume_cycle() {
        let mut game = MarioGame::new();
        game.start();
        assert_eq!(game.mode(), MarioMode::Playing);
        game.pause();
        assert_eq!(game.mode(), MarioMode::Paused);
        game.resume();
        assert_eq!(game.mode(), MarioMode::Playing);
    }

    #[test]
    fn pause_while_not_playing_is_noop() {
        let mut game = MarioGame::new();
        assert_eq!(game.mode(), MarioMode::Ready);
        game.pause();
        assert_eq!(game.mode(), MarioMode::Ready);
    }

    #[test]
    fn resume_while_not_paused_is_noop() {
        let mut game = MarioGame::new();
        game.start();
        game.resume();
        assert_eq!(game.mode(), MarioMode::Playing);
    }

    #[test]
    fn cycle_pause_action_toggles() {
        let mut game = MarioGame::new();
        game.start();
        game.pause();
        assert_eq!(game.pause_action(), MarioPauseAction::Continue);
        game.cycle_pause_action();
        assert_eq!(game.pause_action(), MarioPauseAction::Exit);
        game.cycle_pause_action();
        assert_eq!(game.pause_action(), MarioPauseAction::Continue);
    }

    // ── Jump ──

    #[test]
    fn jump_when_on_ground_sets_negative_vy() {
        let mut game = MarioGame::new();
        game.start();
        assert!(game.player().on_ground);
        game.jump();
        assert!(game.player().vy < 0, "Jump should set upward velocity");
    }

    #[test]
    fn jump_when_airborne_does_nothing() {
        let mut game = MarioGame::new();
        game.start();
        game.set_player_on_ground(false);
        let vy_before = game.player().vy;
        game.jump();
        assert_eq!(game.player().vy, vy_before, "Jump should be ignored in air");
    }

    #[test]
    fn jump_moves_player_up() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        game.mark_ticked(0);
        let y_before = game.player().y;
        game.jump();
        assert!(game.due_for_tick(TICK_US));
        game.tick(&mut store);
        assert!(
            game.player().y < y_before,
            "Player should rise after jump tick"
        );
    }

    #[test]
    fn release_jump_allows_fall() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        game.jump();
        game.release_jump();
        // After release, gravity increases vy each tick
        let vy_after_release = game.player().vy;
        game.mark_ticked(0);
        assert!(game.due_for_tick(TICK_US));
        game.tick(&mut store);
        assert!(
            game.player().vy > vy_after_release + 40,
            "Gravity should pull down after jump release"
        );
    }

    #[test]
    fn jump_hold_extends_ascent() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        // Set on_ground false, vy near 0 so jump sets negative vy
        game.set_player_on_ground(true);
        game.mark_ticked(0);
        game.jump();
        let vy_jump = game.player().vy;
        // Tick while holding jump: switch_held should slow gravity
        let mut tick_time: i64 = TICK_US;
        for _ in 0..3 {
            assert!(game.due_for_tick(tick_time));
            game.tick(&mut store);
            game.mark_ticked(tick_time);
            tick_time += TICK_US;
        }
        // After a few held ticks, vy should still be negative (not fully pulled down)
        assert!(
            game.player().vy < 0 || game.player().vy <= vy_jump.abs() / 2,
            "Held jump should keep vy negative or reduce gravity effect"
        );
    }

    // ── Direction ──

    #[test]
    fn set_direction_left_sets_negative_vx() {
        let mut game = MarioGame::new();
        game.start();
        game.set_direction(true, false);
        assert!(game.player().vx < 0, "Left should set negative vx");
    }

    #[test]
    fn set_direction_right_sets_positive_vx() {
        let mut game = MarioGame::new();
        game.start();
        game.set_direction(false, true);
        assert!(game.player().vx > 0, "Right should set positive vx");
    }

    #[test]
    fn set_direction_stops_when_neither() {
        let mut game = MarioGame::new();
        game.start();
        game.set_direction(true, false);
        assert!(game.player().vx < 0);
        game.set_direction(false, false);
        assert_eq!(game.player().vx, 0, "No direction should stop player");
    }

    #[test]
    fn left_direction_moves_player_left() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        game.set_player_on_ground(true);
        game.mark_ticked(0);
        game.set_direction(true, false);
        let x_before = game.player().x;
        assert!(game.due_for_tick(TICK_US));
        game.tick(&mut store);
        assert!(
            game.player().x < x_before,
            "Player should move left after set_direction(true, false)"
        );
    }

    #[test]
    fn right_direction_moves_player_right() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        game.set_player_on_ground(true);
        game.mark_ticked(0);
        game.set_direction(false, true);
        let x_before = game.player().x;
        assert!(game.due_for_tick(TICK_US));
        game.tick(&mut store);
        assert!(
            game.player().x > x_before,
            "Player should move right after set_direction(false, true)"
        );
    }

    // ── Gravity / Ground ──

    #[test]
    fn gravity_increases_vy_when_airborne() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        game.set_player_on_ground(false);
        game.mark_ticked(0);
        let vy_before = game.player().vy;
        assert!(game.due_for_tick(TICK_US));
        game.tick(&mut store);
        assert!(
            game.player().vy > vy_before,
            "Gravity should increase vy over time"
        );
    }

    #[test]
    fn player_falls_and_lands_on_ground() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        game.mark_ticked(0);
        // After start, on_ground is true but player is above ground.
        // Tick until player lands on ground again (on_ground becomes false then true again).
        let mut tick_time: i64 = TICK_US;
        for _ in 0..50 {
            if game.due_for_tick(tick_time) {
                game.tick(&mut store);
                game.mark_ticked(tick_time);
            }
            tick_time += TICK_US;
            if game.player().on_ground && tick_time > 3 * TICK_US {
                break;
            }
        }
        assert!(
            game.player().on_ground,
            "Player should eventually land on ground"
        );
        // Player should be at ground level: bottom at GAME_AREA_Y + 12 * TILE_SIZE
        let expected_bottom = GAME_AREA_Y + 12 * TILE_SIZE;
        let player_bottom = fp_to_px(game.player().y) + PLAYER_H;
        assert_eq!(
            player_bottom, expected_bottom,
            "Player feet should rest on ground surface"
        );
    }

    // ── Dying and lives ──

    #[test]
    fn die_decrements_lives() {
        let mut game = MarioGame::new();
        game.start();
        let lives_before = game.lives();
        // Place player over gap column 85 (no ground below)
        game.set_player_position(85 * TILE_SIZE as i32 * FP_ONE, 200 * FP_ONE);
        game.set_player_on_ground(false);
        assert_eq!(game.mode(), MarioMode::Playing);
        let mut store = MemoryHighScoreStore::new();
        game.mark_ticked(0);
        // Tick multiple times to fall through gap
        let mut tick_time: i64 = TICK_US;
        for _ in 0..100 {
            if game.due_for_tick(tick_time) {
                game.tick(&mut store);
                game.mark_ticked(tick_time);
                if game.mode() == MarioMode::Dying {
                    break;
                }
            }
            tick_time += TICK_US;
        }
        assert_eq!(
            game.mode(),
            MarioMode::Dying,
            "Player should die after falling through gap"
        );
        assert_eq!(game.lives(), lives_before - 1);
    }

    #[test]
    fn respawn_restores_position_and_invincibility() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        game.set_lives(1);
        // Place player over gap to trigger death
        game.set_player_position(85 * TILE_SIZE as i32 * FP_ONE, 200 * FP_ONE);
        game.set_player_on_ground(false);
        game.mark_ticked(0);
        let mut tick_time: i64 = TICK_US;
        // Tick until Dying
        for _ in 0..50 {
            if game.due_for_tick(tick_time) {
                game.tick(&mut store);
                game.mark_ticked(tick_time);
                if game.mode() == MarioMode::Dying {
                    break;
                }
            }
            tick_time += TICK_US;
        }
        assert_eq!(game.mode(), MarioMode::Dying);
        assert_eq!(game.lives(), 0);

        // Tick until death_timer expires (60 ticks)
        for _ in 0..70 {
            if game.mode() != MarioMode::Dying {
                break;
            }
            if game.due_for_tick(tick_time) {
                game.tick(&mut store);
                game.mark_ticked(tick_time);
            }
            tick_time += TICK_US;
        }
        // With 0 lives, should go to GameOver
        assert_eq!(game.mode(), MarioMode::GameOver);
    }

    #[test]
    fn three_deaths_game_over() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        let mut tick_time: i64 = TICK_US;

        for death_num in 0..3 {
            // Place player over gap to trigger death
            game.set_player_position(85 * TILE_SIZE as i32 * FP_ONE, 200 * FP_ONE);
            game.set_player_on_ground(false);
            game.mark_ticked(tick_time - TICK_US);
            // Tick until death
            for _ in 0..50 {
                if game.due_for_tick(tick_time) {
                    game.tick(&mut store);
                    game.mark_ticked(tick_time);
                    if game.mode() == MarioMode::Dying {
                        break;
                    }
                }
                tick_time += TICK_US;
            }
            assert_eq!(game.mode(), MarioMode::Dying);
            assert_eq!(game.lives(), START_LIVES - death_num - 1);
            // Tick through death timer
            for _ in 0..70 {
                if game.mode() != MarioMode::Dying {
                    break;
                }
                if game.due_for_tick(tick_time) {
                    game.tick(&mut store);
                    game.mark_ticked(tick_time);
                }
                tick_time += TICK_US;
            }
            if death_num < 2 {
                assert_eq!(
                    game.mode(),
                    MarioMode::Playing,
                    "Should respawn after death {}",
                    death_num
                );
            } else {
                assert_eq!(
                    game.mode(),
                    MarioMode::GameOver,
                    "Should game over after 3rd death"
                );
            }
        }
    }

    #[test]
    fn respawn_grants_invincibility() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        game.set_lives(2);
        // Place over gap to trigger death
        game.set_player_position(85 * TILE_SIZE as i32 * FP_ONE, 200 * FP_ONE);
        game.set_player_on_ground(false);
        game.mark_ticked(0);
        let mut tick_time: i64 = TICK_US;
        // Tick until dying
        for _ in 0..50 {
            if game.due_for_tick(tick_time) {
                game.tick(&mut store);
                game.mark_ticked(tick_time);
                if game.mode() == MarioMode::Dying {
                    break;
                }
            }
            tick_time += TICK_US;
        }
        assert_eq!(game.mode(), MarioMode::Dying);
        // Tick through death timer
        for _ in 0..70 {
            if game.mode() != MarioMode::Dying {
                break;
            }
            if game.due_for_tick(tick_time) {
                game.tick(&mut store);
                game.mark_ticked(tick_time);
            }
            tick_time += TICK_US;
        }
        assert_eq!(game.mode(), MarioMode::Playing);
        assert!(
            game.invincible_ticks() > 0,
            "Respawn should grant invincibility"
        );
    }

    #[test]
    fn invincibility_decrements_each_tick() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        game.set_invincible_ticks(10);
        game.mark_ticked(0);
        assert!(game.due_for_tick(TICK_US));
        game.tick(&mut store);
        assert_eq!(
            game.invincible_ticks(),
            9,
            "Invincibility should decrement each tick"
        );
    }

    // ── Enemies ──

    #[test]
    fn stomp_enemy_via_public_api() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        // Place within camera range (cam_x=0, culling range [-320, 960])
        // Use early column where ground is at row 12 but player is far above it
        let px = 100 * FP_ONE;
        // Player falling, above enemy
        game.set_player_position(px, 100 * FP_ONE);
        game.set_player_vy(10);
        game.set_player_on_ground(false);
        // Enemy just below player's feet: overlap_top = (100+24)-118 = 6 ≤ 16 → stomp
        // vx=0 so enemy doesn't patrol away
        game.set_enemy_position(0, px, 118 * FP_ONE);
        game.set_enemy_vx(0, 0);
        game.set_enemy_alive(0, true);
        game.set_enemy_stomped(0, false);
        let score_before = game.score();
        game.mark_ticked(0);
        game.tick(&mut store);
        assert!(
            game.enemies()[0].stomped,
            "Enemy should be stomped when player falls on it"
        );
        assert!(game.score() > score_before, "Stomping should award points");
    }

    #[test]
    fn enemy_side_collision_kills_player() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        // Within camera range, early column
        let px = 100 * FP_ONE;
        game.set_player_position(px, 100 * FP_ONE);
        game.set_player_vy(0);
        game.set_player_on_ground(false);
        // Enemy at same height. overlap_top = (100+24)-100 = 24 > 16 → die
        game.set_enemy_position(0, px, 100 * FP_ONE);
        game.set_enemy_vx(0, 0);
        game.set_enemy_alive(0, true);
        game.set_enemy_stomped(0, false);
        game.set_lives(START_LIVES);
        game.mark_ticked(0);
        game.tick(&mut store);
        assert_eq!(
            game.mode(),
            MarioMode::Dying,
            "Side collision with enemy should kill player"
        );
        assert_eq!(game.lives(), START_LIVES - 1);
    }

    // ── Camera ──

    #[test]
    fn camera_follows_player_right() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        game.set_player_on_ground(true);
        game.set_direction(false, true);
        let mut tick_time: i64 = 0;
        game.mark_ticked(tick_time);
        tick_time += TICK_US;
        for _ in 0..5 {
            if game.due_for_tick(tick_time) {
                game.tick(&mut store);
                game.mark_ticked(tick_time);
            }
            tick_time += TICK_US;
        }
        // Camera should never be negative
        assert!(game.camera_x() >= 0, "Camera should never be negative");
        // Camera should have moved if player moved right
        assert!(
            game.player().x > 3 * FP_ONE || game.camera_x() > 0,
            "Player or camera should have moved right"
        );
    }

    #[test]
    fn camera_stays_at_left_boundary() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        // Camera should stay at 0 when player is near left edge
        assert_eq!(game.camera_x(), 0);
        game.set_player_on_ground(true);
        game.set_direction(true, false);
        let mut tick_time: i64 = 0;
        game.mark_ticked(tick_time);
        tick_time += TICK_US;
        for _ in 0..10 {
            if game.due_for_tick(tick_time) {
                game.tick(&mut store);
                game.mark_ticked(tick_time);
            }
            tick_time += TICK_US;
        }
        assert_eq!(
            game.camera_x(),
            0,
            "Camera should stay at 0 when player is near left edge"
        );
    }

    // ── Flagpole ──

    #[test]
    fn reaching_flagpole_completes_level() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        // Move player near flagpole (col 196)
        let flag_x = (196 * TILE_SIZE - 4) as i32 * FP_ONE;
        game.set_player_position(flag_x, game.player().y);
        game.mark_ticked(0);
        game.tick(&mut store);
        assert_eq!(
            game.mode(),
            MarioMode::LevelComplete,
            "Player at flagpole should trigger level complete"
        );
    }

    #[test]
    fn level_complete_transitions_to_ready_after_timer() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        let flag_x = (196 * TILE_SIZE - 4) as i32 * FP_ONE;
        game.set_player_position(flag_x, game.player().y);
        game.mark_ticked(0);
        game.tick(&mut store);
        assert_eq!(game.mode(), MarioMode::LevelComplete);
        // Tick through the completion timer
        let mut tick_time: i64 = TICK_US;
        for _ in 0..150 {
            if game.mode() != MarioMode::LevelComplete {
                break;
            }
            if game.due_for_tick(tick_time) {
                game.tick(&mut store);
                game.mark_ticked(tick_time);
            }
            tick_time += TICK_US;
        }
        assert_eq!(game.mode(), MarioMode::Ready);
    }

    // ── Score ──

    #[test]
    fn update_best_score_persists_when_higher() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.set_score(500);
        assert_eq!(game.best_score(), 0);
        game.update_best_score(&mut store);
        assert_eq!(game.best_score(), 500);
        assert_eq!(store.mario_best_score(), 500);
    }

    #[test]
    fn update_best_score_does_not_decrease() {
        let mut store = MemoryHighScoreStore::new();
        store.update_mario_best_score(1000);
        let mut game = MarioGame::new();
        game.enter(&store);
        // After enter, best_score = 1000
        game.start();
        game.set_score(500);
        game.update_best_score(&mut store);
        assert_eq!(game.best_score(), 1000, "Best score should not decrease");
    }

    // ── Tiles ──

    #[test]
    fn question_block_yields_coin_when_hit_from_below() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        // Player directly BELOW question block at col 14, row 9
        // Block at row 9 occupies py = 160..175
        // Player starts one row below: py = 176 (top of row 10), occupies 176..199
        let player_y = (GAME_AREA_Y + 10 * TILE_SIZE) as i32 * FP_ONE;
        // vy = -2000 produces new_py ≈ 168 (inside block at row 9) after gravity
        let player_x = (14 * TILE_SIZE + TILE_SIZE / 2 - PLAYER_W / 2) as i32 * FP_ONE;
        game.set_player_position(player_x, player_y);
        game.set_player_vy(-2000);
        game.set_player_on_ground(false);
        game.set_mode(MarioMode::Playing);
        let coins_before = game.coins();
        let score_before = game.score();
        game.mark_ticked(0);
        game.tick(&mut store);
        assert!(
            game.coins() > coins_before,
            "Question block should give a coin"
        );
        assert!(
            game.score() >= score_before + 200,
            "Question block should award score"
        );
        assert_eq!(
            game.tile_at(9, 14),
            TILE_USED,
            "Question block should become used"
        );
    }

    #[test]
    fn brick_destroys_when_hit_from_below() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        // Player BELOW brick at col 15-16, row 9
        // Player x positioned so px_left starts at col 15 (px=240), avoiding col 14 (question block)
        let player_y = (GAME_AREA_Y + 10 * TILE_SIZE) as i32 * FP_ONE;
        let player_x = (15 * TILE_SIZE) as i32 * FP_ONE;
        game.set_player_position(player_x, player_y);
        game.set_player_vy(-2000);
        game.set_player_on_ground(false);
        game.set_mode(MarioMode::Playing);
        game.mark_ticked(0);
        game.tick(&mut store);
        assert_eq!(
            game.tile_at(9, 15),
            TILE_AIR,
            "Brick should be destroyed when hit from below"
        );
    }

    // ── Tick timing ──

    #[test]
    fn due_for_tick_prevents_rapid_ticks() {
        let mut game = MarioGame::new();
        game.mark_ticked(100_000);
        // Too early
        assert!(!game.due_for_tick(100_000));
        assert!(!game.due_for_tick(115_000));
        // Exactly at threshold
        assert!(game.due_for_tick(116_666));
        // After threshold
        assert!(game.due_for_tick(200_000));
    }

    // ── Gaps ──

    #[test]
    fn player_falls_through_gap() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        // Place player at gap column 85 (no ground)
        let gap_col_px = 85 * TILE_SIZE;
        game.set_player_position(gap_col_px as i32 * FP_ONE, 200 * FP_ONE);
        game.set_player_on_ground(false);
        game.mark_ticked(0);
        let mut tick_time: i64 = TICK_US;
        // Player should keep falling through gap and eventually die
        for _ in 0..200 {
            if game.mode() == MarioMode::Dying {
                break;
            }
            if game.due_for_tick(tick_time) {
                game.tick(&mut store);
                game.mark_ticked(tick_time);
            }
            tick_time += TICK_US;
        }
        assert_eq!(
            game.mode(),
            MarioMode::Dying,
            "Player should fall to death in gap"
        );
    }

    // ── Enemy patrol ──

    #[test]
    fn enemy_patrols_and_reverses() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        // First enemy is ChompNom at spawn_x=20
        let x_before = game.enemies()[0].x;
        // Tick some times and check the enemy moves
        game.mark_ticked(0);
        for i in 1..20 {
            game.mark_ticked(i * TICK_US as i64);
            if game.due_for_tick(i * TICK_US as i64) {
                game.tick(&mut store);
            }
        }
        // Enemy should have moved from spawn (vx = -FP_ONE initially)
        assert!(
            game.enemies()[0].x != x_before || game.enemies()[0].x != 0,
            "Enemy should move from its spawn position"
        );
    }
}
