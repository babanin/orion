# Orion Rust

Rust reimplementation of the Orion ESP32-S3 applications from
[`../orion`](../orion). The goal is full parity with the current C++ firmware,
while keeping gameplay and input behavior testable on the host.

## Current Status

- `crates/orion-core` contains tested Rust models for:
  - Home screen and games menu flow
  - Snake game state, movement, scoring, wrap/border behavior
  - Flags mode flow, answer selection, quiz/death-match behavior
  - 2048 tile sliding, merging, scoring, grid-size selection
  - Tetris piece movement, rotation, gravity, line clears, and game-over flow
  - joystick thresholds, button debounce, encoder decoding
  - high-score key behavior and recording display commands
- `crates/orion-firmware` contains ESP-IDF adapters for display, input, NVS,
  runtime app integration, Wi-Fi, SNTP time, and Open-Meteo weather.
- `make build` builds the size-optimized `xtensa-esp32s3-espidf` firmware using
  the ESP-IDF version selected by `esp-idf-sys`.
- Hardware parity is still pending for some deeper C++ behavior, but the main
  display/input/runtime path is now implemented in Rust firmware.

## Project Layout

```text
.
├── crates/
│   ├── orion-core/       # host-testable app/game/input logic
│   └── orion-firmware/   # ESP-IDF binary and hardware adapters
├── main/flags.bin        # source RGB565 flag image payload copied from ../orion
├── main/flags.rle        # generated compressed flag payload used by firmware
├── tools/                # asset metadata generator
├── partitions.csv        # parity partition table from ../orion
├── sdkconfig.defaults    # ESP32-S3 defaults
├── rust-toolchain.toml   # pins the esp Rust toolchain
└── Makefile              # routine build/test/flash commands
```

## Tooling

Required local tools:

- Rust + Cargo via `rustup`
- esp-rs toolchain installed by `espup`
- `ldproxy`
- `cargo-espflash`

This project pins the Rust toolchain to `esp` in `rust-toolchain.toml`.
The firmware target is `xtensa-esp32s3-espidf`.

## Common Commands

Run host tests:

```sh
make test
```

Build the firmware:

```sh
make build
```

`make build` defaults to the release profile and layers
`sdkconfig.release.defaults` over `sdkconfig.defaults`. Use `make build-debug`
for debug bring-up builds.

Build optional OM NOM / Flappy firmware:

```sh
make FEATURES=flappy build
```

Wi-Fi defaults are hardcoded in the Makefile for local development:
`WIFI_SSID=Murlo` and `WIFI_PASSWORD=kotopes4WiFi`. `make build`, `make flash`,
and `make flash-monitor` pass these values into the firmware build as
`ORION_WIFI_SSID` and `ORION_WIFI_PASSWORD`.

On first boot, the firmware seeds these credentials into ESP32 NVS namespace
`wifi`, keys `ssid` and `pass`. Later boots read NVS and do not require the
build-time values unless NVS is erased. Override them when needed:

```sh
make WIFI_SSID='other-ssid' WIFI_PASSWORD='other-password' flash PORT=/dev/cu.usbmodemXXXX
```

Try the local ESP-IDF 6.0.1 compatibility build:

```sh
make build-idf6
```

List likely ESP32 serial ports:

```sh
make ports
```

Flash and monitor:

```sh
make flash PORT=/dev/cu.usbmodemXXXX
make monitor PORT=/dev/cu.usbmodemXXXX
make flash-monitor PORT=/dev/cu.usbmodemXXXX
```

## Hardware Target

- ESP32-S3-DevKitC-1 N16R8
- KY-023 / HW-504 analog joystick
- 2.8 inch 320x240 ST7789V SPI TFT LCD
- Optional KY-040 / EC11 rotary encoder with push button

The Rust firmware should preserve the C++ project's hardware behavior and NVS
compatibility. 2048 best scores use NVS namespace `game2048` with keys
`best_3x3`, `best_4x4`, and `best_5x5` per grid size. Tetris currently does
not persist scores. Keep pin choices centralized when the hardware adapters are
implemented, and keep this README aligned with those constants.

## Home Screen

The firmware boots to a Home screen showing Saint Petersburg time, date, and
temperature. Time is synchronized with SNTP using Moscow timezone (`MSK-3`).
Temperature is fetched from Open-Meteo for Saint Petersburg and refreshes every
10 minutes while Wi-Fi is connected.

If Wi-Fi, time sync, or weather fetch is unavailable, the Home screen stays
usable and shows placeholders/status text. Press the KY-023 switch or the
encoder switch on Home to open the games menu. In the games menu, use joystick
up/down or encoder rotation to select a game, press switch to open it, and long
press switch to return Home.

