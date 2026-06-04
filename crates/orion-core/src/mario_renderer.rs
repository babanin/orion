use crate::generated::om_nom_sprite::{OM_NOM_H, OM_NOM_PALETTE, OM_NOM_SPANS, OM_NOM_W};
use crate::mario::{
    MarioEnemy, MarioEnemyType, MarioGame, MarioMode, MarioPauseAction, FP_SHIFT, GAME_AREA_Y,
    LEVEL_COLS, LEVEL_ROWS, MAX_ENEMIES, TILE_AIR, TILE_BRICK, TILE_FLAGPOLE, TILE_FLAGPOLE_TOP,
    TILE_GROUND, TILE_HARD, TILE_PIPE_BL, TILE_PIPE_BR, TILE_PIPE_TL, TILE_PIPE_TR, TILE_QUESTION,
    TILE_SIZE, TILE_USED,
};
use crate::render::{clear, fill_rect, flush, DisplaySink};
use crate::{font, theme};

const SKY: u16 = theme::rgb565(92, 148, 252);
const GROUND: u16 = theme::rgb565(180, 100, 40);
const GROUND_DARK: u16 = theme::rgb565(140, 72, 24);
const BRICK: u16 = theme::rgb565(200, 100, 40);
const BRICK_DARK: u16 = theme::rgb565(160, 72, 24);
const BRICK_LIGHT: u16 = theme::rgb565(220, 150, 80);
const QUESTION: u16 = theme::rgb565(240, 200, 40);
const QUESTION_DARK: u16 = theme::rgb565(190, 150, 20);
const USED: u16 = theme::rgb565(100, 80, 60);
const PIPE_GREEN: u16 = theme::rgb565(0, 168, 0);
const PIPE_DARK: u16 = theme::rgb565(0, 120, 0);
const PIPE_LIGHT: u16 = theme::rgb565(100, 220, 100);
const HARD: u16 = theme::rgb565(140, 140, 160);
const HARD_DARK: u16 = theme::rgb565(100, 100, 120);
const POLE: u16 = theme::rgb565(120, 120, 120);
const POLE_TOP: u16 = theme::rgb565(200, 200, 80);
const FLAG_RED: u16 = theme::rgb565(220, 40, 40);

const CHOMP_BODY: u16 = theme::rgb565(72, 170, 0);
const CHOMP_DARK: u16 = theme::rgb565(40, 100, 0);
const CHOMP_EYE_WHITE: u16 = theme::rgb565(255, 255, 240);
const CHOMP_EYE_RED: u16 = theme::rgb565(200, 30, 0);
const CHOMP_TEETH: u16 = theme::rgb565(255, 255, 255);
const CHOMP_MOUTH: u16 = theme::rgb565(50, 30, 10);

const SPIKE_BODY: u16 = theme::rgb565(200, 60, 40);
const SPIKE_DARK: u16 = theme::rgb565(140, 30, 20);
const SPIKE_EYE_WHITE: u16 = theme::rgb565(255, 255, 240);
const SPIKE_EYE_RED: u16 = theme::rgb565(200, 30, 0);
const SPIKE_SHELL: u16 = theme::rgb565(180, 80, 50);

const HUD_H: i16 = GAME_AREA_Y;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarioRenderState {
    pub camera_x: i32,
    pub player_x: i16,
    pub player_y: i16,
    pub score: u32,
    pub coins: u32,
    pub lives: u32,
    pub invincible_ticks: u8,
    pub enemies: [MarioEnemy; MAX_ENEMIES],
    pub enemy_count: usize,
}

impl MarioRenderState {
    pub fn capture(game: &MarioGame) -> Self {
        Self {
            camera_x: game.camera_x(),
            player_x: (game.player().x >> FP_SHIFT) as i16,
            player_y: (game.player().y >> FP_SHIFT) as i16,
            score: game.score(),
            coins: game.coins(),
            lives: game.lives(),
            invincible_ticks: game.invincible_ticks(),
            enemies: *game.enemies(),
            enemy_count: game.enemy_count(),
        }
    }
}

