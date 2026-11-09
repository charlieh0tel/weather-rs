# weather-rs

Fetches current METARs from [aviationweather.gov](https://aviationweather.gov)
and reads them aloud, or writes the announcement to an audio file for an
AllStar node or other Asterisk system.

The `weather` package installs two commands:

- `metar` prints the decoded weather for an airport.
- `speak-weather` turns it into speech with eSpeak NG or Google Cloud
  Text-to-Speech, or prints the text for another TTS engine.

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
```

Without `-o`, `speak-weather` speaks through the default audio device.  With
`-o FILE`, it writes the audio there; the format comes from `-a` or else the
file's extension.

Options shared by all engines:

| Option | Meaning |
|---|---|
| `-f`, `--format` | `speech` (default), `brief`, `detailed` or `aviation` |
| `-t`, `--temp-unit` | `fahrenheit` (default) or `celsius` |
| `-o`, `--output` | Write to a file instead of speaking or printing |

Audio formats (`-a`): `mp3`, `wav`, `ogg`, `ulaw`, `alaw`, `gsm` and `sln16`.

`--help` on each subcommand lists the rest.

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

`allstar-weather NODE ICAO` speaks the weather at an airport and plays it on
a node with `rpt localplay`.  It uses Google's `en-US-Chirp3-HD-Leda` voice
by default; `-h` lists options to change the engine, voice, gain, format and
style.  It runs well from cron, for example every two hours from 9 am to 5
pm:

```
5 9-17/2 * * * allstar-weather 65314 KSJC
```

It needs to run `sudo -u asterisk asterisk` without a password, and the user
running it needs the Google API key.

## License

BSD-3-Clause; see [LICENSE](LICENSE).
