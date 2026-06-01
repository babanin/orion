use core::ffi::c_char;
use core::ptr;
use core::str;

use esp_idf_sys as sys;
use orion_core::{CalendarDate, ClockTime, HomeSnapshot, HomeStatus};

use crate::nvs_store::{load_or_seed_wifi_credentials, WifiCredentials};

const CONNECT_RETRY_US: i64 = 10_000_000;
const WEATHER_REFRESH_US: i64 = 10 * 60 * 1_000_000;
const CLOCK_REFRESH_US: i64 = 60 * 1_000_000;
const WEATHER_URL: &[u8] = b"https://api.open-meteo.com/v1/forecast?latitude=59.9386&longitude=30.3141&current=temperature_2m&timezone=Europe%2FMoscow\0";
const NTP_SERVER: &[u8] = b"pool.ntp.org\0";
const TZ_NAME: &[u8] = b"TZ\0";
const TZ_MOSCOW: &[u8] = b"MSK-3\0";

#[derive(Debug)]
pub struct NetworkManager {
    credentials: Option<WifiCredentials>,
    wifi_initialized: bool,
    sntp_started: bool,
    next_connect_us: i64,
    next_weather_us: i64,
    next_clock_us: i64,
    last_temperature: Option<i16>,
    snapshot: HomeSnapshot,
}

impl NetworkManager {
    pub const fn new() -> Self {
        Self {
            credentials: None,
            wifi_initialized: false,
            sntp_started: false,
            next_connect_us: 0,
            next_weather_us: 0,
            next_clock_us: 0,
            last_temperature: None,
            snapshot: HomeSnapshot::placeholders(),
        }
    }

    pub fn init(&mut self, now_us: i64) {
        set_timezone();
        self.credentials = load_or_seed_wifi_credentials().unwrap_or(None);
        if let Some(credentials) = self.credentials {
            if init_wifi(credentials).is_ok() {
                self.wifi_initialized = true;
                self.next_connect_us = now_us;
                self.snapshot.status = HomeStatus::Time;
            }
        }
        self.refresh_clock(now_us);
    }

    pub fn update(&mut self, now_us: i64) -> bool {
        let before = self.snapshot;
        if !self.wifi_initialized {
            self.snapshot.status = HomeStatus::Wifi;
            self.refresh_clock(now_us);
            return self.snapshot != before;
        }

        let connected = wifi_connected();
        if !connected && now_us >= self.next_connect_us {
            unsafe {
                let _ = sys::esp_wifi_connect();
            }
            self.next_connect_us = now_us + CONNECT_RETRY_US;
        }

        if connected && !self.sntp_started {
            start_sntp();
            self.sntp_started = true;
        }

        if now_us >= self.next_clock_us {
            self.refresh_clock(now_us);
        }

        if connected && now_us >= self.next_weather_us {
            if let Some(temp) = fetch_temperature_tenths() {
                self.last_temperature = Some(temp);
                self.snapshot.temperature_tenths_c = Some(temp);
                self.next_weather_us = now_us + WEATHER_REFRESH_US;
            } else {
                self.next_weather_us = now_us + CONNECT_RETRY_US;
            }
        }

        self.snapshot.status = if !connected {
            HomeStatus::Wifi
        } else if self.snapshot.time.is_none() {
            HomeStatus::Time
        } else if self.snapshot.temperature_tenths_c.is_none() {
            HomeStatus::Weather
        } else {
            HomeStatus::Ready
        };
        self.snapshot != before
    }

    pub const fn snapshot(&self) -> HomeSnapshot {
        self.snapshot
    }