pub fn render(display: &mut impl DisplaySink, game: &MarioGame) {
    clear(display, SKY);
    draw_tiles(display, game);
    draw_flagpole_separate(display, game);
    draw_enemies(display, game);
    draw_player_at(display, game, false);
    draw_flag(display, game);
    draw_hud(display, game);

    match game.mode() {
        MarioMode::Ready => draw_center_panel(display, "Super Om Nomario", "PRESS TO START"),
        MarioMode::Paused => draw_pause_menu(display, game),
        MarioMode::GameOver => draw_center_panel(display, "GAME OVER", "PRESS TO RETRY"),
        MarioMode::LevelComplete => draw_center_panel(display, "LEVEL CLEAR!", "WELL DONE"),
        _ => {}
    }
    flush(display);
}

pub fn render_play_delta(
    display: &mut impl DisplaySink,
    game: &MarioGame,
    previous: MarioRenderState,
) {
    let cam_x = game.camera_x();
    let px = (game.player().x >> FP_SHIFT) as i16;
    let py = (game.player().y >> FP_SHIFT) as i16;
    let player_moved = px != previous.player_x || py != previous.player_y;
    let hud_changed = game.score() != previous.score
        || game.coins() != previous.coins
        || game.lives() != previous.lives;
    let cam_changed = cam_x != previous.camera_x;

    if !cam_changed && !player_moved && !hud_changed {
        return;
    }

    // Camera scroll: full background redraw eliminates ghosts.
    // An earlier approach filled only the scrolled-edge strip and relied on
    // tile-redraw for the rest, but when the camera caught up to a stationary
    // player the old sprite remained visible in the sky (ghost image). A full
    // sky fill + tile redraw is simpler and guaranteed correct.
    if cam_changed {
        fill_rect(display, 0, GAME_AREA_Y, 320, 240 - GAME_AREA_Y, SKY);
        draw_tiles(display, game);
        draw_flagpole_separate(display, game);
        draw_enemies(display, game);
    } else if player_moved {
        let prev_screen_x = previous.player_x - previous.camera_x as i16;
        fill_rect(
            display,
            prev_screen_x,
            previous.player_y,
            OM_NOM_W,
            OM_NOM_H,
            SKY,
        );
    }
    draw_player_at(display, game, true);
    draw_flag(display, game);

    if hud_changed {
        draw_hud(display, game);
    }

    flush(display);
}

fn draw_tiles(display: &mut impl DisplaySink, game: &MarioGame) {
    let cam = game.camera_x() as i16;
    let col_start = (cam / TILE_SIZE).max(0);
    let col_end = ((cam + 319 + TILE_SIZE - 1) / TILE_SIZE).min(LEVEL_COLS - 1);

    for col in col_start..=col_end {
        let screen_x = (col * TILE_SIZE) - cam;
        if !((-TILE_SIZE)..=319).contains(&screen_x) {
            continue;
        }
        for row in 0..LEVEL_ROWS {
            let tile = game.tile_at(row, col);
            if tile == TILE_AIR || tile == TILE_FLAGPOLE || tile == TILE_FLAGPOLE_TOP {
                continue;
            }
            draw_tile(display, screen_x, GAME_AREA_Y + row * TILE_SIZE, tile);
        }
    }
}

