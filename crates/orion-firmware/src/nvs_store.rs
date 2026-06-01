use esp_idf_sys as sys;
use orion_core::{
    game2048::GridSize, high_score_index, high_score_key, game2048_score_key,
    BorderMode, HighScoreStore, SpeedTier, HIGH_SCORE_BUCKET_COUNT, GAME2048_SCORE_BUCKET_COUNT,
};

const NVS_NAME_LEN: usize = 16;

pub struct NvsHighScoreStore {
    snake: [u32; HIGH_SCORE_BUCKET_COUNT],
    flags_death_match: u32,
    game2048: [u32; GAME2048_SCORE_BUCKET_COUNT],
}

impl NvsHighScoreStore {
    pub const fn new() -> Self {
        Self {
            snake: [0; HIGH_SCORE_BUCKET_COUNT],
            flags_death_match: 0,
            game2048: [0; GAME2048_SCORE_BUCKET_COUNT],
        }
    }

    pub fn init(&mut self) -> Result<(), sys::EspError> {
        unsafe {
            let mut err = sys::nvs_flash_init();
            if err == sys::ESP_ERR_NVS_NO_FREE_PAGES || err == sys::ESP_ERR_NVS_NEW_VERSION_FOUND {
                sys::esp!(sys::nvs_flash_erase())?;
                err = sys::nvs_flash_init();
            }
            sys::esp!(err)?;
        }

        self.load_snake()?;
        self.load_flags()?;
        self.load_game2048()?;
        Ok(())
    }

    fn load_snake(&mut self) -> Result<(), sys::EspError> {
        let handle = open_namespace("snake")?;
        for index in 0..HIGH_SCORE_BUCKET_COUNT {
            self.snake[index] = get_u32(handle, key_name(index))?.unwrap_or(0);
        }

        if let Some(legacy_best) = get_u32(handle, "best_score")? {
            let index = high_score_index(SpeedTier::Normal, BorderMode::Borders);
            if legacy_best > self.snake[index] {
                self.snake[index] = legacy_best;
                set_u32(handle, key_name(index), legacy_best)?;
            }
            let legacy_key = c_name("best_score");
            unsafe {
                let err = sys::nvs_erase_key(handle, legacy_key.as_ptr());
                if err == sys::ESP_OK || err == sys::ESP_ERR_NVS_NOT_FOUND {
                    sys::esp!(sys::nvs_commit(handle))?;
                } else {
                    sys::esp!(err)?;
                }
            }
        }
        unsafe {
            sys::nvs_close(handle);
        }
        Ok(())
    }

    fn load_flags(&mut self) -> Result<(), sys::EspError> {
        let handle = open_namespace("flags")?;
        self.flags_death_match = get_u32(handle, "death_best")?.unwrap_or(0);
        unsafe {
            sys::nvs_close(handle);
        }
        Ok(())
    }

    fn load_game2048(&mut self) -> Result<(), sys::EspError> {
        let handle = open_namespace("game2048")?;
        for index in 0..GAME2048_SCORE_BUCKET_COUNT {
            self.game2048[index] = get_u32(handle, game2048_score_key(index))?.unwrap_or(0);
        }
        unsafe {
            sys::nvs_close(handle);
        }
        Ok(())
    }

    fn save_snake_score(&mut self, index: usize, score: u32) -> Result<(), sys::EspError> {
        if score <= self.snake[index] {
            return Ok(());
        }
        self.snake[index] = score;
        let handle = open_namespace("snake")?;
        let result = set_u32(handle, key_name(index), score);
        unsafe {
            sys::nvs_close(handle);
        }
        result
    }

    fn save_flags_score(&mut self, score: u32) -> Result<(), sys::EspError> {
        if score <= self.flags_death_match {
            return Ok(());
        }
        self.flags_death_match = score;
        let handle = open_namespace("flags")?;
        let result = set_u32(handle, "death_best", score);
        unsafe {
            sys::nvs_close(handle);
        }
        result
    }

    fn save_game2048_score(&mut self, index: usize, score: u32) -> Result<(), sys::EspError> {
        if score <= self.game2048[index] {
            return Ok(());
        }
        self.game2048[index] = score;
        let handle = open_namespace("game2048")?;
        let result = set_u32(handle, game2048_score_key(index), score);
        unsafe {
            sys::nvs_close(handle);
        }
        result
    }
}

impl Default for NvsHighScoreStore {
    fn default() -> Self {
        Self::new()
    }
}

impl HighScoreStore for NvsHighScoreStore {
    fn best_score(&self, speed: SpeedTier, border: BorderMode) -> u32 {
        self.snake[high_score_index(speed, border)]
    }

    fn update_if_better(&mut self, score: u32, speed: SpeedTier, border: BorderMode) {
        let index = high_score_index(speed, border);
        let _ = self.save_snake_score(index, score);
    }

    fn flags_death_match_best_score(&self) -> u32 {
        self.flags_death_match
    }

    fn update_flags_death_match_best_score(&mut self, score: u32) {
        let _ = self.save_flags_score(score);
    }

    fn game2048_best_score(&self, grid_size: GridSize) -> u32 {
        self.game2048[grid_size.index()]
    }

    fn update_game2048_best_score(&mut self, score: u32, grid_size: GridSize) {
        let _ = self.save_game2048_score(grid_size.index(), score);
    }
}

fn open_namespace(namespace: &str) -> Result<sys::nvs_handle_t, sys::EspError> {
    let namespace = c_name(namespace);
    let mut handle = 0;
    unsafe {
        sys::esp!(sys::nvs_open(
            namespace.as_ptr(),
            sys::nvs_open_mode_t_NVS_READWRITE,
            &mut handle
        ))?;
    }
    Ok(handle)
}

fn get_u32(handle: sys::nvs_handle_t, key: &str) -> Result<Option<u32>, sys::EspError> {
    let key = c_name(key);
    let mut value = 0;
    let err = unsafe { sys::nvs_get_u32(handle, key.as_ptr(), &mut value) };
    if err == sys::ESP_ERR_NVS_NOT_FOUND {
        Ok(None)
    } else {
        sys::EspError::convert(err).map(|_| Some(value))
    }
}

fn set_u32(handle: sys::nvs_handle_t, key: &str, value: u32) -> Result<(), sys::EspError> {
    let key = c_name(key);
    unsafe {
        sys::esp!(sys::nvs_set_u32(handle, key.as_ptr(), value))?;
        sys::esp!(sys::nvs_commit(handle))
    }
}

fn key_name(index: usize) -> &'static str {
    high_score_key(index)
}

fn c_name(name: &str) -> [core::ffi::c_char; NVS_NAME_LEN] {
    let mut out = [0; NVS_NAME_LEN];
    let bytes = name.as_bytes();
    let len = bytes.len().min(NVS_NAME_LEN - 1);
    let mut index = 0;
    while index < len {
        out[index] = bytes[index] as core::ffi::c_char;
        index += 1;
    }
    out
}
