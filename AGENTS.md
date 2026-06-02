# Project Notes

This repository is the Rust reimplementation of the Orion ESP32-S3 apps from
`/Users/ivan/projects/github/esp/orion`.

Current status:

- `orion-core` contains host-testable Rust models for launcher flow, Snake,
  Flags, 2048, Tetris, input decoding, high-score behavior, deterministic RNG,
  home/menu UI behavior, and render command recording.
- `orion-firmware` contains ESP-IDF adapters for the ST7789V display, ADC
  joystick, KY-040 / EC11 encoder, NVS high scores, runtime app integration,
  Wi-Fi, SNTP time, Open-Meteo weather, and HW-508 V0.2 speaker via LEDC PWM.
- Full hardware parity is not implemented yet. Some C++ details still need to
  be ported or verified on hardware, especially deeper display parity and any
  remaining gameplay/runtime edge cases.
- `make build` currently builds the release/size-optimized firmware using the
  ESP-IDF version selected by `esp-idf-sys`.
- `make build-idf6` attempts to use local ESP-IDF 6.0.1 from
  `/Users/ivan/.espressif/v6.0.1/esp-idf`; at scaffold time it reaches ESP-IDF
  configuration but fails during `esp-idf-sys 0.37.2` bindgen against local
  ESP-IDF 6 mbedTLS headers.

Development target:

- Rust workspace with two crates:
  - `crates/orion-core`: platform-independent logic and tests.
  - `crates/orion-firmware`: ESP-IDF binary and hardware adapters.
- Keep ESP32-S3 target support through the `esp` Rust toolchain pinned in
  `rust-toolchain.toml`.
- Firmware target is `xtensa-esp32s3-espidf`.
- Prefer keeping `orion-core` free of ESP-IDF dependencies so it remains
  testable with `cargo test -p orion-core`.
- Keep firmware-specific FFI, unsafe code, ESP-IDF driver calls, NVS, GPIO, ADC,
  SPI LCD, Wi-Fi, SNTP, HTTP, and FreeRTOS delay code inside `orion-firmware`.
- `crates/orion-firmware/orion_app_main/wifi_shim.c` exists to expose the
  ESP-IDF `WIFI_INIT_CONFIG_DEFAULT()` macro as a callable C function for Rust.
  Prefer keeping that shim instead of manually reconstructing
  `wifi_init_config_t` in Rust.
- Avoid heap allocation in gameplay and rendering hot paths unless there is a
  concrete reason. Current test scaffolding may use `Vec` for host-only fakes.
- Prefer small, explicit types and traits over broad global state.
- Do not use exceptions or RTTI concepts; this is Rust firmware.

Behavior to preserve from the C++ project:

- Firmware boots to a Home screen showing Saint Petersburg time, date, weather
  temperature, and a `MENU` button. Pressing the KY-023 or encoder switch opens
  the games menu.
- The games menu lists `Flags`, `Snake`, `2048`, and `Tetris` with small icons.
  Joystick up/down or encoder rotation changes selection; switch press opens the
  selected game; long switch press returns from the games menu to Home.
- Snake renders on an ST7789V 320x240 SPI TFT.
- Snake uses KY-023 joystick direction and switch controls.
- Optional KY-040 / EC11 rotary encoder adjusts Snake speed during play and
  changes menu selections.
- Optional KY-040 / EC11 switch mirrors the KY-023 switch.
- Snake best scores persist separately per speed and border mode in ESP32 NVS
  namespace `snake`.
- Flags is a 4-choice flag quiz with `Practice`, `Quiz 20`, and `Death Match`.
- Flags Death Match best score persists in ESP32 NVS namespace `flags`, key
  `death_best`.
- 2048 is a tile-sliding puzzle game with grid sizes 3x3, 4x4, and 5x5.
- 2048 uses joystick direction to slide tiles and switch to start/pause.
- 2048 best scores persist per grid size in ESP32 NVS namespace `game2048`,
  keys `best_3x3`, `best_4x4`, `best_5x5`.
- 2048 uses delta rendering during play (only changed cells are redrawn).
- 2048 uses bilinear-interpolated antialiased font rendering for tile numbers
  at scale 2 (5×5 grids, 10×14 px characters) and scale 3 (3×3 and 4×4 grids,
  15×21 px characters). Each output pixel's coverage is computed by placing its
  center in the scaled glyph space, then bilinearly interpolating the four
  nearest font bitmap bits. The resulting coverage (0–4) is blended with the
  tile background color via `rgb565_blend` to produce smooth edges without
  requiring an alpha channel or per-pixel transparency. Scale-1 bitmap font is
  used for large digit counts on small cells where AA would not fit.
