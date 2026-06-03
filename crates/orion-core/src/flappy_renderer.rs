use core::fmt::Write;

use crate::flappy::{
    FlappyGame, FlappyMode, FlappyObstacle, FlappyPauseAction, FLAPPY_FLOOR_Y, FLAPPY_GAP_H,
    FLAPPY_OBSTACLE_W, FLAPPY_PLAYER_X, FLAPPY_PLAY_TOP,
};
use crate::generated::om_nom_sprite::{OM_NOM_H, OM_NOM_PALETTE, OM_NOM_SPANS, OM_NOM_W};
use crate::render::{clear, fill_rect, flush, DisplaySink, Rect};
use crate::{font, theme};

const SKY: u16 = theme::rgb565(18, 31, 42);
const CANDLE: u16 = theme::rgb565(246, 210, 126);
const CANDLE_DARK: u16 = theme::rgb565(166, 114, 58);
const FLAME: u16 = theme::rgb565(255, 92, 38);
const JELLY: u16 = theme::rgb565(180, 58, 214);
const JELLY_DARK: u16 = theme::rgb565(107, 39, 150);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlappyRenderState {
    pub player_y: i16,
    pub score: u32,
    pub best_score: u32,
    pub lives: u32,
    pub obstacles: [FlappyObstacle; crate::flappy::FLAPPY_OBSTACLE_COUNT],
}

impl FlappyRenderState {
    pub fn capture(game: &FlappyGame) -> Self {
        Self {
            player_y: game.player_y(),
            score: game.score(),
            best_score: game.best_score(),
            lives: game.lives(),
            obstacles: *game.obstacles(),
        }
    }
}

pub fn render(display: &mut impl DisplaySink, game: &FlappyGame) {
    clear(display, SKY);
    draw_background(display);
    draw_hud(display, game.score(), game.best_score(), game.lives());
    for obstacle in game.obstacles() {
        draw_obstacle(display, *obstacle);
    }
    draw_player(display, FLAPPY_PLAYER_X, game.player_y());

    match game.mode() {
        FlappyMode::Ready => draw_center_panel(display, "OM NOM", "PRESS TO START"),
        FlappyMode::Playing => {}
        FlappyMode::Paused => draw_pause_menu(display, game),
        FlappyMode::GameOver => draw_center_panel(display, "GAME OVER", "PRESS TO RETRY"),
    }
    flush(display);
}

pub fn render_pause_menu(display: &mut impl DisplaySink, game: &FlappyGame) {
    render(display, game);
}

pub fn render_play_delta(
    display: &mut impl DisplaySink,
    game: &FlappyGame,
    previous: FlappyRenderState,
) {
    let player_y = game.player_y();
    let hud_changed = game.score() != previous.score
        || game.best_score() != previous.best_score
        || game.lives() != previous.lives;
    let obstacles_changed = previous
        .obstacles
        .iter()
        .zip(game.obstacles().iter())
        .any(|(previous_obstacle, obstacle)| previous_obstacle != obstacle);
    if player_y == previous.player_y && !hud_changed && !obstacles_changed {
        return;
    }

    erase_player_delta(display, previous.player_y, player_y);

    if hud_changed {
        draw_hud(display, game.score(), game.best_score(), game.lives());
    }

    for (previous_obstacle, obstacle) in previous.obstacles.iter().zip(game.obstacles().iter()) {
        draw_obstacle_delta(display, *previous_obstacle, *obstacle);
    }
    draw_player(display, FLAPPY_PLAYER_X, player_y);
    flush(display);
}

fn draw_background(display: &mut impl DisplaySink) {
    fill_rect(display, 0, 0, 320, 24, theme::HUD);
    fill_rect(display, 0, FLAPPY_FLOOR_Y, 320, 16, JELLY_DARK);
    fill_rect(display, 0, FLAPPY_FLOOR_Y + 4, 320, 12, JELLY);
}

fn draw_hud(display: &mut impl DisplaySink, score: u32, best_score: u32, lives: u32) {
    fill_rect(display, 0, 0, 320, 24, theme::HUD);
    font::draw_text(display, 8, 7, "OM NOM", theme::TEXT, 1);

    let mut score_text = font::TextBuffer::<16>::new();
    let _ = write!(score_text, "SCORE {}", score);
    font::draw_text(display, 78, 7, score_text.as_str(), theme::TEXT, 1);

    let mut lives_text = font::TextBuffer::<16>::new();
    let _ = write!(lives_text, "LIVES {}", lives);
    font::draw_text(display, 170, 7, lives_text.as_str(), theme::TEXT, 1);

    let mut best_text = font::TextBuffer::<16>::new();
    let _ = write!(best_text, "BEST {}", best_score);
    font::draw_text(display, 248, 7, best_text.as_str(), theme::MUTED, 1);
}