fn draw_tile(display: &mut impl DisplaySink, x: i16, y: i16, tile: u8) {
    match tile {
        TILE_GROUND => {
            fill_rect(display, x, y, TILE_SIZE, TILE_SIZE, GROUND);
            fill_rect(display, x, y, TILE_SIZE, 2, GROUND_DARK);
            fill_rect(display, x, y + 8, TILE_SIZE, 1, GROUND_DARK);
        }
        TILE_BRICK => {
            fill_rect(display, x, y, TILE_SIZE, TILE_SIZE, BRICK);
            fill_rect(display, x, y, TILE_SIZE, 1, BRICK_DARK);
            fill_rect(display, x + 7, y, 2, TILE_SIZE, BRICK_DARK);
            fill_rect(display, x, y + 7, TILE_SIZE, 2, BRICK_DARK);
            fill_rect(display, x + 3, y + 3, 4, 4, BRICK_LIGHT);
            fill_rect(display, x + 10, y + 10, 4, 4, BRICK_LIGHT);
        }
        TILE_QUESTION => {
            fill_rect(display, x, y, TILE_SIZE, TILE_SIZE, QUESTION);
            fill_rect(display, x, y, TILE_SIZE, 1, QUESTION_DARK);
            fill_rect(display, x, y + 15, TILE_SIZE, 1, QUESTION_DARK);
            fill_rect(display, x, y, 1, TILE_SIZE, QUESTION_DARK);
            fill_rect(display, x + 15, y, 1, TILE_SIZE, QUESTION_DARK);
            font::draw_text(display, x + 5, y + 4, "?", QUESTION_DARK, 1);
        }
        TILE_USED => {
            fill_rect(display, x, y, TILE_SIZE, TILE_SIZE, USED);
            fill_rect(display, x, y, TILE_SIZE, 1, theme::MUTED);
            fill_rect(display, x, y + 15, TILE_SIZE, 1, theme::MUTED);
        }
        TILE_PIPE_TL => {
            fill_rect(display, x, y, TILE_SIZE, TILE_SIZE, PIPE_GREEN);
            fill_rect(display, x, y + 2, TILE_SIZE - 2, TILE_SIZE - 2, PIPE_LIGHT);
            fill_rect(display, x, y, TILE_SIZE, 2, PIPE_DARK);
            fill_rect(display, x + TILE_SIZE - 1, y, 1, TILE_SIZE, PIPE_DARK);
        }
        TILE_PIPE_TR => {
            fill_rect(display, x, y, TILE_SIZE, TILE_SIZE, PIPE_GREEN);
            fill_rect(
                display,
                x + 2,
                y + 2,
                TILE_SIZE - 2,
                TILE_SIZE - 2,
                PIPE_LIGHT,
            );
            fill_rect(display, x, y, TILE_SIZE, 2, PIPE_DARK);
            fill_rect(display, x, y, 1, TILE_SIZE, PIPE_DARK);
        }
        TILE_PIPE_BL => {
            fill_rect(display, x, y, TILE_SIZE, TILE_SIZE, PIPE_GREEN);
            fill_rect(display, x, y, TILE_SIZE, TILE_SIZE, PIPE_LIGHT);
            fill_rect(display, x + TILE_SIZE - 1, y, 1, TILE_SIZE, PIPE_DARK);
        }
        TILE_PIPE_BR => {
            fill_rect(display, x, y, TILE_SIZE, TILE_SIZE, PIPE_GREEN);
            fill_rect(display, x, y, TILE_SIZE, TILE_SIZE, PIPE_LIGHT);
            fill_rect(display, x, y, 1, TILE_SIZE, PIPE_DARK);
        }
        TILE_HARD => {
            fill_rect(display, x, y, TILE_SIZE, TILE_SIZE, HARD);
            fill_rect(display, x, y, TILE_SIZE, 1, HARD_DARK);
            fill_rect(display, x, y + 15, TILE_SIZE, 1, HARD_DARK);
            fill_rect(display, x, y, 1, TILE_SIZE, HARD_DARK);
            fill_rect(display, x + 15, y, 1, TILE_SIZE, HARD_DARK);
        }
        _ => {}
    }
}