    fn refresh_clock(&mut self, now_us: i64) {
        self.next_clock_us = now_us + CLOCK_REFRESH_US;
        if let Some((time, date)) = local_datetime() {
            self.snapshot.time = Some(time);
            self.snapshot.date = Some(date);
        }
        self.snapshot.temperature_tenths_c = self.last_temperature;
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

fn set_timezone() {
    unsafe {
        let _ = sys::setenv(TZ_NAME.as_ptr().cast(), TZ_MOSCOW.as_ptr().cast(), 1);
        sys::tzset();
    }
}

fn init_wifi(credentials: WifiCredentials) -> Result<(), sys::EspError> {
    unsafe {
        sys::esp!(sys::esp_netif_init())?;
        let err = sys::esp_event_loop_create_default();
        if err != sys::ESP_OK && err != sys::ESP_ERR_INVALID_STATE {
            sys::esp!(err)?;
        }
        let _ = sys::esp_netif_create_default_wifi_sta();

        let config = sys::orion_wifi_init_config_default();
        sys::esp!(sys::esp_wifi_init(&config))?;
        sys::esp!(sys::esp_wifi_set_mode(sys::wifi_mode_t_WIFI_MODE_STA))?;

        let mut wifi_config = sys::wifi_config_t::default();
        copy_bytes(&mut wifi_config.sta.ssid, credentials.ssid());
        copy_bytes(&mut wifi_config.sta.password, credentials.password());
        sys::esp!(sys::esp_wifi_set_config(
            sys::wifi_interface_t_WIFI_IF_STA,
            &mut wifi_config
        ))?;
        sys::esp!(sys::esp_wifi_start())?;
        let _ = sys::esp_wifi_connect();
    }
    Ok(())
}

fn copy_bytes<const N: usize>(out: &mut [u8; N], input: &[u8]) {
    let len = input.len().min(N);
    out[..len].copy_from_slice(&input[..len]);
}

fn wifi_connected() -> bool {
    unsafe {
        let mut ap = sys::wifi_ap_record_t::default();
        sys::esp_wifi_sta_get_ap_info(&mut ap) == sys::ESP_OK
    }
}

fn start_sntp() {
    unsafe {
        if sys::esp_sntp_enabled() {
            return;
        }
        sys::esp_sntp_setoperatingmode(sys::esp_sntp_operatingmode_t_ESP_SNTP_OPMODE_POLL);
        sys::esp_sntp_setservername(0, NTP_SERVER.as_ptr().cast());
        sys::esp_sntp_init();
    }
}

fn local_datetime() -> Option<(ClockTime, CalendarDate)> {
    unsafe {
        let now = sys::time(ptr::null_mut());
        if now < 1_700_000_000 {
            return None;
        }
        let mut tm = sys::tm::default();
        if sys::localtime_r(&now, &mut tm).is_null() {
            return None;
        }
        Some((
            ClockTime::new(tm.tm_hour as u8, tm.tm_min as u8),
            CalendarDate::new(
                (tm.tm_year + 1900) as u16,
                (tm.tm_mon + 1) as u8,
                tm.tm_mday as u8,
            ),
        ))
    }
}

fn fetch_temperature_tenths() -> Option<i16> {
    unsafe {
        let config = sys::esp_http_client_config_t {
            url: WEATHER_URL.as_ptr().cast(),
            method: sys::esp_http_client_method_t_HTTP_METHOD_GET,
            timeout_ms: 5_000,
            buffer_size: 768,
            crt_bundle_attach: Some(sys::esp_crt_bundle_attach),
            ..Default::default()
        };
        let client = sys::esp_http_client_init(&config);
        if client.is_null() {
            return None;
        }
        let result = fetch_temperature_with_client(client);
        let _ = sys::esp_http_client_cleanup(client);
        result
    }
}

unsafe fn fetch_temperature_with_client(client: sys::esp_http_client_handle_t) -> Option<i16> {
    if sys::esp_http_client_open(client, 0) != sys::ESP_OK {
        return None;
    }
    let result = read_temperature_response(client);
    let _ = sys::esp_http_client_close(client);
    result
}

unsafe fn read_temperature_response(client: sys::esp_http_client_handle_t) -> Option<i16> {
    if sys::esp_http_client_fetch_headers(client) < 0 {
        return None;
    }
    if sys::esp_http_client_get_status_code(client) != 200 {
        return None;
    }
    let mut buf = [0 as c_char; 768];
    let len = sys::esp_http_client_read_response(client, buf.as_mut_ptr(), buf.len() as i32);
    if len <= 0 {
        return None;
    }
    let bytes = core::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), len as usize);
    let response = str::from_utf8(bytes).ok()?;
    parse_temperature_tenths(response)
}

fn parse_temperature_tenths(response: &str) -> Option<i16> {
    let key = "\"temperature_2m\":";
    let mut search = response;
    loop {
        let start = search.find(key)? + key.len();
        let value = parse_temperature_value(&search[start..]);
        if value.is_some() {
            return value;
        }
        search = &search[start..];
    }
}

fn parse_temperature_value(input: &str) -> Option<i16> {
    let mut value = 0_i16;
    let mut sign = 1_i16;
    let mut tenths = 0_i16;
    let mut seen_digit = false;
    let mut after_dot = false;
    for byte in input.as_bytes().iter().copied() {
        match byte {
            b' ' | b'\n' | b'\r' | b'\t' if !seen_digit => {}
            b'-' if !seen_digit => sign = -1,
            b'0'..=b'9' => {
                seen_digit = true;
                if after_dot {
                    tenths = (byte - b'0') as i16;
                    break;
                }
                value = value
                    .saturating_mul(10)
                    .saturating_add((byte - b'0') as i16);
            }
            b'.' => after_dot = true,
            _ => {
                break;
            }
        }
    }
    if seen_digit {
        Some(sign * (value * 10 + tenths))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_open_meteo_temperature() {
        assert_eq!(
            parse_temperature_tenths(r#"{"current":{"temperature_2m":-4.7}}"#),
            Some(-47)
        );
        assert_eq!(
            parse_temperature_tenths(r#"{"current":{"temperature_2m":12}}"#),
            Some(120)
        );
        assert_eq!(
            parse_temperature_tenths(
                r#"{"current_units":{"temperature_2m":"C"},"current":{"time":"2026-06-01T13:00","temperature_2m":18.4}}"#
            ),
            Some(184)
        );
    }
}