fn draw_obstacle(display: &mut impl DisplaySink, obstacle: FlappyObstacle) {
    let top_h = obstacle.gap_y - FLAPPY_PLAY_TOP;
    fill_rect(
        display,
        obstacle.x,
        FLAPPY_PLAY_TOP,
        FLAPPY_OBSTACLE_W,
        top_h,
        CANDLE,
    );
    fill_rect(
        display,
        obstacle.x + FLAPPY_OBSTACLE_W / 2 - 2,
        FLAPPY_PLAY_TOP,
        4,
        top_h,
        CANDLE_DARK,
    );
    fill_rect(
        display,
        obstacle.x - 2,
        obstacle.gap_y - 8,
        FLAPPY_OBSTACLE_W + 4,
        8,
        CANDLE_DARK,
    );
    fill_rect(display, obstacle.x + 8, obstacle.gap_y - 13, 8, 5, FLAME);

    let jelly_y = obstacle.gap_y + FLAPPY_GAP_H;
    let jelly_h = FLAPPY_FLOOR_Y - jelly_y;
    fill_rect(
        display,
        obstacle.x,
        jelly_y,
        FLAPPY_OBSTACLE_W,
        jelly_h,
        JELLY,
    );
    fill_rect(
        display,
        obstacle.x - 2,
        jelly_y,
        FLAPPY_OBSTACLE_W + 4,
        9,
        JELLY_DARK,
    );
    fill_rect(display, obstacle.x + 4, jelly_y + 5, 5, 5, theme::TEXT);
    fill_rect(display, obstacle.x + 15, jelly_y + 5, 5, 5, theme::TEXT);
}

fn draw_obstacle_delta(
    display: &mut impl DisplaySink,
    previous: FlappyObstacle,
    current: FlappyObstacle,
) {
    if current == previous {
        return;
    }

    if current.gap_y == previous.gap_y && current.x < previous.x {
        erase_obstacle_trailing_edge(display, previous, previous.x - current.x);
    } else {
        erase_obstacle(display, previous);
    }
    draw_obstacle(display, current);
}

fn erase_obstacle_trailing_edge(display: &mut impl DisplaySink, obstacle: FlappyObstacle, dx: i16) {
    let dx = dx.max(1);
    let old_right = obstacle.x + FLAPPY_OBSTACLE_W + 2;
    fill_rect(
        display,
        old_right - dx,
        FLAPPY_PLAY_TOP,
        dx,
        FLAPPY_FLOOR_Y - FLAPPY_PLAY_TOP,
        SKY,
    );
}

fn erase_obstacle(display: &mut impl DisplaySink, obstacle: FlappyObstacle) {
    let rect = Rect {
        x: obstacle.x - 3,
        y: FLAPPY_PLAY_TOP,
        w: FLAPPY_OBSTACLE_W + 6,
        h: FLAPPY_FLOOR_Y - FLAPPY_PLAY_TOP,
    };
    fill_rect(display, rect.x, rect.y, rect.w, rect.h, SKY);
}

fn draw_player(display: &mut impl DisplaySink, x: i16, y: i16) {
    for span in OM_NOM_SPANS {
        fill_rect(
            display,
            x + span.x,
            y + span.y,
            span.w,
            1,
            OM_NOM_PALETTE[span.palette as usize],
        );
    }
}

fn erase_player_delta(display: &mut impl DisplaySink, previous_y: i16, current_y: i16) {
    if previous_y == current_y {
        return;
    }

    clear_player_border(display, previous_y, current_y);
    for span in OM_NOM_SPANS {
        let screen_y = previous_y + span.y;
        let start_x = FLAPPY_PLAYER_X + span.x;
        let end_x = start_x + span.w;
        let mut run_start = None;

        let mut x = start_x;
        while x < end_x {
            if sprite_covers_screen_pixel(x, screen_y, current_y) {
                if let Some(start) = run_start {
                    fill_rect(display, start, screen_y, x - start, 1, SKY);
                    run_start = None;
                }
            } else if run_start.is_none() {
                run_start = Some(x);
            }
            x += 1;
        }

        if let Some(start) = run_start {
            fill_rect(display, start, screen_y, end_x - start, 1, SKY);
        }
    }
}

fn clear_player_border(display: &mut impl DisplaySink, previous_y: i16, current_y: i16) {
    let border_x = FLAPPY_PLAYER_X - 1;
    let border_w = OM_NOM_W + 2;

    if !current_sprite_covers_screen_y(previous_y - 1, current_y) {
        fill_rect(display, border_x, previous_y - 1, border_w, 1, SKY);
    }
    if !current_sprite_covers_screen_y(previous_y + OM_NOM_H, current_y) {
        fill_rect(display, border_x, previous_y + OM_NOM_H, border_w, 1, SKY);
    }
    fill_rect(display, border_x, previous_y, 1, OM_NOM_H, SKY);
    fill_rect(
        display,
        border_x + OM_NOM_W + 1,
        previous_y,
        1,
        OM_NOM_H,
        SKY,
    );
}