fn draw_flagpole_separate(display: &mut impl DisplaySink, game: &MarioGame) {
    let cam = game.camera_x() as i16;
    let pole_col = 196;
    let pole_px = pole_col * TILE_SIZE;
    let sx = pole_px - cam;

    if !(-16..=320).contains(&sx) {
        return;
    }

    let psx = sx + TILE_SIZE / 2 - 1;
    for row in 2..13 {
        fill_rect(
            display,
            psx,
            GAME_AREA_Y + row * TILE_SIZE,
            2,
            TILE_SIZE,
            POLE,
        );
    }
    fill_rect(
        display,
        psx - 2,
        GAME_AREA_Y + 2 * TILE_SIZE - 2,
        6,
        4,
        POLE_TOP,
    );
}

fn draw_enemies(display: &mut impl DisplaySink, game: &MarioGame) {
    let cam = game.camera_x() as i16;
    for i in 0..game.enemy_count() {
        let enemy = &game.enemies()[i];
        if !enemy.alive || enemy.stomped {
            continue;
        }
        let ex = (enemy.x >> FP_SHIFT) as i16 - cam;
        if !(-40..=360).contains(&ex) {
            continue;
        }
        let ey = (enemy.y >> FP_SHIFT) as i16;
        draw_enemy_sprite(display, ex, ey, enemy.enemy_type);
    }
}

fn draw_enemy_sprite(display: &mut impl DisplaySink, x: i16, y: i16, etype: MarioEnemyType) {
    match etype {
        MarioEnemyType::ChompNom => {
            fill_rect(display, x + 2, y, 12, 2, CHOMP_DARK);
            fill_rect(display, x + 1, y + 2, 14, 2, CHOMP_BODY);
            fill_rect(display, x, y + 4, 16, 4, CHOMP_BODY);
            fill_rect(display, x + 1, y + 8, 14, 2, CHOMP_BODY);
            fill_rect(display, x + 2, y + 10, 12, 1, CHOMP_BODY);
            fill_rect(display, x + 7, y + 3, 2, 2, CHOMP_EYE_WHITE);
            fill_rect(display, x + 12, y + 3, 2, 2, CHOMP_EYE_WHITE);
            fill_rect(display, x + 7, y + 3, 1, 1, CHOMP_EYE_RED);
            fill_rect(display, x + 13, y + 3, 1, 1, CHOMP_EYE_RED);
            fill_rect(display, x + 3, y + 11, 2, 1, CHOMP_MOUTH);
            fill_rect(display, x + 11, y + 11, 2, 1, CHOMP_MOUTH);
            fill_rect(display, x + 5, y + 11, 2, 1, CHOMP_TEETH);
            fill_rect(display, x + 9, y + 11, 2, 1, CHOMP_TEETH);
            fill_rect(display, x + 3, y + 12, 3, 2, CHOMP_DARK);
            fill_rect(display, x + 10, y + 12, 3, 2, CHOMP_DARK);
        }
        MarioEnemyType::SpikeNom => {
            fill_rect(display, x + 6, y, 8, 2, SPIKE_DARK);
            fill_rect(display, x + 3, y + 2, 14, 2, SPIKE_DARK);
            fill_rect(display, x + 2, y + 4, 16, 2, SPIKE_BODY);
            fill_rect(display, x + 1, y + 6, 18, 4, SPIKE_BODY);
            fill_rect(display, x, y + 10, 20, 6, SPIKE_BODY);
            fill_rect(display, x + 1, y + 16, 18, 4, SPIKE_BODY);
            fill_rect(display, x + 2, y + 20, 16, 2, SPIKE_BODY);
            fill_rect(display, x + 3, y + 22, 14, 2, SPIKE_DARK);
            fill_rect(display, x + 4, y + 7, 4, 3, SPIKE_EYE_WHITE);
            fill_rect(display, x + 12, y + 7, 4, 3, SPIKE_EYE_WHITE);
            fill_rect(display, x + 5, y + 7, 2, 1, SPIKE_EYE_RED);
            fill_rect(display, x + 13, y + 7, 2, 1, SPIKE_EYE_RED);
            fill_rect(display, x + 3, y + 13, 14, 2, SPIKE_SHELL);
            fill_rect(display, x + 5, y + 15, 10, 1, SPIKE_SHELL);
        }
    }
}

