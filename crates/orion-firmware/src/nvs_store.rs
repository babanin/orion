use esp_idf_sys as sys;
use orion_core::{
    game2048::GridSize, game2048_score_key, high_score_index, high_score_key, BorderMode,
    HighScoreStore, SpeedTier, GAME2048_SCORE_BUCKET_COUNT, HIGH_SCORE_BUCKET_COUNT,
};

const NVS_NAME_LEN: usize = 16;
const WIFI_SSID_MAX: usize = 32;
const WIFI_PASS_MAX: usize = 64;
const NVS_WIFI_STR_MAX: usize = WIFI_PASS_MAX + 1;

const DEFAULT_WIFI_SSID: &str = match option_env!("ORION_WIFI_SSID") {
    Some(value) => value,
    None => "Murlo",
};
const DEFAULT_WIFI_PASSWORD: Option<&str> = option_env!("ORION_WIFI_PASSWORD");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WifiCredentials {
    ssid: [u8; WIFI_SSID_MAX],
    ssid_len: usize,
    password: [u8; WIFI_PASS_MAX],
    password_len: usize,
}

impl WifiCredentials {
    pub const fn empty() -> Self {
        Self {
            ssid: [0; WIFI_SSID_MAX],
            ssid_len: 0,
            password: [0; WIFI_PASS_MAX],
            password_len: 0,
        }
    }

    pub fn new(ssid: &str, password: &str) -> Option<Self> {
        if ssid.is_empty()
            || ssid.len() > WIFI_SSID_MAX
            || password.len() >= WIFI_PASS_MAX
            || password.as_bytes().contains(&0)
            || ssid.as_bytes().contains(&0)
        {
            return None;
        }

        let mut credentials = Self::empty();
        credentials.ssid[..ssid.len()].copy_from_slice(ssid.as_bytes());
        credentials.ssid_len = ssid.len();
        credentials.password[..password.len()].copy_from_slice(password.as_bytes());
        credentials.password_len = password.len();
        Some(credentials)
    }

    pub fn ssid(&self) -> &[u8] {
        &self.ssid[..self.ssid_len]
    }

    pub fn password(&self) -> &[u8] {
        &self.password[..self.password_len]
    }
}

pub struct NvsHighScoreStore {
    snake: [u32; HIGH_SCORE_BUCKET_COUNT],
    flags_death_match: u32,
    game2048: [u32; GAME2048_SCORE_BUCKET_COUNT],
    #[cfg(feature = "flappy")]
    flappy: u32,
}

impl NvsHighScoreStore {
    pub const fn new() -> Self {
        Self {
            snake: [0; HIGH_SCORE_BUCKET_COUNT],
            flags_death_match: 0,
            game2048: [0; GAME2048_SCORE_BUCKET_COUNT],
            #[cfg(feature = "flappy")]
            flappy: 0,
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
        #[cfg(feature = "flappy")]
        self.load_flappy()?;
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

    #[cfg(feature = "flappy")]
    fn load_flappy(&mut self) -> Result<(), sys::EspError> {
        let handle = open_namespace("flappy")?;
        self.flappy = get_u32(handle, "best_score")?.unwrap_or(0);
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

    #[cfg(feature = "flappy")]
    fn save_flappy_score(&mut self, score: u32) -> Result<(), sys::EspError> {
        if score <= self.flappy {
            return Ok(());
        }
        self.flappy = score;
        let handle = open_namespace("flappy")?;
        let result = set_u32(handle, "best_score", score);
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

    #[cfg(feature = "flappy")]
    fn flappy_best_score(&self) -> u32 {
        self.flappy
    }

    #[cfg(feature = "flappy")]
    fn update_flappy_best_score(&mut self, score: u32) {
        let _ = self.save_flappy_score(score);
    }
}

pub fn load_or_seed_wifi_credentials() -> Result<Option<WifiCredentials>, sys::EspError> {
    let handle = open_namespace("wifi")?;
    let result = (|| {
        let loaded = read_wifi_credentials(handle)?;
        if loaded.is_some() {
            Ok(loaded)
        } else if let Some(password) = DEFAULT_WIFI_PASSWORD {
            let seeded = WifiCredentials::new(DEFAULT_WIFI_SSID, password);
            if let Some(credentials) = seeded {
                set_str(handle, "ssid", DEFAULT_WIFI_SSID)?;
                set_str(handle, "pass", password)?;
                Ok(Some(credentials))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    })();
    unsafe {
        sys::nvs_close(handle);
    }
    result
}

fn read_wifi_credentials(
    handle: sys::nvs_handle_t,
) -> Result<Option<WifiCredentials>, sys::EspError> {
    let mut ssid = [0_u8; WIFI_SSID_MAX];
    let mut password = [0_u8; WIFI_PASS_MAX];
    let Some(ssid_len) = get_str(handle, "ssid", &mut ssid)? else {
        return Ok(None);
    };
    let Some(password_len) = get_str(handle, "pass", &mut password)? else {
        return Ok(None);
    };
    if ssid_len == 0 {
        return Ok(None);
    }
    Ok(Some(WifiCredentials {
        ssid,
        ssid_len,
        password,
        password_len,
    }))
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

fn get_str(
    handle: sys::nvs_handle_t,
    key: &str,
    out: &mut [u8],
) -> Result<Option<usize>, sys::EspError> {
    let key = c_name(key);
    let mut buf = [0 as core::ffi::c_char; NVS_WIFI_STR_MAX];
    let mut len = buf.len();
    let err = unsafe { sys::nvs_get_str(handle, key.as_ptr(), buf.as_mut_ptr(), &mut len) };
    if err == sys::ESP_ERR_NVS_NOT_FOUND {
        return Ok(None);
    }
    sys::EspError::convert(err)?;
    if len == 0 || len - 1 > out.len() {
        return Ok(None);
    }
    for index in 0..len - 1 {
        out[index] = buf[index] as u8;
    }
    Ok(Some(len - 1))
}

fn set_str(handle: sys::nvs_handle_t, key: &str, value: &str) -> Result<(), sys::EspError> {
    let key = c_name(key);
    let value = c_value(value);
    unsafe {
        sys::esp!(sys::nvs_set_str(handle, key.as_ptr(), value.as_ptr()))?;
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

fn c_value(value: &str) -> [core::ffi::c_char; NVS_WIFI_STR_MAX] {
    let mut out = [0; NVS_WIFI_STR_MAX];
    let bytes = value.as_bytes();
    let len = bytes.len().min(NVS_WIFI_STR_MAX - 1);
    let mut index = 0;
    while index < len {
        out[index] = bytes[index] as core::ffi::c_char;
        index += 1;
    }
    out
}
