# Makefile for weather-rs testing

# Configuration
ICAO ?= KJFK
OUTPUT_DIR ?= /tmp

# Binaries
ESPEAK_BIN = ./target/release/speak-weather-espeak
GOOGLE_BIN = ./target/release/speak-weather-google-tts

# Output files
ESPEAK_GSM = $(OUTPUT_DIR)/$(ICAO)-espeak.gsm
GOOGLE_GSM = $(OUTPUT_DIR)/$(ICAO)-google.gsm

all: test

build:
	cargo build --release

test: build test-espeak test-google test-gsm-espeak test-gsm-google

test-espeak: build
	$(ESPEAK_BIN) $(ICAO)

test-google: build
	$(GOOGLE_BIN) $(ICAO)

test-gsm-espeak: build
	$(ESPEAK_BIN) $(ICAO) --output $(ESPEAK_GSM) --audio-format gsm
	play $(ESPEAK_GSM)

test-gsm-google: build
	$(GOOGLE_BIN) $(ICAO) --output $(GOOGLE_GSM) --audio-format gsm
	play $(GOOGLE_GSM)

clean:
	rm -f $(OUTPUT_DIR)/*-espeak.gsm $(OUTPUT_DIR)/*-google.gsm


.PHONY: all build test test-espeak test-google test-gsm-espeak test-gsm-google clean
