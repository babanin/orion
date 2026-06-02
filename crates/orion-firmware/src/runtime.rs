use esp_idf_sys as sys;
#[cfg(feature = "flappy")]
use orion_core::FlappyApplication;
use orion_core::{
    render_launcher, AppAction, FlagsApplication, Game2048Application, InputFrame, Launcher,
    LauncherAction, SnakeApplication, TetrisApplication,
};

use crate::display::Display;
use crate::encoder::Encoder;
use crate::esp_rng::EspRng;
use crate::joystick::Joystick;
use crate::network::NetworkManager;
use crate::nvs_store::NvsHighScoreStore;
use crate::speaker::Speaker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveApp {
    Flags,
    Snake,
    Game2048,
    Tetris,
    #[cfg(feature = "flappy")]
    Flappy,
}

#[cfg(feature = "flappy")]
const APP_TITLES: [&str; 6] = ["FLAGS", "SNAKE", "2048", "TETRIS", "OM NOM", "HOME"];
#[cfg(not(feature = "flappy"))]
const APP_TITLES: [&str; 5] = ["FLAGS", "SNAKE", "2048", "TETRIS", "HOME"];

pub struct OrionRuntime {
    high_scores: NvsHighScoreStore,
    display: Display,
    speaker: Speaker,
    joystick: Joystick,
    encoder: Encoder,
    network: NetworkManager,
    rng: EspRng,
    launcher: Launcher<{ APP_TITLES.len() }>,
    flags: FlagsApplication,
    snake: SnakeApplication,
    game2048: Game2048Application,
    tetris: TetrisApplication,
    #[cfg(feature = "flappy")]
    flappy: FlappyApplication,
    active_app: Option<ActiveApp>,
}

impl OrionRuntime {
    pub fn new() -> Self {
        Self {
            high_scores: NvsHighScoreStore::new(),
            display: Display::new(),
            speaker: Speaker::new(),
            joystick: Joystick::new(),
            encoder: Encoder::new(),
            network: NetworkManager::new(),
            rng: EspRng,
            launcher: Launcher::new(APP_TITLES),
            flags: FlagsApplication::default(),
            snake: SnakeApplication::new(),
            game2048: Game2048Application::new(),
            tetris: TetrisApplication::new(),
            #[cfg(feature = "flappy")]
            flappy: FlappyApplication::new(),
            active_app: None,
        }
    }

    pub fn init(&mut self) -> Result<(), sys::EspError> {
        boot_log("orion: nvs init\n");
        self.high_scores.init()?;
        boot_log("orion: display init\n");
        self.display.init()?;
        boot_log("orion: joystick init\n");
        self.joystick.init()?;
        boot_log("orion: encoder init\n");
        self.encoder.init()?;
        boot_log("orion: speaker init\n");
        self.speaker.init()?;
        boot_log("orion: launcher render\n");
        self.render_launcher();
        boot_log("orion: network init\n");
        self.network.init(now_us());
        self.render_launcher();
        boot_log("orion: init done\n");
        Ok(())
    }

    pub fn run(&mut self) -> ! {
        loop {
            let now = now_us();
            let input = InputFrame {
                joystick: self.joystick.poll(now),
                encoder: self.encoder.poll(now),
            };

            match self.active_app {
                None => {
                    if self.network.update(now)
                        && self.launcher.view() == orion_core::LauncherView::Home
                    {
                        self.render_launcher();
                    }
                    self.handle_launcher_input(input, now);
                }
                Some(ActiveApp::Flags) => {
                    let action = self.flags.update(
                        &mut self.display,
                        &mut self.high_scores,
                        &mut self.rng,
                        input,
                        now,
                    );
                    self.handle_app_action(action, ActiveApp::Flags);
                }
                Some(ActiveApp::Snake) => {
                    let action = self.snake.update(
                        &mut self.display,
                        &mut self.high_scores,
                        &mut self.rng,
                        input,
                        now,
                    );
                    self.handle_app_action(action, ActiveApp::Snake);
                }
                Some(ActiveApp::Game2048) => {
                    let action = self.game2048.update(
                        &mut self.display,
                        &mut self.high_scores,
                        &mut self.rng,
                        input,
                        now,
                    );
                    self.handle_app_action(action, ActiveApp::Game2048);
                }
                Some(ActiveApp::Tetris) => {
                    let action = self
                        .tetris
                        .update(&mut self.display, &mut self.rng, input, now);
                    self.handle_app_action(action, ActiveApp::Tetris);
                }
                #[cfg(feature = "flappy")]
                Some(ActiveApp::Flappy) => {
                    let action = self.flappy.update(
                        &mut self.display,
                        &mut self.high_scores,
                        &mut self.rng,
                        input,
                        now,
                    );
                    self.handle_app_action(action, ActiveApp::Flappy);
                }
            }

            unsafe {
                let ticks = ((20 * sys::configTICK_RATE_HZ) + 999) / 1000;
                sys::vTaskDelay(ticks.max(1));
            }
        }
    }

