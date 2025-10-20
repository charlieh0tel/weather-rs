# Makefile for weather-rs testing

# Configuration
ICAO ?= KJFK
OUTPUT_DIR ?= /tmp
PROFILE ?= release

# Build command
BUILD_CMD = cargo build $(if $(filter release,$(PROFILE)),--release)
RUN_CMD = cargo run $(if $(filter release,$(PROFILE)),--release) --bin

# Audio formats
FORMATS = wav mp3 ogg ulaw alaw gsm
WEATHER_FORMATS = speech brief detailed aviation

all: test

build:
	$(BUILD_CMD)

test: build test-weather test-all-formats test-speech

# Weather binary tests
test-weather: build $(addprefix test-weather-,$(WEATHER_FORMATS))

test-weather-%: build
	$(RUN_CMD) weather -- --format $* $(ICAO)

# Format testing
test-all-formats: $(addprefix test-espeak-,$(FORMATS)) $(addprefix test-google-,$(FORMATS))

# Pattern rules for format testing
test-espeak-%: build
	$(RUN_CMD) speak-weather-espeak -- $(ICAO) --output $(OUTPUT_DIR)/$(ICAO)-espeak.$* --audio-format $*
	@file $(OUTPUT_DIR)/$(ICAO)-espeak.$*

test-google-%: build
	$(RUN_CMD) speak-weather-google-tts -- $(ICAO) --output $(OUTPUT_DIR)/$(ICAO)-google.$* --audio-format $*
	@file $(OUTPUT_DIR)/$(ICAO)-google.$*

# Speech playback tests
test-speech: build
	$(RUN_CMD) speak-weather-espeak -- $(ICAO)
	$(RUN_CMD) speak-weather-google-tts -- $(ICAO)

clean:
	rm -f $(OUTPUT_DIR)/$(ICAO)-espeak.* $(OUTPUT_DIR)/$(ICAO)-google.*

help:
	@echo "Available targets:"
	@echo "  test              - Run all tests"
	@echo "  test-weather      - Test weather binary ($(WEATHER_FORMATS))"
	@echo "  test-all-formats  - Test all audio formats ($(FORMATS))"
	@echo "  test-speech       - Test live speech playback"
	@echo "  clean             - Remove generated files"
	@echo ""
	@echo "Configuration:"
	@echo "  PROFILE=$(PROFILE)  - Build profile (release or debug)"
	@echo "  ICAO=$(ICAO)        - Airport code to test"
	@echo "  OUTPUT_DIR=$(OUTPUT_DIR) - Directory for audio files"

.PHONY: all build test test-weather test-all-formats test-speech clean help \
        $(addprefix test-weather-,$(WEATHER_FORMATS)) \
        $(addprefix test-espeak-,$(FORMATS)) \
        $(addprefix test-google-,$(FORMATS))
