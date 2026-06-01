# Project Notes

This repository is the Rust reimplementation of the Orion ESP32-S3 apps from
`/Users/ivan/projects/github/esp/orion`.

Current status:

- `orion-core` contains host-testable Rust models for launcher flow, Snake,
  Flags, 2048, input decoding, high-score behavior, deterministic RNG, and
  render command recording.
- `orion-firmware` is a minimal ESP-IDF Rust firmware shell that currently proves
  the ESP32-S3 Rust build path.
- Full hardware parity is not implemented yet. Display, ADC joystick, encoder,
  NVS, LCD rendering, and app runtime integration still need to be ported from
  the C++ project.
- `make build` currently succeeds using the ESP-IDF version selected by
  `esp-idf-sys`.
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
  SPI LCD, and FreeRTOS delay code inside `orion-firmware`.
- Avoid heap allocation in gameplay and rendering hot paths unless there is a
  concrete reason. Current test scaffolding may use `Vec` for host-only fakes.
- Prefer small, explicit types and traits over broad global state.
- Do not use exceptions or RTTI concepts; this is Rust firmware.

Behavior to preserve from the C++ project:

- Firmware boots to a top-level menu with `Flags`, `Snake`, and `2048`.
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
- NVS namespaces and keys must stay compatible with the C++ firmware.

Hardware notes:

- Target board: ESP32-S3-DevKitC-1 N16R8.
- Display: 2.8 inch 320x240 ST7789V SPI TFT LCD.
- Joystick: KY-023 / HW-504 analog module.
- Encoder: KY-040 / EC11 quadrature encoder with push button.
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

- `main/flags.bin` is copied from the C++ project and contains generated RGB565
  flag image data.
- `tools/generate_flags_assets.py` generates Rust flag metadata from the C++
  `flags_assets.cpp` table for now.
- Generated Rust flag metadata lives in
  `crates/orion-core/src/generated/flags_assets.rs`.
- Commit source, `Cargo.toml`, `Cargo.lock`, `Makefile`, `README.md`,
  `AGENTS.md`, `sdkconfig.defaults`, `partitions.csv`, `main/flags.bin`, and
  generated source metadata.
- Do not commit generated build directories: `target/`, `.embuild/`, `build/`,
  or `build-*`.

Local workflow:

- Run host tests with:
  - `make test`
  - or `cargo test -p orion-core`
- Build default ESP-IDF Rust firmware with:
  - `make build`
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
  selection, and 2048 random tile placement.
- Use recording/fake display and fake score stores for host tests.
- Hardware-dependent code in `orion-firmware` should be thin adapters around
  tested `orion-core` behavior.
- Before handing off work, run at least `make test`; run `make build` whenever
  firmware-facing code or Cargo configuration changes.