    fn render_launcher(&mut self) {
        render_launcher(
            &mut self.display,
            APP_TITLES,
            self.launcher.view(),
            self.launcher.selected_index(),
            self.network.snapshot(),
        );
    }

    fn handle_launcher_input(&mut self, input: InputFrame, now_us: i64) {
        match self.launcher.update(input, now_us) {
            LauncherAction::None => {}
            LauncherAction::Redraw => self.render_launcher(),
            LauncherAction::GoHome => {
                self.joystick.reset_button();
                self.encoder.reset_button();
                self.render_launcher();
            }
            LauncherAction::Enter(index) => {
                let app = match index {
                    0 => ActiveApp::Flags,
                    1 => ActiveApp::Snake,
                    2 => ActiveApp::Game2048,
                    3 => ActiveApp::Tetris,
                    #[cfg(feature = "flappy")]
                    4 => ActiveApp::Flappy,
                    _ => {
                        self.launcher.show_home();
                        self.joystick.reset_button();
                        self.encoder.reset_button();
                        self.render_launcher();
                        return;
                    }
                };
                self.active_app = Some(app);
                self.joystick.reset_button();
                self.encoder.reset_button();
                match app {
                    ActiveApp::Flags => {
                        self.flags.enter(&self.high_scores);
                        self.flags.render_full(&mut self.display);
                    }
                    ActiveApp::Snake => {
                        self.snake.enter(&self.high_scores);
                        self.snake.render_full(&mut self.display);
                    }
                    ActiveApp::Game2048 => {
                        self.game2048.enter(&self.high_scores);
                        self.game2048.render_full(&mut self.display);
                    }
                    ActiveApp::Tetris => {
                        self.tetris.enter();
                        self.tetris.render_full(&mut self.display);
                    }
                    #[cfg(feature = "flappy")]
                    ActiveApp::Flappy => {
                        self.flappy.enter(&self.high_scores);
                        self.flappy.render_full(&mut self.display);
                    }
                }
            }
        }
    }

    fn handle_app_action(&mut self, action: AppAction, app: ActiveApp) {
        match action {
            AppAction::None => {}
            AppAction::RedrawFull => match app {
                ActiveApp::Flags => self.flags.render_full(&mut self.display),
                ActiveApp::Snake => self.snake.render_full(&mut self.display),
                ActiveApp::Game2048 => self.game2048.render_full(&mut self.display),
                ActiveApp::Tetris => self.tetris.render_full(&mut self.display),
                #[cfg(feature = "flappy")]
                ActiveApp::Flappy => self.flappy.render_full(&mut self.display),
            },
            AppAction::ExitToLauncher => {
                self.active_app = None;
                self.launcher.show_game_menu();
                self.joystick.reset_button();
                self.encoder.reset_button();
                self.render_launcher();
            }
        }
    }
}

impl Default for OrionRuntime {
    fn default() -> Self {
        Self::new()
    }
}

fn now_us() -> i64 {
    unsafe { sys::esp_timer_get_time() }
}

fn boot_log(message: &str) {
    for byte in message.as_bytes() {
        unsafe {
            sys::esp_rom_printf(b"%c\0".as_ptr().cast(), *byte as i32);
        }
    }
}
