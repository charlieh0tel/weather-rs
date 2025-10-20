#!/bin/bash

set -o errexit
set -o nounset
set -o xtrace

NODE=65314
ICAO="KSJC"
ENGINE="google"
#ENGINE="espeak"
FORMAT="ulaw"

. ~/google_cloud_tts_api.shfrag

FILE="$(mktemp "/tmp/tmpXXXXXXXXXX.${FORMAT}")"
trap '/bin/rm -f "${FILE}"' EXIT SIGINT SIGTERM

cargo run --bin "speak-weather" -- "${ENGINE}" -f aviation -a "${FORMAT}" -o "${FILE}" "${ICAO}"

ls -l "${FILE}"
file "${FILE}"

cd /tmp
sudo -u asterisk asterisk -x "rpt localplay ${NODE} ${FILE%.*}"

sleep 10
