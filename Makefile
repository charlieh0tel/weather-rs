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
.PHONY: all

build:
	$(BUILD_CMD)
.PHONY: build

test: build test-weather test-all-formats test-speech test-text
.PHONY: test

test-weather: build
	$(RUN_CMD) weather -- $(ICAO)
.PHONY: test-weather

test-all-formats: $(addprefix test-espeak-,$(FORMATS)) $(addprefix test-google-,$(FORMATS))
.PHONY: test-all-formats

test-espeak-%: build
	$(RUN_CMD) speak-weather -- espeak $(ICAO) --output $(OUTPUT_DIR)/$(ICAO)-espeak.$* --audio-format $* --format aviation
	@file $(OUTPUT_DIR)/$(ICAO)-espeak.$*

test-google-%: build
	$(RUN_CMD) speak-weather -- google $(ICAO) --output $(OUTPUT_DIR)/$(ICAO)-google.$* --audio-format $* --format aviation
	@file $(OUTPUT_DIR)/$(ICAO)-google.$*

test-announcement-formats: $(addprefix test-announcement-,$(ANNOUNCEMENT_FORMATS))
.PHONY: test-announcement-formats

test-announcement-%: build
	$(RUN_CMD) speak-weather -- text $(ICAO) --format $* --output $(OUTPUT_DIR)/$(ICAO)-$*.txt
	@echo "Generated announcement ($*):"
	@cat $(OUTPUT_DIR)/$(ICAO)-$*.txt

test-speech: build
	$(RUN_CMD) speak-weather -- espeak $(ICAO) --format aviation
	$(RUN_CMD) speak-weather -- google $(ICAO) --format aviation
.PHONY: test-speech

test-text: build
	$(RUN_CMD) speak-weather -- text $(ICAO) --format aviation
.PHONY: test-text

clean:
	rm -f $(OUTPUT_DIR)/$(ICAO)-espeak.* $(OUTPUT_DIR)/$(ICAO)-google.* $(OUTPUT_DIR)/$(ICAO)-*.txt
.PHONY: clean
