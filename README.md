# weather-rs

Weather reports, read aloud.  It fetches current conditions (METARs) from
[aviationweather.gov](https://aviationweather.gov) and, in the US, the
National Weather Service's forecast and alerts from
[api.weather.gov](https://www.weather.gov/documentation/services-web-api).  It
speaks them, writes them to audio files, or plays them on an AllStar node.

The `weather` package installs four commands:

| Command | What it does |
|---|---|
| `metar` | Prints the decoded METAR for an airport |
| `speak-weather` | Speaks a report, writes it to an audio file, or prints its text |
| `allstar-weather` | Speaks a report and plays it on an AllStar node |
| `nws-lookup` | Prints the National Weather Service identifiers for a place |

They are named so as not to conflict with Debian's `weather-util`, which owns
`/usr/bin/weather`.  Each has a man page, and `--help` lists all its options.

## Install

Debian packages for amd64 and arm64 are attached to each
[GitHub release](https://github.com/charlieh0tel/weather-rs/releases):

```sh
sudo apt install ./weather_*_amd64.deb
```

To build from source, install the packages in `packages.txt` and run
`cargo build --release`.  Writing `ulaw`, `alaw` and `gsm`, and `sln16` from
eSpeak, needs `sox`.

## Reports and locations

`speak-weather` and `allstar-weather` announce one report, chosen with `-r`:

| Report | Source | Location |
|---|---|---|
| `metar` (default) | aviationweather.gov | An ICAO airport, such as `KSJC` |
| `forecast` | api.weather.gov (US only) | Any place, as below |
| `alerts` | api.weather.gov (US only) | Any place, as below |

A place, for `forecast`, `alerts` and `nws-lookup`, is any of:

- an ICAO station, such as `KSJC`, located by api.weather.gov;
- `latitude,longitude`, such as `37.3592,-121.9242`;
- a US street address, such as `"200 E Santa Clara St, San Jose, CA"`, located
  by the [US Census geocoder](https://geocoding.geo.census.gov/geocoder/).

The forecast reads the next half-day periods, two by default.  An alert is read
as its headline, such as "Wind Advisory issued November 8 at 3:02PM AKST until
November 10 at 6:00PM AKST by NWS Fairbanks AK"; with no alerts in effect, it
says so.  If a report can't be fetched, the command fails.

Options for the report:

| Option | Meaning |
|---|---|
| `-r`, `--report` | `metar` (default), `forecast` or `alerts` |
| `-f`, `--format` | For `metar`: `speech`, `brief`, `detailed` or `aviation` |
| `-t`, `--temp-unit` | For `metar`: `fahrenheit` (default) or `celsius` |
| `--periods` | For `forecast`: how many half-day periods to read (default 2) |

## metar

```sh
metar KSJC
```

## speak-weather

```sh
speak-weather espeak KSJC
speak-weather google --voice-name en-US-Chirp3-HD-Leda KSJC
speak-weather google -r forecast -o forecast.mp3 KSJC
speak-weather text -f aviation KSJC
speak-weather text -r alerts "200 E Santa Clara St, San Jose, CA"
```

The first word chooses the engine: `espeak` (eSpeak NG), `google` (Google
Cloud Text-to-Speech) or `text`, which prints the announcement for another
engine.  Besides the report options, all three take:

| Option | Meaning |
|---|---|
| `-o`, `--output` | Write to a file instead of speaking or printing |
| `-f`, `--format` | Defaults to `speech` |

Without `-o`, `espeak` and `google` speak through the default audio device.
With `-o FILE`, they write audio there, in the format from `-a` or else the
file's extension: `mp3`, `wav`, `ogg`, `ulaw`, `alaw`, `gsm` or `sln16`.

`espeak`:

| Option | Meaning |
|---|---|
| `-v`, `--voice` | `default`, `us-female`, `us-male`, `uk-female` or `uk-male` |
| `-s`, `--speed` | Words per minute |
| `-p`, `--pitch` | 0 to 99 |
| `-g`, `--gap` | Gap between words, in 10 ms units |

`google`:

| Option | Meaning |
|---|---|
| `-v`, `--voice` | The same presets, mapped to Neural2 voices |
| `--voice-name` | Any Google voice by its full name, such as `en-US-Chirp3-HD-Leda` |
| `--gain` | Louder or softer, in dB from -96 to 16 (default 0) |
| `--api-key-file` | The file holding the API key (see below) |

## allstar-weather

```sh
allstar-weather 65314 KSJC
allstar-weather -r forecast 65314 KSJC
allstar-weather -e espeak -a ulaw 65314 KSJC
```

`allstar-weather NODE LOCATION` speaks a report and plays it on the node with
`rpt localplay`, by running `sudo -u asterisk asterisk`, which must not ask for
a password.  Besides the report options:

| Option | Meaning |
|---|---|
| `-f`, `--format` | Defaults to `brief` |
| `-e`, `--engine` | `google` (default) or `espeak` |
| `-v`, `--voice-name` | Google voice (default `en-US-Chirp3-HD-Leda`) |
| `-g`, `--gain` | Google gain in dB (default 6) |
| `--api-key-file` | The file holding the Google API key (see below) |
| `-a`, `--audio-format` | Format to play (default `sln16`; see below) |
| `-w`, `--wait` | Seconds the node gets to open the file before it is deleted (default 60) |

It runs well from cron.  For example, the METAR every two hours from 9 am to 5
pm, and the forecast at 6:35 am:

```
5 9-17/2 * * * allstar-weather 65314 KSJC >>/tmp/allstar-weather.log 2>&1
35 6 * * * allstar-weather -r forecast 65314 KSJC >>/tmp/allstar-weather.log 2>&1
```

Without the redirect, cron mails any errors, which usually means they are lost.

## nws-lookup

```sh
nws-lookup KSJC
nws-lookup 200 E Santa Clara St, San Jose, CA
```

It prints the forecast office, grid square, zones and nearest observation
stations that the National Weather Service uses for a place.  An address needs
no quotes here.  `-n` changes how many stations it lists.

```
$ nws-lookup KSJC
Location:        37.3592,-121.9242 (San Jose, San Jose International Airport)
Nearest city:    San Jose, CA
Forecast office: MTR
Grid:            MTR 98,83
Forecast zone:   CAZ513
County zone:     CAC085
Time zone:       America/Los_Angeles
Stations:        KSJC   San Jose, San Jose International Airport (1.1 km)
                 KRHV   San Jose / Reid / Hillv (10.1 km)
                 KNUQ   Mountain View, Moffett Field (12.2 km)
                 LSGC1  LOS GATOS (16.3 km)
                 AW020  AE6EO MOUNTAIN VIEW (16.6 km)
```

## The Google API key

`speak-weather google` and `allstar-weather` read the key from the file given
with `--api-key-file`, else from `GOOGLE_CLOUD_API_KEY`, else from
`~/.config/speak-weather/google-api-key` (under `$XDG_CONFIG_HOME` if set).  A
cron job runs as the crontab's owner, so it finds that user's key file.  Keep
the file private:

```sh
mkdir -p ~/.config/speak-weather
(umask 077; cat > ~/.config/speak-weather/google-api-key)
```

## Asterisk audio

`sln16` is Asterisk's raw 16 kHz signed linear format.  Writing that and
letting Asterisk convert it for each channel gives better quality than writing
8 kHz `ulaw`.  Asterisk needs `codec_resample` to convert it for 8 kHz
channels; ASL3 doesn't load it by default, so add this to
`/etc/asterisk/modules.conf`:

```
load => codec_resample.so
```

## License

BSD-3-Clause; see [LICENSE](LICENSE).