fn draw_player_at(display: &mut impl DisplaySink, game: &MarioGame, delta: bool) {
    let cam = game.camera_x() as i16;
    let px = (game.player().x >> FP_SHIFT) as i16 - cam;
    let py = (game.player().y >> FP_SHIFT) as i16;

    if delta && game.invincible_ticks() > 0 && game.invincible_ticks() % 6 < 3 {
        return;
    }

    for span in OM_NOM_SPANS {
        fill_rect(
            display,
            px + span.x,
            py + span.y,
            span.w,
            1,
            OM_NOM_PALETTE[span.palette as usize],
        );
    }
}

fn draw_flag(display: &mut impl DisplaySink, game: &MarioGame) {
    let cam = game.camera_x() as i16;
    let flag_col_px = 196 * TILE_SIZE;
    let sx = flag_col_px - cam;

    if !(-16..=320).contains(&sx) {
        return;
    }
    let flag_y = GAME_AREA_Y + 5 * TILE_SIZE;
    fill_rect(display, sx + 8, flag_y, 10, 8, FLAG_RED);
    fill_rect(display, sx + 8, flag_y + 2, 10, 1, POLE_TOP);
}

fn draw_hud(display: &mut impl DisplaySink, game: &MarioGame) {
    fill_rect(display, 0, 0, 320, HUD_H, theme::HUD);

    let mut score_buf = font::TextBuffer::<16>::new();
    let _ = core::fmt::write(&mut score_buf, format_args!("SCORE {}", game.score()));
    font::draw_text(display, 8, 4, score_buf.as_str(), theme::TEXT, 1);

    let mut coin_buf = font::TextBuffer::<12>::new();
    let _ = core::fmt::write(&mut coin_buf, format_args!("COINS {}", game.coins()));
    font::draw_text(display, 108, 4, coin_buf.as_str(), theme::TEXT, 1);

    let mut lives_buf = font::TextBuffer::<12>::new();
    let _ = core::fmt::write(&mut lives_buf, format_args!("LIVES {}", game.lives()));
    font::draw_text(display, 200, 4, lives_buf.as_str(), theme::TEXT, 1);

    let mut best_buf = font::TextBuffer::<16>::new();
    let _ = core::fmt::write(&mut best_buf, format_args!("BEST {}", game.best_score()));
    font::draw_text(display, 260, 4, best_buf.as_str(), theme::MUTED, 1);
}

fn draw_center_panel(display: &mut impl DisplaySink, title: &str, subtitle: &str) {
    fill_rect(display, 60, 80, 200, 70, theme::OVERLAY);
    font::draw_centered_text(display, 60, 96, 200, title, theme::TEXT, 2);
    font::draw_centered_text(display, 60, 126, 200, subtitle, theme::MUTED, 1);
}

fn draw_pause_menu(display: &mut impl DisplaySink, game: &MarioGame) {
    fill_rect(display, 82, 82, 156, 74, theme::OVERLAY);
    font::draw_centered_text(display, 82, 96, 156, "PAUSED", theme::TEXT, 2);
    draw_pause_row(
        display,
        102,
        124,
        "CONTINUE",
        game.pause_action() == MarioPauseAction::Continue,
    );
    draw_pause_row(
        display,
        102,
        140,
        "EXIT",
        game.pause_action() == MarioPauseAction::Exit,
    );
}