## Tetris Controls

Tetris renders into a 240x320 portrait surface rotated 90 degrees clockwise onto
the landscape display. During play:

- Joystick left/right moves the active piece.
- Joystick down soft-drops the piece.
- Short joystick or encoder switch press rotates the piece.
- Long joystick or encoder switch press pauses.
- Encoder rotation moves the active piece horizontally.

## Wiring

Use a common ground for all modules. Power modules from `3V3`; ESP32-S3 GPIO is
3.3 V logic.

| Module | Module Pin | ESP32-S3 DevKitC-1 Pin | Notes |
|---|---:|---:|---|
| ST7789V TFT | VCC | 3V3 | Display power |
| ST7789V TFT | GND | GND | Common ground |
| ST7789V TFT | SCL / SCK / CLK | GPIO12 | SPI clock |
| ST7789V TFT | SDA / MOSI / DIN | GPIO11 | SPI MOSI |
| ST7789V TFT | SDO / MISO / DO | GPIO13 | SPI MISO; optional |
| ST7789V TFT | CS | GPIO10 | SPI chip select |
| ST7789V TFT | DC / A0 | GPIO9 | Data/command |
| ST7789V TFT | RST / RES | GPIO8 | Reset |
| ST7789V TFT | BL / LED | 3V3 | Always on by default |
| KY-023 | + / VCC | 3V3 | Joystick power |
| KY-023 | GND | GND | Common ground |
| KY-023 | VRx | GPIO1 | ADC1_CH0 |
| KY-023 | VRy | GPIO2 | ADC1_CH1 |
| KY-023 | SW | GPIO4 | Input pullup, active-low |
| KY-040 / EC11 | + / VCC | 3V3 | Encoder power |
| KY-040 / EC11 | GND | GND | Common ground |
| KY-040 / EC11 | CLK / S1 / A | GPIO5 | Input pullup |
| KY-040 / EC11 | DT / S2 / B | GPIO6 | Input pullup |
| KY-040 / EC11 | SW / KEY | GPIO7 | Input pullup, active-low |

Avoid GPIO19 and GPIO20 because they are commonly used for native USB on
ESP32-S3 boards.

## Testing Model

`orion-core` should remain platform independent. Add tests there for gameplay,
input state, scoring, renderer command behavior, and persistence-key logic.

Current test coverage includes host unit tests for the launcher, input, Snake,
Flags, 2048, Tetris, rendering helpers, score stores, the flag RLE decoder, and
weather response parsing. Run them with:

```sh
cargo test -p orion-core
```

Firmware code in `orion-firmware` should stay as thin as possible: ESP-IDF
drivers, NVS, LCD transfers, FreeRTOS delays, and adapters into `orion-core`.
Use `make size-check` before merging firmware-facing changes; it fails when
release `text + data` exceeds `SIZE_BUDGET`.

## ESP-IDF Notes

`make build` uses the managed ESP-IDF selected by `esp-idf-sys` and currently
builds successfully.

`make build-idf6` sources `/Users/ivan/.espressif/v6.0.1/esp-idf/export.sh`.
At scaffold time this reached ESP-IDF 6.0.1 configuration, but
`esp-idf-sys 0.37.2` bindgen failed against the local ESP-IDF 6 mbedTLS headers
with `mbedtls/aes.h` not found. Keep `build-idf6` as the compatibility check
while deciding whether to patch bindings, adjust components, or wait for newer
ESP-IDF 6 support.

## Assets

`main/flags.bin` is the source RGB565 flag payload copied from the C++ project.
`main/flags.rle` is the generated RLE-compressed payload included in the
firmware image.

Generate Rust flag metadata and the compressed payload from the existing C++
asset table:

```sh
python3 tools/generate_flags_assets.py \
  --source ../orion/main/flags_assets.cpp \
  --raw-bin main/flags.bin \
  --compressed-output main/flags.rle \
  --output crates/orion-core/src/generated/flags_assets.rs
```

To regenerate from the checked-in Rust metadata instead of the C++ tree:

```sh
python3 tools/generate_flags_assets.py \
  --source crates/orion-core/src/generated/flags_assets.rs \
  --source-format rust \
  --raw-bin main/flags.bin \
  --compressed-output main/flags.rle \
  --output crates/orion-core/src/generated/flags_assets.rs
```

Commit generated Rust metadata, `main/flags.bin`, and `main/flags.rle` so normal
builds do not need network access.
