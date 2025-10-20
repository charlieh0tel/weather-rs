# Makefile for weather-rs testing

# Configuration
ICAO ?= KJFK
OUTPUT_DIR ?= /tmp
PROFILE ?= release

BUILD_CMD := cargo build $(if $(filter release,$(PROFILE)),--release)
RUN_CMD := cargo run $(if $(filter release,$(PROFILE)),--release) --bin

FORMATS := wav ulaw alaw gsm
ANNOUNCEMENT_FORMATS := speech brief detailed aviation

all: test

build:
	$(BUILD_CMD)

test: build test-weather test-all-formats test-speech test-text

test-weather: build
	$(RUN_CMD) weather -- $(ICAO)

test-all-formats: $(addprefix test-espeak-,$(FORMATS)) $(addprefix test-google-,$(FORMATS))

test-espeak-%: build
	$(RUN_CMD) speak-weather -- espeak $(ICAO) --output $(OUTPUT_DIR)/$(ICAO)-espeak.$* --audio-format $* --format aviation
	@file $(OUTPUT_DIR)/$(ICAO)-espeak.$*

test-google-%: build
	$(RUN_CMD) speak-weather -- google $(ICAO) --output $(OUTPUT_DIR)/$(ICAO)-google.$* --audio-format $* --format aviation
	@file $(OUTPUT_DIR)/$(ICAO)-google.$*

test-announcement-formats: $(addprefix test-announcement-,$(ANNOUNCEMENT_FORMATS))

test-announcement-%: build
	$(RUN_CMD) speak-weather -- text $(ICAO) --format $* --output $(OUTPUT_DIR)/$(ICAO)-$*.txt
	@echo "Generated announcement ($*):"
	@cat $(OUTPUT_DIR)/$(ICAO)-$*.txt

test-speech: build
	$(RUN_CMD) speak-weather -- espeak $(ICAO) --format aviation
	$(RUN_CMD) speak-weather -- google $(ICAO) --format aviation

test-text: build
	$(RUN_CMD) speak-weather -- text $(ICAO) --format aviation

clean:
	rm -f $(OUTPUT_DIR)/$(ICAO)-espeak.* $(OUTPUT_DIR)/$(ICAO)-google.* $(OUTPUT_DIR)/$(ICAO)-*.txt

help:
	@echo "Available targets:"
	@echo "  test                     - Run all tests"
	@echo "  test-weather             - Test weather binary (data only)"
	@echo "  test-all-formats         - Test all audio formats"
	@echo "  test-announcement-formats - Test all announcement formats"
	@echo "  test-speech              - Test live speech playback"
	@echo "  test-text                - Test text engine"
	@echo "  clean                    - Remove generated files"
	@echo ""
	@echo "Configuration:"
	@echo "  PROFILE=$(PROFILE)       - Build profile (release or debug)"
	@echo "  ICAO=$(ICAO)             - Airport code to test"
	@echo "  OUTPUT_DIR=$(OUTPUT_DIR) - Directory for audio files"

.PHONY: all build test test-weather test-all-formats test-announcement-formats test-speech test-text clean help