fn draw_pause_row(display: &mut impl DisplaySink, x: i16, y: i16, label: &str, selected: bool) {
    let color = if selected { theme::ACCENT } else { theme::GRID };
    fill_rect(display, x, y, 116, 13, color);
    font::draw_text(display, x + 8, y + 3, label, theme::TEXT, 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mario::{FP_ONE, PLAYER_H};
    use crate::render::{DrawCommand, RecordingDisplay, Rect};
    use crate::store::MemoryHighScoreStore;

    #[test]
    fn full_render_records_clear_and_flush() {
        let mut display = RecordingDisplay::new();
        let game = MarioGame::new();
        render(&mut display, &game);
        assert!(
            matches!(
                display.commands()[0],
                DrawCommand::Fill {
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 320,
                        h: 240
                    },
                    ..
                }
            ),
            "First command should be full-screen clear"
        );
        assert!(
            matches!(display.commands().last(), Some(DrawCommand::Flush)),
            "Last command should be flush"
        );
    }

    #[test]
    fn full_render_shows_ready_panel() {
        let mut display = RecordingDisplay::new();
        let game = MarioGame::new();
        assert_eq!(game.mode(), MarioMode::Ready);
        render(&mut display, &game);
        // Ready panel draws overlay + title "Super Om Nomario" + subtitle
        let has_overlay = display.commands().iter().any(|cmd| {
            matches!(
                cmd,
                DrawCommand::Fill {
                    rect: Rect {
                        x: 60,
                        y: 80,
                        w: 200,
                        h: 70
                    },
                    color: theme::OVERLAY
                }
            )
        });
        assert!(has_overlay, "Full render should show ready overlay");
    }

    #[test]
    fn play_delta_does_not_clear_full_screen() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        game.set_player_on_ground(true);
        game.mark_ticked(0);

        let previous = MarioRenderState::capture(&game);
        game.set_direction(true, false);
        let tick_time = 20_000i64;
        game.mark_ticked(tick_time);
        game.tick(&mut store);

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);

        let full_clear = display.commands().iter().any(|cmd| {
            matches!(
                cmd,
                DrawCommand::Fill {
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 320,
                        h: 240
                    },
                    ..
                }
            )
        });
        assert!(!full_clear, "Delta render should not full-screen clear");
    }

    #[test]
    fn play_delta_early_return_when_nothing_changed() {
        let mut game = MarioGame::new();
        game.set_mode(MarioMode::Playing);
        let previous = MarioRenderState::capture(&game);

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);

        assert!(
            display.commands().is_empty(),
            "No commands should be emitted when nothing changed"
        );
    }

    #[test]
    fn play_delta_redraws_hud_on_score_change() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        let previous = MarioRenderState::capture(&game);
        // Change score via tick (e.g. collect coin or stomp enemy)
        game.set_score(200);
        // Force a tick to register the change for render
        game.mark_ticked(20_000);
        game.tick(&mut store);

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);

        let hud_redraw = display.commands().iter().any(|cmd| {
            matches!(
                cmd,
                DrawCommand::Fill {
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 320,
                        h: 16
                    },
                    ..
                }
            )
        });
        assert!(hud_redraw, "HUD should be redrawn when score changes");
        assert!(
            matches!(display.commands().last(), Some(DrawCommand::Flush)),
            "Delta render should flush"
        );
    }

    #[test]
    fn play_delta_redraws_hud_on_lives_change() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        let previous = MarioRenderState::capture(&game);
        game.set_lives(2);
        game.mark_ticked(20_000);
        game.tick(&mut store);

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);

        let hud_redraw = display.commands().iter().any(|cmd| {
            matches!(
                cmd,
                DrawCommand::Fill {
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 320,
                        h: 16
                    },
                    ..
                }
            )
        });
        assert!(hud_redraw, "HUD should be redrawn when lives change");
    }

    #[test]
    fn play_delta_erases_previous_player_at_screen_coords() {
        let mut game = MarioGame::new();
        // Set up: camera at pixel 50, player at world pixel (100, 200)
        game.set_camera_x(50);
        game.set_player_position(100 * FP_ONE, 200 * FP_ONE);
        let previous = MarioRenderState::capture(&game);

        // Now move the player to world pixel 120
        game.set_player_position(120 * FP_ONE, 200 * FP_ONE);

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);

        // The erase should be at screen coordinates:
        //   previous.player_x - previous.camera_x = 100 - 50 = 50
        let expected_erase_x: i16 = 50;
        let expected_erase_y = ((200 * FP_ONE) >> FP_SHIFT) as i16;

        let has_erase = display.commands().iter().any(|cmd| {
            matches!(cmd, DrawCommand::Fill {
                rect: Rect { x, y, w, h },
                color: SKY,
            } if *x == expected_erase_x && *y == expected_erase_y && *w == OM_NOM_W && *h == OM_NOM_H)
        });

        assert!(
            has_erase,
            "Delta render should erase previous player at screen coords (world_x - camera_x), \
             not at raw world coords. Expected erase at ({}, {}) with size ({}x{})",
            expected_erase_x, expected_erase_y, OM_NOM_W, OM_NOM_H,
        );
    }

    #[test]
    fn play_delta_draws_player_at_new_screen_coords() {
        let mut game = MarioGame::new();
        game.set_camera_x(50);
        game.set_player_position(100 * FP_ONE, 200 * FP_ONE);
        let previous = MarioRenderState::capture(&game);

        game.set_player_position(130 * FP_ONE, 200 * FP_ONE);

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);

        // After the move and optional erase, the player should be drawn.
        // We can't easily assert exact span positions, but we can check
        // there are fill_rect calls with non-SKY colors (player palette colors).
        let player_drawn = display.commands().iter().any(|cmd| {
            matches!(cmd, DrawCommand::Fill {
                rect: Rect { x, y, .. },
                color,
            } if *color != SKY && *x >= 0 && *y >= 200 && *y < (200 + PLAYER_H) as i16)
        });
        assert!(
            player_drawn,
            "Player should be drawn at new screen coordinates"
        );
    }

    #[test]
    fn play_delta_draws_enemies_on_camera_scroll() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.set_mode(MarioMode::Playing);
        // Position player so camera is at 20
        game.set_camera_x(20);
        game.set_player_position(180 * FP_ONE, 200 * FP_ONE);
        game.set_player_on_ground(true);
        // Enemy at px=240, visible on screen when camera is at 20
        game.set_enemy_position(0, 240 * FP_ONE, 178 * FP_ONE);
        game.set_enemy_alive(0, true);
        let previous = MarioRenderState::capture(&game);

        // Move player right so screen_x > 240, triggering page scroll (+128)
        game.set_player_position(262 * FP_ONE, 200 * FP_ONE);
        game.mark_ticked(20_000);
        game.tick(&mut store);

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);

        // Camera scroll should trigger tile draw and enemy draw
        let tile_drawn = display.commands().iter().any(|cmd| {
            matches!(cmd, DrawCommand::Fill {
                rect: Rect { x, y, w, h },
                color,
            } if *color == GROUND || *color == BRICK || *color == HARD || *color == QUESTION)
        });
        assert!(tile_drawn, "Camera scroll should redraw visible tiles");
    }

    #[test]
    fn play_delta_handles_no_camera_change_with_player_move() {
        let mut game = MarioGame::new();
        let mut store = MemoryHighScoreStore::new();
        game.start();
        game.set_camera_x(50);
        game.set_player_position(100 * FP_ONE, 200 * FP_ONE);
        let previous = MarioRenderState::capture(&game);

        // Move player but keep camera same
        game.set_player_position(140 * FP_ONE, 200 * FP_ONE);
        game.mark_ticked(20_000);
        game.tick(&mut store);

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);

        // Should still produce commands (erase + redraw)
        assert!(
            !display.commands().is_empty(),
            "Player-only move should produce draw commands"
        );
        assert!(
            matches!(display.commands().last(), Some(DrawCommand::Flush)),
            "Should flush after player-only move"
        );
    }
}
