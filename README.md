# weather-rs

`wx` reports the weather for a place: current conditions (METARs) from
[aviationweather.gov](https://aviationweather.gov), and the National Weather
Service's forecast and alerts from
[api.weather.gov](https://www.weather.gov/documentation/services-web-api).  It
prints them, speaks them, writes them to audio files, or plays them on an
AllStar node.

```sh
wx metar KSJC                                  # print the current conditions
wx metar Mountain View, CA --say               # speak them on this machine
wx nws KSJC --out forecast.mp3                 # write the forecast as audio
wx alerts KSJC --node 65314 --only-if-any      # play any alerts on a node
wx metar KSJC --node 65314 --to-links          # play on the node and its links
wx where Mountain View, CA                     # NWS office, grid, zones, stations
```

`wx --help`, `wx <report> --help` and `man wx-<report>` list every option.

## Install

The `weather-rs` Debian package, for amd64 and arm64, is attached to each
[GitHub release](https://github.com/charlieh0tel/weather-rs/releases):

```sh
sudo apt install ./weather-rs_*_amd64.deb
```

It replaces the old `weather` package, whose commands were `metar`,
`speak-weather`, `allstar-weather` and `nws-lookup`.

To build from source, install the packages in `packages.txt` and run
`cargo build --release`.  Writing `ulaw`, `alaw` and `gsm`, and `sln16` with
eSpeak, needs `sox`.

## Reports

| Report | What it gives | Source |
|---|---|---|
| `wx metar PLACE` | Current conditions at the nearest METAR station | aviationweather.gov |
| `wx nws PLACE` | The forecast for the next half-day periods | api.weather.gov (US only) |
| `wx alerts PLACE` | Active alerts | api.weather.gov (US only) |
| `wx where PLACE` | The forecast office, grid, zones and nearest stations | api.weather.gov (US only) |

Report options:

| Option | Report | Meaning |
|---|---|---|
| `-s`, `--style` | `metar` | `full`, `brief`, `detailed` or `aviation` (default `brief` with `--node`, else `full`) |
| `-t`, `--temp-unit` | `metar` | `fahrenheit` (default) or `celsius` |
| `--raw` | `metar` | Print the raw METAR instead |
| `-p`, `--periods` | `nws` | How many half-day periods to read (default 2) |
| `--only-if-any` | `alerts` | Announce nothing when none are in effect |
| `-n`, `--stations` | `where` | How many nearby stations to list (default 5) |

An alert is read as its headline, such as "Wind Advisory issued November 8 at
3:02PM AKST until November 10 at 6:00PM AKST by NWS Fairbanks AK".  With no
alerts in effect, `wx alerts` says so; with `--only-if-any` it announces
nothing and exits successfully, so alerts can be checked often.  If a report
can't be fetched, `wx` fails.

## Places

Every report takes a place, without needing quotes:

| Place | Example | Located by |
|---|---|---|
| A station | `KSJC` | api.weather.gov, or used as is by `metar` |
| Coordinates | `37.3592,-121.9242` | |
| A US street address, starting with a house number | `200 E Santa Clara St, San Jose, CA` | [US Census geocoder](https://geocoding.geo.census.gov/geocoder/) |
| Any other name, in the US | `Mountain View, CA`, `94043` | [OpenStreetMap Nominatim](https://nominatim.openstreetmap.org/) |

For a place that isn't a station, `wx metar` uses the nearest NWS observation
station with a METAR, and says which.  A station outside the US works for
`metar` but not for the NWS reports.

## Outputs

Without an output option, `wx` prints the announcement.  Status messages go to
stderr, so stdout carries only the announcement.

| Option | Meaning |
|---|---|
| `--say` | Speak it on this machine |
| `-o`, `--out FILE` | Write it as audio, in the format the extension names: `mp3`, `wav`, `ogg`, `ulaw`, `alaw`, `gsm` or `sln16` |
| `--node NODE` | Play it on an AllStar node |
| `-l`, `--to-links` | With `--node`, send it to linked nodes too |
| `-a`, `--audio-format` | Override the format (default `sln16` with `--node`) |
| `-w`, `--wait` | With `--node`, seconds the node gets to open the file before it is deleted (default 60) |

### Voices

| Option | Meaning |
|---|---|
| `-v`, `--voice` | A Google voice name, such as `en-US-Chirp3-HD-Leda` (the default), or `espeak` |
| `-g`, `--gain` | Google volume gain in dB, -96 to 16 (default 6) |
| `--api-key-file` | The file holding the Google API key |

A Google voice needs an API key; printing never does.  `wx` reads the key from
`--api-key-file`, else from `GOOGLE_CLOUD_API_KEY`, else from
`~/.config/weather-rs/google-api-key` (under `$XDG_CONFIG_HOME` if set).  Keep
the file private:

```sh
mkdir -p ~/.config/weather-rs
(umask 077; cat > ~/.config/weather-rs/google-api-key)
```

## AllStar

`--node` plays the announcement with `rpt localplay`, on the node's own
transmitter, or with `--to-links`, `rpt playback`, which also sends it to every
linked node.  It runs `sudo -u asterisk asterisk`, which must not ask for a
password.

`sln16` is Asterisk's raw 16 kHz signed linear format.  Writing that and
letting Asterisk convert it for each channel gives better quality than writing
8 kHz `ulaw`.  Asterisk needs `codec_resample` to convert it for 8 kHz
channels; ASL3 doesn't load it by default, so add this to
`/etc/asterisk/modules.conf`:

```
load => codec_resample.so
```

`wx` runs well from cron.  For example, the METAR every two hours from 9 am to
5 pm, the forecast at 6:35 am, and any alerts every 15 minutes:

```
5 9-17/2 * * * wx metar KSJC --node 65314 >>/tmp/wx.log 2>&1
35 6 * * *     wx nws KSJC --node 65314 >>/tmp/wx.log 2>&1
*/15 * * * *   wx alerts KSJC --node 65314 --only-if-any >>/tmp/wx.log 2>&1
```

Without the redirect, cron mails any errors, which usually means they are lost.
A cron job runs as the crontab's owner, so it finds that user's key file.

## License

BSD-3-Clause; see [LICENSE](LICENSE).
