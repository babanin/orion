# Orion Rust

Rust reimplementation of the Orion ESP32-S3 applications from
[`../orion`](../orion). The goal is full parity with the current C++ firmware,
while keeping gameplay and input behavior testable on the host.

## Current Status

- `crates/orion-core` contains tested Rust models for:
  - launcher selection
  - Snake game state, movement, scoring, wrap/border behavior
  - Flags mode flow, answer selection, quiz/death-match behavior
  - 2048 tile sliding, merging, scoring, grid-size selection
  - joystick thresholds, button debounce, encoder decoding
  - high-score key behavior and recording display commands
- `crates/orion-firmware` is a minimal ESP-IDF Rust firmware shell.
- `make build` succeeds for `xtensa-esp32s3-espidf` using the ESP-IDF version
  selected by `esp-idf-sys`.
- Hardware parity is still pending: ST7789V rendering, ADC joystick, encoder,
  NVS storage, and runtime app integration still need to be ported.

## Project Layout

```text
.
├── crates/
│   ├── orion-core/       # host-testable app/game/input logic
│   └── orion-firmware/   # ESP-IDF binary and hardware adapters
├── main/flags.bin        # RGB565 flag image payload copied from ../orion
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

Build the current firmware shell:

```sh
make build
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
`best_3x3`, `best_4x4`, and `best_5x5` per grid size. Keep pin choices
centralized when the hardware adapters are implemented, and keep this README
aligned with those constants.

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

Current test coverage includes 51 unit tests. Run them with:

```sh
cargo test -p orion-core
```

Firmware code in `orion-firmware` should stay as thin as possible: ESP-IDF
drivers, NVS, LCD transfers, FreeRTOS delays, and adapters into `orion-core`.

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

`main/flags.bin` is the generated RGB565 flag payload copied from the C++
project.

For now, Rust flag metadata is generated from the existing C++ asset table:

```sh
python3 tools/generate_flags_assets.py \
  --source ../orion/main/flags_assets.cpp \
  --output crates/orion-core/src/generated/flags_assets.rs
```

Commit generated Rust metadata and `main/flags.bin` so normal builds do not
need network access.

