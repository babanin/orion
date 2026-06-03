#[cfg(target_os = "espidf")]
mod display;
#[cfg(target_os = "espidf")]
mod encoder;
#[cfg(target_os = "espidf")]
mod esp_rng;
#[cfg(target_os = "espidf")]
mod hardware;
#[cfg(target_os = "espidf")]
mod joystick;
#[cfg(target_os = "espidf")]
mod network;
#[cfg(target_os = "espidf")]
mod nvs_store;
#[cfg(target_os = "espidf")]
mod runtime;
#[cfg(target_os = "espidf")]
mod speaker;

#[cfg(target_os = "espidf")]
fn main() {
    boot_log("orion: app_main\n");
    esp_idf_sys::link_patches();
    boot_log("orion: patches linked\n");

    boot_log("orion: runtime new\n");
    let mut runtime = Box::new(runtime::OrionRuntime::new());
    boot_log("orion: runtime init\n");
    if runtime.init().is_err() {
        boot_log("orion: runtime init failed\n");
        loop {
            unsafe {
                esp_idf_sys::vTaskDelay(esp_idf_sys::configTICK_RATE_HZ);
            }
        }
    }
    boot_log("orion: runtime run\n");
    runtime.run();
}

#[cfg(target_os = "espidf")]
fn boot_log(message: &str) {
    let bytes = message.as_bytes();
    for byte in bytes {
        unsafe {
            esp_idf_sys::esp_rom_printf(b"%c\0".as_ptr().cast(), *byte as i32);
        }
    }
}

#[cfg(not(target_os = "espidf"))]
fn main() {
    println!("orion-firmware is only runnable on the xtensa-esp32s3-espidf target");
}
