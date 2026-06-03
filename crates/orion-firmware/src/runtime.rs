use esp_idf_sys as sys;
#[cfg(feature = "flappy")]
use orion_core::FlappyApplication;
use orion_core::{
    render_launcher, AppAction, FlagsApplication, Game2048Application, InputFrame, Launcher,
    LauncherAction, PomodoroApplication, SnakeApplication, TetrisApplication,
};

use crate::display::Display;
use crate::encoder::Encoder;
use crate::esp_rng::EspRng;
use crate::joystick::Joystick;
use crate::network::NetworkManager;
use crate::nvs_store::NvsHighScoreStore;
use crate::pomodoro_nvs_store::NvsPomodoroSettingsStore;
use crate::speaker::Speaker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveGame {
    Flags,
    Snake,
    Game2048,
    Tetris,
    #[cfg(feature = "flappy")]
    Flappy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveTool {
    Pomodoro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveApp {
    Game(ActiveGame),
    Tool(ActiveTool),
}

#[cfg(feature = "flappy")]
const GAME_TITLES: [&str; 6] = ["FLAGS", "SNAKE", "2048", "TETRIS", "OM NOM", "HOME"];
#[cfg(not(feature = "flappy"))]
const GAME_TITLES: [&str; 5] = ["FLAGS", "SNAKE", "2048", "TETRIS", "HOME"];
const APP_TITLES: [&str; 2] = ["POMODORO", "HOME"];

pub struct OrionRuntime {
    high_scores: NvsHighScoreStore,
    pomodoro_settings: NvsPomodoroSettingsStore,
    display: Display,
    speaker: Speaker,
    joystick: Joystick,
    encoder: Encoder,
    network: NetworkManager,
    rng: EspRng,
    launcher: Launcher<{ GAME_TITLES.len() }, { APP_TITLES.len() }>,
    flags: FlagsApplication,
    snake: SnakeApplication,
    game2048: Game2048Application,
    tetris: TetrisApplication,
    #[cfg(feature = "flappy")]
    flappy: FlappyApplication,
    pomodoro: PomodoroApplication,
    active_app: Option<ActiveApp>,
}

impl OrionRuntime {
    pub fn new() -> Self {
        Self {
            high_scores: NvsHighScoreStore::new(),
            pomodoro_settings: NvsPomodoroSettingsStore::new(),
            display: Display::new(),
            speaker: Speaker::new(),
            joystick: Joystick::new(),
            encoder: Encoder::new(),
            network: NetworkManager::new(),
            rng: EspRng,
            launcher: Launcher::new(GAME_TITLES, APP_TITLES),
            flags: FlagsApplication::default(),
            snake: SnakeApplication::new(),
            game2048: Game2048Application::new(),
            tetris: TetrisApplication::new(),
            #[cfg(feature = "flappy")]
            flappy: FlappyApplication::new(),
            pomodoro: PomodoroApplication::new(),
            active_app: None,
        }
    }

    pub fn init(&mut self) -> Result<(), sys::EspError> {
        boot_log("orion: nvs init\n");
        self.high_scores.init()?;
        self.pomodoro_settings.init()?;
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
                    self.handle_launcher_input(input.without_encoder(), now);
                }
                Some(ActiveApp::Game(ActiveGame::Flags)) => {
                    let action = self.flags.update(
                        &mut self.display,
                        &mut self.high_scores,
                        &mut self.rng,
                        input.without_encoder(),
                        now,
                    );
                    self.handle_app_action(action, ActiveApp::Game(ActiveGame::Flags));
                }
                Some(ActiveApp::Game(ActiveGame::Snake)) => {
                    let action = self.snake.update(
                        &mut self.display,
                        &mut self.high_scores,
                        &mut self.rng,
                        input.without_encoder(),
                        now,
                    );
                    self.handle_app_action(action, ActiveApp::Game(ActiveGame::Snake));
                }
                Some(ActiveApp::Game(ActiveGame::Game2048)) => {
                    let action = self.game2048.update(
                        &mut self.display,
                        &mut self.high_scores,
                        &mut self.rng,
                        input.without_encoder(),
                        now,
                    );
                    self.handle_app_action(action, ActiveApp::Game(ActiveGame::Game2048));
                }
                Some(ActiveApp::Game(ActiveGame::Tetris)) => {
                    let action = self.tetris.update(
                        &mut self.display,
                        &mut self.rng,
                        input.without_encoder(),
                        now,
                    );
                    self.handle_app_action(action, ActiveApp::Game(ActiveGame::Tetris));
                }
                #[cfg(feature = "flappy")]
                Some(ActiveApp::Game(ActiveGame::Flappy)) => {
                    let action = self.flappy.update(
                        &mut self.display,
                        &mut self.high_scores,
                        &mut self.rng,
                        &mut self.speaker,
                        input.without_encoder(),
                        now,
                    );
                    self.handle_app_action(action, ActiveApp::Game(ActiveGame::Flappy));
                }
                Some(ActiveApp::Tool(ActiveTool::Pomodoro)) => {
                    let action = self.pomodoro.update(
                        &mut self.display,
                        &mut self.speaker,
                        &mut self.pomodoro_settings,
                        input,
                        now,
                    );
                    self.handle_app_action(action, ActiveApp::Tool(ActiveTool::Pomodoro));
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
            GAME_TITLES,
            APP_TITLES,
            self.launcher.view(),
            self.launcher.selected_index(),
            self.launcher.home_selection(),
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
            LauncherAction::EnterGame(index) => {
                let app = match index {
                    0 => ActiveApp::Game(ActiveGame::Flags),
                    1 => ActiveApp::Game(ActiveGame::Snake),
                    2 => ActiveApp::Game(ActiveGame::Game2048),
                    3 => ActiveApp::Game(ActiveGame::Tetris),
                    #[cfg(feature = "flappy")]
                    4 => ActiveApp::Game(ActiveGame::Flappy),
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
                    ActiveApp::Game(ActiveGame::Flags) => {
                        self.flags.enter(&self.high_scores);
                        self.flags.render_full(&mut self.display);
                    }
                    ActiveApp::Game(ActiveGame::Snake) => {
                        self.snake.enter(&self.high_scores);
                        self.snake.render_full(&mut self.display);
                    }
                    ActiveApp::Game(ActiveGame::Game2048) => {
                        self.game2048.enter(&self.high_scores);
                        self.game2048.render_full(&mut self.display);
                    }
                    ActiveApp::Game(ActiveGame::Tetris) => {
                        self.tetris.enter();
                        self.tetris.render_full(&mut self.display);
                    }
                    #[cfg(feature = "flappy")]
                    ActiveApp::Game(ActiveGame::Flappy) => {
                        self.flappy.enter(&self.high_scores);
                        self.flappy.render_full(&mut self.display);
                    }
                    ActiveApp::Tool(_) => {}
                }
            }
            LauncherAction::EnterApp(index) => {
                let app = match index {
                    0 => ActiveApp::Tool(ActiveTool::Pomodoro),
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
                    ActiveApp::Tool(ActiveTool::Pomodoro) => {
                        self.pomodoro.enter(&self.pomodoro_settings);
                        self.pomodoro.render_full(&mut self.display);
                    }
                    ActiveApp::Game(_) => {}
                }
            }
        }
    }

    fn handle_app_action(&mut self, action: AppAction, app: ActiveApp) {
        match action {
            AppAction::None => {}
            AppAction::RedrawFull => match app {
                ActiveApp::Game(ActiveGame::Flags) => self.flags.render_full(&mut self.display),
                ActiveApp::Game(ActiveGame::Snake) => self.snake.render_full(&mut self.display),
                ActiveApp::Game(ActiveGame::Game2048) => {
                    self.game2048.render_full(&mut self.display)
                }
                ActiveApp::Game(ActiveGame::Tetris) => self.tetris.render_full(&mut self.display),
                #[cfg(feature = "flappy")]
                ActiveApp::Game(ActiveGame::Flappy) => self.flappy.render_full(&mut self.display),
                ActiveApp::Tool(ActiveTool::Pomodoro) => {
                    self.pomodoro.render_full(&mut self.display)
                }
            },
            AppAction::ExitToLauncher => {
                self.active_app = None;
                match app {
                    ActiveApp::Game(_) => self.launcher.show_game_menu(),
                    ActiveApp::Tool(_) => self.launcher.show_app_menu(),
                }
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
