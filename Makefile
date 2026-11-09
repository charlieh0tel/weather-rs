# Makefile for weather-rs testing.  The google targets call Google Cloud TTS, which is billed.

# Configuration
ICAO ?= KJFK
OUTPUT_DIR ?= /tmp
PROFILE ?= release

BUILD_CMD := cargo build $(if $(filter release,$(PROFILE)),--release)
WX := cargo run $(if $(filter release,$(PROFILE)),--release) --bin wx --

FORMATS := wav ulaw alaw gsm sln16
STYLES := full brief detailed aviation

all: test
.PHONY: all

build:
	$(BUILD_CMD)
.PHONY: build

test: build test-reports test-all-formats test-speech
.PHONY: test

test-reports: build
	$(WX) metar $(ICAO)
	$(WX) metar --raw $(ICAO)
	$(WX) nws $(ICAO)
	$(WX) alerts $(ICAO)
	$(WX) where $(ICAO)
.PHONY: test-reports

test-all-formats: $(addprefix test-espeak-,$(FORMATS)) $(addprefix test-google-,$(FORMATS))
.PHONY: test-all-formats

test-espeak-%: build
	$(WX) metar $(ICAO) --style aviation --voice espeak --out $(OUTPUT_DIR)/$(ICAO)-espeak.$*
	@file $(OUTPUT_DIR)/$(ICAO)-espeak.$*

test-google-%: build
	$(WX) metar $(ICAO) --style aviation --out $(OUTPUT_DIR)/$(ICAO)-google.$*
	@file $(OUTPUT_DIR)/$(ICAO)-google.$*

test-styles: $(addprefix test-style-,$(STYLES))
.PHONY: test-styles

test-style-%: build
	$(WX) metar $(ICAO) --style $*

test-speech: build
	$(WX) metar $(ICAO) --style aviation --voice espeak --say
	$(WX) metar $(ICAO) --style aviation --say
.PHONY: test-speech

clean:
	rm -f $(OUTPUT_DIR)/$(ICAO)-espeak.* $(OUTPUT_DIR)/$(ICAO)-google.*
.PHONY: clean