- Tetris is a 10x20 falling-block game rendered as a 240x320 portrait surface
  rotated 90 degrees clockwise onto the landscape ST7789V display.
- Tetris uses joystick left/right to move, joystick down to soft drop, short
  joystick or encoder switch press to rotate, and long joystick or encoder
  switch press to pause during play. Start and menu confirmation still use
  normal switch press.
- Tetris uses delta rendering during play for movement and gravity ticks; avoid
  full-screen redraws on normal piece movement to prevent LCD blinking.
- Tetris currently has no NVS high-score contract. Add one deliberately before
  persisting scores so namespace/key compatibility can be documented.
- NVS namespaces and keys must stay compatible with the C++ firmware.
- Wi-Fi credentials are stored in ESP32 NVS namespace `wifi`, keys `ssid` and
  `pass`. The Makefile currently supplies default build-time values
  `WIFI_SSID=Murlo` and `WIFI_PASSWORD=kotopes4WiFi` to seed NVS on first boot.
- Home screen time comes from SNTP using Moscow timezone (`MSK-3`), and weather
  temperature comes from Open-Meteo for Saint Petersburg, Russia. Weather
  refreshes every 10 minutes when Wi-Fi is connected; offline states should keep
  Home and games usable with placeholders/status text.

Hardware notes:

- Target board: ESP32-S3-DevKitC-1 N16R8.
- Display: 2.8 inch 320x240 ST7789V SPI TFT LCD.
- Joystick: KY-023 / HW-504 analog module.
- Encoder: KY-040 / EC11 quadrature encoder with push button.
- Speaker: HW-508 V0.2 speaker module via LEDC PWM on GPIO14.
- Keep hardware pin choices centralized in firmware constants when hardware
  adapters are implemented.
- Keep README wiring tables aligned with firmware constants.
- KY-023 must be handled as ADC axes plus debounced active-low switch.
- KY-040 / EC11 must be handled as quadrature encoder, separate from KY-023.
- ST7789V SPI LCD transfers are queued asynchronously. Do not reuse or mutate a
  pixel buffer passed to `esp_lcd_panel_draw_bitmap()` until the queue is
  drained. Preserve the C++ firmware's fill-buffer rotation behavior when
  porting the display surface.

Assets and generated files:

- `main/flags.bin` is copied from the C++ project and contains source RGB565
  flag image data.
- `main/flags.rle` is generated from `main/flags.bin` and is the compressed
  payload included in firmware.
- `tools/generate_flags_assets.py` generates Rust flag metadata from the C++
  `flags_assets.cpp` table and can regenerate `main/flags.rle` from
  checked-in Rust metadata.
- Generated Rust flag metadata lives in
  `crates/orion-core/src/generated/flags_assets.rs`.
- Commit source, `Cargo.toml`, `Cargo.lock`, `Makefile`, `README.md`,
  `AGENTS.md`, `sdkconfig.defaults`, `sdkconfig.release.defaults`,
  `partitions.csv`, `main/flags.bin`, `main/flags.rle`, and generated source
  metadata.
- Do not commit generated build directories: `target/`, `.embuild/`, `build/`,
  or `build-*`.

Local workflow:

- Run host tests with:
  - `make test`
  - or `cargo test -p orion-core`
- Build default ESP-IDF Rust firmware with:
  - `make build`
- Build debug ESP-IDF Rust firmware with:
  - `make build-debug`
- Build firmware with OM NOM / Flappy enabled by default with:
  - `make build`
- `make build` and flash targets pass Makefile Wi-Fi defaults into the firmware
  build as `ORION_WIFI_SSID` and `ORION_WIFI_PASSWORD`. Override with
  `make WIFI_SSID=... WIFI_PASSWORD=... build` if needed.
- Try the local ESP-IDF 6.0.1 compatibility build with:
  - `make build-idf6`
- List likely serial ports with:
  - `make ports`
- Flash/monitor with:
  - `make flash PORT=/dev/cu.usbmodemXXXX`
  - `make monitor PORT=/dev/cu.usbmodemXXXX`
  - `make flash-monitor PORT=/dev/cu.usbmodemXXXX`

Testing expectations:

- Add or update `orion-core` tests for every gameplay or input-state change.
- Keep deterministic RNG paths for tests of Snake food placement, Flags answer
  selection, 2048 random tile placement, and Tetris piece selection.
- Use recording/fake display and fake score stores for host tests.
- Hardware-dependent code in `orion-firmware` should be thin adapters around
  tested `orion-core` behavior.
- Before handing off work, run at least `make test`; run `make build` whenever
  firmware-facing code or Cargo configuration changes.
- Run `make size-check` for firmware size-sensitive changes.
