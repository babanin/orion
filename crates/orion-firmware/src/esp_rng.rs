use orion_core::Rng;

pub struct EspRng;

impl Rng for EspRng {
    fn next_u32(&mut self) -> u32 {
        unsafe { esp_idf_sys::esp_random() }
    }
}
