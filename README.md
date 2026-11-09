# weather-rs

Fetches current METARs from [aviationweather.gov](https://aviationweather.gov)
and reads them aloud, or writes the announcement to an audio file for an
AllStar node or other Asterisk system.  In the US, it can instead read the
National Weather Service's forecast or alerts from
[api.weather.gov](https://www.weather.gov/documentation/services-web-api).

The `weather` package installs these commands:

- `metar` prints the decoded weather for an airport.
- `speak-weather` turns it into speech with eSpeak NG or Google Cloud
  Text-to-Speech, or prints the text for another TTS engine.
- `nws-lookup` finds the National Weather Service identifiers for a place.
- `allstar-weather` announces a report on an AllStar node.

They are named so as not to conflict with Debian's `weather-util`, which owns
`/usr/bin/weather`.

## Install

Debian packages for amd64 and arm64 are attached to each
[GitHub release](https://github.com/charlieh0tel/weather-rs/releases):

```sh
sudo apt install ./weather_*_amd64.deb
```

To build from source, install the packages in `packages.txt` and run
`cargo build --release`.  Writing `ulaw`, `alaw` and `gsm`, and `sln16` from
eSpeak, needs `sox`.

## Usage

```sh
metar KSJC
speak-weather espeak KSJC
speak-weather google --voice-name en-US-Chirp3-HD-Leda KSJC
speak-weather text -f aviation KSJC
speak-weather text -r forecast KSJC
speak-weather text -r alerts "200 E Santa Clara St, San Jose, CA"
nws-lookup 200 E Santa Clara St, San Jose, CA
```

Without `-o`, `speak-weather` speaks through the default audio device.  With
`-o FILE`, it writes the audio there; the format comes from `-a` or else the
file's extension.

Options shared by all engines:

| Option | Meaning |
|---|---|
| `-r`, `--report` | `metar` (default), `forecast` or `alerts` |
| `-f`, `--format` | For `metar`: `speech` (default), `brief`, `detailed` or `aviation` |
| `-t`, `--temp-unit` | For `metar`: `fahrenheit` (default) or `celsius` |
| `--periods` | For `forecast`: how many half-day periods to read (default 2) |
| `-o`, `--output` | Write to a file instead of speaking or printing |

For `metar`, the location is an ICAO airport.  For `forecast` and `alerts`,
it is any place `nws-lookup` accepts (see below).

Audio formats (`-a`): `mp3`, `wav`, `ogg`, `ulaw`, `alaw`, `gsm` and `sln16`.

`--help` on each subcommand lists the rest, and each command has a man page,
such as `man speak-weather-google`.

### National Weather Service

The forecast and alerts cover only the US and its territories.  An alert is
read as its headline, such as "Wind Advisory issued November 8 at 3:02PM AKST
until November 10 at 6:00PM AKST by NWS Fairbanks AK"; with no alerts in
effect, it says so.

`speak-weather -r forecast`, `-r alerts` and `nws-lookup` take a place as any
of:

- an ICAO station, such as `KSJC`, located by api.weather.gov;
- `latitude,longitude`, such as `37.3592,-121.9242`;
- a US street address, located by the
  [US Census geocoder](https://geocoding.geo.census.gov/geocoder/).

`nws-lookup` prints the forecast office, grid square, zones and nearest
observation stations that the National Weather Service uses for the place:

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

`-n` changes how many stations it lists.

### eSpeak NG

`-v` chooses a voice preset (`default`, `us-female`, `us-male`, `uk-female`,
`uk-male`), and `-s`, `-p` and `-g` override its speed, pitch and word gap.

### Google Cloud TTS

`-v` chooses one of the same presets, mapped to Neural2 voices.
`--voice-name` takes any Google voice by its full name, such as
`en-US-Chirp3-HD-Leda`.  `--gain` makes it louder or softer, in dB from −96
to 16.

The API key is read from `GOOGLE_CLOUD_API_KEY`, or else from
`~/.config/speak-weather/google-api-key` (under `$XDG_CONFIG_HOME` if set).
Keep the file private:

```sh
mkdir -p ~/.config/speak-weather
(umask 077; cat > ~/.config/speak-weather/google-api-key)
```

## Asterisk and AllStar

`sln16` is Asterisk's raw 16 kHz signed linear format.  Writing that and
letting Asterisk convert it for each channel gives better quality than
writing 8 kHz `ulaw`.  Asterisk needs `codec_resample` to convert it for 8
kHz channels; ASL3 doesn't load it by default, so add this to
`/etc/asterisk/modules.conf`:

```
load => codec_resample.so
```

`allstar-weather NODE LOCATION` speaks a report and plays it on a node with
`rpt localplay`.  `-r` chooses the report as for `speak-weather`: the METAR
for an airport by default, or the forecast or alerts for a place.  It uses
Google's `en-US-Chirp3-HD-Leda` voice, 6 dB louder, by default; `-e`, `-v`,
`-g` and `-a` change the engine, voice, gain and audio format, and `--help`
lists the rest.  It runs well from cron,
for example the METAR every two hours from 9 am to 5 pm, and the forecast at
6:35 am:

```
5 9-17/2 * * * allstar-weather 65314 KSJC
35 6 * * * allstar-weather -r forecast 65314 KSJC
```

It needs to run `sudo -u asterisk asterisk` without a password, and the user
running it needs the Google API key.

## License

BSD-3-Clause; see [LICENSE](LICENSE).
