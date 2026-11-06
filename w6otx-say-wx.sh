#!/bin/bash
#
# Announces the current weather on an AllStar node.

set -o errexit
set -o nounset

readonly NODE=65314
readonly ICAO="KSJC"
#readonly ENGINE="google"
readonly ENGINE="espeak"
readonly -a ENGINE_ARGS=("-v" "uk-female")
readonly FORMAT="ulaw"
readonly STYLE="brief"
# How long a busy node gets to open the file before it is deleted.
readonly PLAYBACK_WAIT_SECONDS=60

audio_file=""

cleanup() {
  if [[ -n "${audio_file}" ]]; then
    /bin/rm -f "${audio_file}"
  fi
}

main() {
  trap cleanup EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM

  if [[ "${ENGINE}" == "google" ]]; then
    # shellcheck source=/dev/null
    source ~/google_cloud_tts_api.shfrag
  fi

  audio_file="$(mktemp "/tmp/tmpXXXXXXXXXX.${FORMAT}")"

  speak-weather "${ENGINE}" "${ENGINE_ARGS[@]}" -f "${STYLE}" -a "${FORMAT}" \
    -o "${audio_file}" "${ICAO}"

  chmod 0644 "${audio_file}"

  cd /tmp
  sudo -u asterisk asterisk -x "rpt localplay ${NODE} ${audio_file%.*}"

  sleep "${PLAYBACK_WAIT_SECONDS}"
}

main "$@"
