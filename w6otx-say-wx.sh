#!/bin/bash

set -o errexit
set -o nounset
set -o xtrace

NODE=65314
ICAO="KSJC"
#ENGINE="google"
ENGINE="espeak"
ENGINE_ARGS=("-v" "uk-female")
FORMAT="ulaw"
STYLE="brief"

. ~/google_cloud_tts_api.shfrag

FILE="$(mktemp "/tmp/tmpXXXXXXXXXX.${FORMAT}")"
trap '/bin/rm -f "${FILE}"' EXIT SIGINT SIGTERM

speak-weather "${ENGINE}" "${ENGINE_ARGS[@]}" -f "${STYLE}" -a "${FORMAT}" -o "${FILE}" "${ICAO}"

chmod 0644 "${FILE}"

cd /tmp
sudo -u asterisk asterisk -x "rpt localplay ${NODE} ${FILE%.*}"

# don't delete the file before asterisk opens it.
sleep 5

exit 0