fn current_sprite_covers_screen_y(screen_y: i16, current_y: i16) -> bool {
    OM_NOM_SPANS
        .iter()
        .any(|span| current_y + span.y == screen_y)
}

fn sprite_covers_screen_pixel(x: i16, y: i16, sprite_y: i16) -> bool {
    OM_NOM_SPANS.iter().any(|span| {
        let span_y = sprite_y + span.y;
        let span_x = FLAPPY_PLAYER_X + span.x;
        y == span_y && x >= span_x && x < span_x + span.w
    })
}

fn draw_center_panel(display: &mut impl DisplaySink, title: &str, subtitle: &str) {
    fill_rect(display, 76, 90, 168, 58, theme::OVERLAY);
    font::draw_centered_text(display, 76, 104, 168, title, theme::TEXT, 2);
    font::draw_centered_text(display, 76, 132, 168, subtitle, theme::MUTED, 1);
}

fn draw_pause_menu(display: &mut impl DisplaySink, game: &FlappyGame) {
    fill_rect(display, 82, 82, 156, 74, theme::OVERLAY);
    font::draw_centered_text(display, 82, 96, 156, "PAUSED", theme::TEXT, 2);
    draw_pause_row(
        display,
        102,
        124,
        "CONTINUE",
        game.pause_action() == FlappyPauseAction::Continue,
    );
    draw_pause_row(
        display,
        102,
        140,
        "EXIT",
        game.pause_action() == FlappyPauseAction::Exit,
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
    use crate::flappy::FlappyGame;
    use crate::render::{DrawCommand, RecordingDisplay};
    use crate::rng::ScriptedRng;
    use crate::store::MemoryHighScoreStore;

    #[test]
    fn full_render_records_clear_and_flush() {
        let mut display = RecordingDisplay::new();
        let game = FlappyGame::new();
        render(&mut display, &game);
        assert!(matches!(
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
        ));
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn render_state_captures_lives_for_hud_deltas() {
        let game = FlappyGame::new();
        let state = FlappyRenderState::capture(&game);
        assert_eq!(state.lives, game.lives());
    }

    #[test]
    fn play_delta_skips_velocity_only_change() {
        let store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        let previous = FlappyRenderState::capture(&game);
        game.flap();

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);
        assert!(display.commands().is_empty());
    }

    #[test]
    fn play_delta_does_not_clear_full_screen() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        let previous = FlappyRenderState::capture(&game);
        game.tick(&mut store, &mut rng);

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);
        assert!(!display.commands().iter().any(|command| {
            matches!(
                command,
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
        }));
        assert!(matches!(
            display.commands().last(),
            Some(DrawCommand::Flush)
        ));
    }

    #[test]
    fn play_delta_does_not_clear_whole_player_box() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        let previous = FlappyRenderState::capture(&game);
        game.tick(&mut store, &mut rng);

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);

        assert!(!display.commands().iter().any(|command| {
            matches!(
                command,
                DrawCommand::Fill {
                    rect: Rect { w, h, .. },
                    color: SKY,
                } if *w == OM_NOM_W + 2 && *h == OM_NOM_H + 2
            )
        }));
    }

    #[test]
    fn play_delta_redraws_hud_when_lives_change_without_full_clear() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        let previous = FlappyRenderState::capture(&game);
        game.set_player_y_for_test(FLAPPY_FLOOR_Y);
        game.tick(&mut store, &mut rng);

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);

        assert!(display.commands().iter().any(|command| {
            matches!(
                command,
                DrawCommand::Fill {
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 320,
                        h: 24
                    },
                    ..
                }
            )
        }));
        assert!(!display.commands().iter().any(|command| {
            matches!(
                command,
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
        }));
    }

    #[test]
    fn play_delta_scroll_erases_only_obstacle_trailing_edge() {
        let mut store = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([1]);
        let mut game = FlappyGame::new();
        game.start(&store, &mut rng, 0);
        let previous = FlappyRenderState::capture(&game);
        game.tick(&mut store, &mut rng);

        let mut display = RecordingDisplay::new();
        render_play_delta(&mut display, &game, previous);

        assert!(display.commands().iter().any(|command| {
            matches!(
                command,
                DrawCommand::Fill {
                    rect: Rect { w: 2, h, .. },
                    color: SKY,
                } if *h == FLAPPY_FLOOR_Y - FLAPPY_PLAY_TOP
            )
        }));
        assert!(!display.commands().iter().any(|command| {
            matches!(
                command,
                DrawCommand::Fill {
                    rect: Rect { w, h, .. },
                    color: SKY,
                } if *w == FLAPPY_OBSTACLE_W + 6 && *h == FLAPPY_FLOOR_Y - FLAPPY_PLAY_TOP
            )
        }));
    }
}
