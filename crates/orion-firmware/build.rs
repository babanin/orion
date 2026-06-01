fn main() {
    println!("cargo:rerun-if-env-changed=ORION_WIFI_SSID");
    println!("cargo:rerun-if-env-changed=ORION_WIFI_PASSWORD");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("espidf") {
        embuild::espidf::sysenv::output();
    }
}
