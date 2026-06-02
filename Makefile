SHELL := /bin/bash

ESP_EXPORT ?= /Users/ivan/export-esp.sh
ESP_IDF_EXPORT ?= /Users/ivan/.espressif/v6.0.1/esp-idf/export.sh
WIFI_SSID ?= Murlo
WIFI_PASSWORD ?= kotopes4WiFi
DETECTED_PORT := $(firstword $(wildcard /dev/cu.usbmodem*) $(wildcard /dev/cu.usbserial*) $(wildcard /dev/cu.SLAB_USBtoUART*) $(wildcard /dev/cu.wchusbserial*))
PORT ?= $(DETECTED_PORT)
FLASH_BAUD ?= 921600
FLASH_BEFORE ?= usb-reset
MONITOR_BAUD ?= 115200
TARGET := xtensa-esp32s3-espidf
PROFILE ?= debug
PROFILE_FLAG_debug =
PROFILE_FLAG_release = --release
PROFILE_FLAG = $(PROFILE_FLAG_$(PROFILE))
ESP_IDF_SYS_BUILD_DIR = $(firstword $(wildcard target/$(TARGET)/$(PROFILE)/build/esp-idf-sys-*/out/build) $(wildcard target/$(TARGET)/debug/build/esp-idf-sys-*/out/build))
BOOTLOADER_BIN = $(ESP_IDF_SYS_BUILD_DIR)/bootloader/bootloader.bin

CARGO_ESP = . "$(ESP_EXPORT)" >/dev/null && ORION_WIFI_SSID="$(WIFI_SSID)" ORION_WIFI_PASSWORD="$(WIFI_PASSWORD)" ESP_IDF_SYS_ROOT_CRATE=orion-firmware cargo +esp
CARGO_ESP_IDF6 = . "$(ESP_EXPORT)" >/dev/null && . "$(ESP_IDF_EXPORT)" >/dev/null && ORION_WIFI_SSID="$(WIFI_SSID)" ORION_WIFI_PASSWORD="$(WIFI_PASSWORD)" ESP_IDF_SYS_ROOT_CRATE=orion-firmware cargo +esp
ESPFLASH = . "$(ESP_EXPORT)" >/dev/null && espflash
FIRMWARE_ELF = target/$(TARGET)/$(PROFILE)/orion-firmware

.PHONY: help env-check test coverage coverage-html coverage-lcov coverage-xml build build-release build-idf6 flash flash-release monitor flash-monitor flash-monitor-release erase-nvs ports size size-release clean

help:
	@echo "Orion Rust targets:"
	@echo "  make test             Run host tests for orion-core"
	@echo "  make build            Build ESP32-S3 firmware with esp-idf-sys default ESP-IDF"
	@echo "  make build-release    Build size-optimized ESP32-S3 firmware"
	@echo "  make build-idf6       Try building against local ESP-IDF 6.0.1"
	@echo "  make flash PORT=...   Flash ESP32-S3 firmware"
	@echo "  make flash-release    Flash size-optimized firmware"
	@echo "  make monitor PORT=... Monitor serial output"
	@echo "  make flash-monitor    Flash and monitor"
	@echo "  make flash-monitor-release Flash size-optimized firmware and monitor"
	@echo "  make erase-nvs        Erase persisted scores/settings"
	@echo "  make ports            List likely serial ports"
	@echo "  make coverage         Show coverage summary for orion-core"
	@echo "  make coverage-html    Generate HTML coverage report in target/llvm-cov/html/"
	@echo "  make coverage-lcov    Generate lcov.info coverage report"
	@echo "  make coverage-xml     Generate Cobertura XML coverage report"
	@echo "  make size             Show firmware size"
	@echo "  make clean            Remove Cargo build output"

env-check:
	@test -f "$(ESP_EXPORT)" || (echo "esp-rs export script not found: $(ESP_EXPORT)" && exit 1)
	@test -f "$(ESP_IDF_EXPORT)" || (echo "ESP-IDF export script not found: $(ESP_IDF_EXPORT)" && exit 1)
	@command -v cargo >/dev/null
	@command -v ldproxy >/dev/null
	@command -v cargo-espflash >/dev/null

test:
	cargo test -p orion-core

coverage:
	RUSTUP_TOOLCHAIN=stable cargo llvm-cov -p orion-core --summary-only

coverage-html:
	RUSTUP_TOOLCHAIN=stable cargo llvm-cov -p orion-core --html

coverage-lcov:
	RUSTUP_TOOLCHAIN=stable cargo llvm-cov -p orion-core --lcov --output-path lcov.info

coverage-xml:
	RUSTUP_TOOLCHAIN=stable cargo llvm-cov -p orion-core --cobertura --output-path coverage.xml

build: env-check
	$(CARGO_ESP) build -p orion-firmware --target "$(TARGET)" $(PROFILE_FLAG)

build-release:
	$(MAKE) build PROFILE=release

build-idf6: env-check
	$(CARGO_ESP_IDF6) build -p orion-firmware --target "$(TARGET)"

flash: env-check build
	$(ESPFLASH) flash -p "$(PORT)" -B "$(FLASH_BAUD)" --before "$(FLASH_BEFORE)" --bootloader "$(BOOTLOADER_BIN)" --partition-table partitions.csv --target-app-partition factory "$(FIRMWARE_ELF)"

flash-release:
	$(MAKE) flash PROFILE=release

monitor: env-check
	espflash monitor -p "$(PORT)" -B "$(MONITOR_BAUD)"

flash-monitor: env-check build
	$(ESPFLASH) flash -p "$(PORT)" -B "$(FLASH_BAUD)" --before "$(FLASH_BEFORE)" --monitor --monitor-baud "$(MONITOR_BAUD)" --bootloader "$(BOOTLOADER_BIN)" --partition-table partitions.csv --target-app-partition factory "$(FIRMWARE_ELF)"

flash-monitor-release:
	$(MAKE) flash-monitor PROFILE=release

erase-nvs: env-check
	$(ESPFLASH) erase-parts -p "$(PORT)" -B "$(FLASH_BAUD)" --before "$(FLASH_BEFORE)" --partition-table partitions.csv nvs

ports:
	@echo "Likely ESP32 serial ports:"
	@ls /dev/cu.usbmodem* /dev/cu.usbserial* /dev/cu.SLAB_USBtoUART* /dev/cu.wchusbserial* 2>/dev/null || true
	@echo
	@echo "Default PORT: $(if $(PORT),$(PORT),none detected)"

size: build
	xtensa-esp32s3-elf-size "$(FIRMWARE_ELF)" || true

size-release:
	$(MAKE) size PROFILE=release

clean:
	cargo clean
