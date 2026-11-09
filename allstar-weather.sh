#!/bin/bash
#
# Announces the weather on an AllStar node.
#
# Usage: allstar-weather [options] NODE LOCATION
#
# Suitable for cron, for example every two hours from 9 am to 5 pm:
#   5 9-17/2 * * * allstar-weather 65314 KSJC
#   35 6 * * * allstar-weather -r forecast 65314 KSJC

set -o errexit
set -o nounset

# How long a busy node gets to open the file before it is deleted.
readonly PLAYBACK_WAIT_SECONDS=60

audio_file=""

usage() {
  cat <<EOF
Usage: ${0##*/} [options] NODE LOCATION

LOCATION is an ICAO airport for metar, or for forecast and alerts any place
speak-weather accepts.

Options:
  -r REPORT  metar (default), forecast or alerts
  -e ENGINE  google (default) or espeak
  -v VOICE   Google voice name (default: en-US-Chirp3-HD-Leda)
  -g GAIN    Google volume gain in dB (default: 6)
  -f FORMAT  Audio format (default: sln16, which needs codec_resample)
  -s STYLE   metar announcement style (default: brief)
  -h         Show this help
EOF
}

cleanup() {
  if [[ -n "${audio_file}" ]]; then
    rm -f "${audio_file}"
  fi
}

main() {
  local report="metar"
  local engine="google"
  local voice="en-US-Chirp3-HD-Leda"
  local gain="6"
  local format="sln16"
  local style="brief"

  local opt
  while getopts "r:e:v:g:f:s:h" opt; do
    case "${opt}" in
      r) report="${OPTARG}" ;;
      e) engine="${OPTARG}" ;;
      v) voice="${OPTARG}" ;;
      g) gain="${OPTARG}" ;;
      f) format="${OPTARG}" ;;
      s) style="${OPTARG}" ;;
      h)
        usage
        exit 0
        ;;
      *)
        usage >&2
        exit 2
        ;;
    esac
  done
  shift $((OPTIND - 1))
  if (($# != 2)); then
    usage >&2
    exit 2
  fi
  local -r node="$1"
  local -r location="$2"

  local -a engine_args=()
  case "${engine}" in
    google) engine_args=("--voice-name" "${voice}" "--gain" "${gain}") ;;
    espeak) ;;
    *)
      echo "Unknown engine: ${engine}" >&2
      exit 2
      ;;
  esac

  trap cleanup EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM

  audio_file="$(mktemp "/tmp/tmpXXXXXXXXXX.${format}")"

  speak-weather "${engine}" "${engine_args[@]}" -r "${report}" -f "${style}" \
    -a "${format}" -o "${audio_file}" "${location}"

  chmod 0644 "${audio_file}"

  cd /tmp
  sudo -u asterisk asterisk -x "rpt localplay ${node} ${audio_file%.*}"

  sleep "${PLAYBACK_WAIT_SECONDS}"
}

main "$@"
