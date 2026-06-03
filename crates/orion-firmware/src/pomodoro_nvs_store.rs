use esp_idf_sys as sys;
use orion_core::PomodoroSettingsStore;

pub struct NvsPomodoroSettingsStore {
    minutes: u8,
    seconds: u8,
}

impl NvsPomodoroSettingsStore {
    pub const fn new() -> Self {
        Self {
            minutes: 25,
            seconds: 0,
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
        self.load()?;
        Ok(())
    }

    fn load(&mut self) -> Result<(), sys::EspError> {
        let handle = open_namespace("pomodoro")?;
        self.minutes = get_u32(handle, "minutes")?.unwrap_or(25) as u8;
        self.seconds = get_u32(handle, "seconds")?.unwrap_or(0) as u8;
        unsafe {
            sys::nvs_close(handle);
        }
        Ok(())
    }

    fn save(&mut self, minutes: u8, seconds: u8) -> Result<(), sys::EspError> {
        self.minutes = minutes;
        self.seconds = seconds;
        let handle = open_namespace("pomodoro")?;
        set_u32(handle, "minutes", minutes as u32)?;
        set_u32(handle, "seconds", seconds as u32)?;
        unsafe {
            sys::nvs_close(handle);
        }
        Ok(())
    }
}

impl Default for NvsPomodoroSettingsStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PomodoroSettingsStore for NvsPomodoroSettingsStore {
    fn pomodoro_minutes(&self) -> u8 {
        self.minutes
    }

    fn pomodoro_seconds(&self) -> u8 {
        self.seconds
    }

    fn save_pomodoro_settings(&mut self, minutes: u8, seconds: u8) {
        let _ = self.save(minutes, seconds);
    }
}

const NVS_NAME_LEN: usize = 16;

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
