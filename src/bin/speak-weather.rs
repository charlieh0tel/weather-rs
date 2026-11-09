use clap::{Args, Parser, Subcommand};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use weather::{
    fetch_weather_data,
    nws::{Location, NwsClient},
    tts::{
        AnnouncementFormat, AudioFormat, TemperatureUnit, Voice, alerts_announcement,
        espeak::{EspeakTts, EspeakVoice},
        execute_tts_output, forecast_announcement, generate_weather_announcement,
        google_tts::GoogleTts,
    },
};

#[derive(Parser, Debug)]
#[command(author, version, about = "Speak aviation weather with multiple TTS engines", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Use eSpeak TTS engine
    Espeak(EspeakArgs),
    /// Use Google Cloud TTS engine
    Google(GoogleArgs),
    /// Output text for external TTS engines
    Text(TextArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Report {
    /// Current conditions at an airport, from aviationweather.gov
    Metar,
    /// The National Weather Service forecast (US only)
    Forecast,
    /// Active National Weather Service alerts (US only)
    Alerts,
}

#[derive(Args, Debug)]
struct CommonArgs {
    /// ICAO airport for metar (e.g., KSJC); station, lat,lon or US address for forecast and
    /// alerts
    location: String,

    /// What to announce
    #[arg(short, long, value_enum, default_value = "metar")]
    report: Report,

    /// Output format for the metar announcement
    #[arg(short, long, value_enum, default_value = "speech")]
    format: AnnouncementFormat,

    /// Temperature unit for metar (the detailed format always gives both)
    #[arg(short, long, value_enum, default_value = "fahrenheit")]
    temp_unit: TemperatureUnit,

    /// Number of half-day forecast periods [default: 2]
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..))]
    periods: Option<u8>,

    /// Save output to file instead of speaking/printing
    #[arg(short, long)]
    output: Option<String>,
}

fn announcement(common: &CommonArgs) -> Result<String, Box<dyn std::error::Error>> {
    if common.periods.is_some() && common.report != Report::Forecast {
        return Err("--periods is only for --report forecast".into());
    }
    println!("Fetching weather for {}...\n", common.location);

    if common.report == Report::Metar {
        let metar = fetch_weather_data(&common.location)?;
        return Ok(generate_weather_announcement(
            &metar,
            &common.format,
            common.temp_unit,
        ));
    }

    let location: Location = common.location.parse()?;
    let nws = NwsClient::new()?;
    let coordinates = nws.locate(&location)?.coordinates;
    if common.report == Report::Alerts {
        return Ok(alerts_announcement(&nws.alerts(coordinates)?));
    }
    let periods = nws.forecast(&nws.point(coordinates)?)?;
    let count = periods.len().min(common.periods.unwrap_or(2).into());
    Ok(forecast_announcement(&periods[..count]))
}

#[derive(Args, Debug)]
struct EspeakArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Audio format for the output file [default: from its extension, else wav]
    #[arg(short = 'a', long, value_enum, requires = "output")]
    audio_format: Option<AudioFormat>,

    /// Voice to use for speech
    #[arg(short, long, value_enum, default_value = "default")]
    voice: Voice,

    /// Speech speed in words per minute [default: from the voice]
    #[arg(short, long)]
    speed: Option<u32>,

    /// Voice pitch (0-99) [default: from the voice]
    #[arg(short = 'p', long)]
    pitch: Option<u32>,

    /// Gap between words in 10ms units [default: from the voice]
    #[arg(short = 'g', long)]
    gap: Option<u32>,
}

#[derive(Args, Debug)]
struct GoogleArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Audio format for the output file [default: from its extension, else mp3]
    #[arg(short = 'a', long, value_enum, requires = "output")]
    audio_format: Option<AudioFormat>,

    /// Voice to use for speech
    #[arg(short, long, value_enum, default_value = "default")]
    voice: Voice,

    /// Any Google voice by its full name, such as en-US-Chirp3-HD-Leda (overrides --voice)
    #[arg(long, conflicts_with = "voice")]
    voice_name: Option<String>,

    /// Volume relative to the voice's native level, in dB (-96 to 16)
    #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
    gain: f64,
}

#[derive(Args, Debug)]
struct TextArgs {
    #[command(flatten)]
    common: CommonArgs,
}

fn create_espeak_voice(
    voice: Voice,
    speed: Option<u32>,
    pitch: Option<u32>,
    gap: Option<u32>,
) -> EspeakVoice {
    let preset = EspeakVoice::from(voice);
    EspeakVoice {
        speed: speed.unwrap_or(preset.speed),
        pitch: pitch.unwrap_or(preset.pitch),
        gap: gap.unwrap_or(preset.gap),
        ..preset
    }
}

fn handle_espeak(args: EspeakArgs) -> Result<(), Box<dyn std::error::Error>> {
    let announcement = announcement(&args.common)?;
    println!("Announcement text: {}\n", announcement);

    let voice = create_espeak_voice(args.voice, args.speed, args.pitch, args.gap);
    let tts = EspeakTts::new(voice)?;
    let audio_format = AudioFormat::resolve(
        args.audio_format,
        args.common.output.as_deref(),
        AudioFormat::Wav,
    )?;
    execute_tts_output(&tts, &announcement, args.common.output, &audio_format)?;

    Ok(())
}

const API_KEY_VAR: &str = "GOOGLE_CLOUD_API_KEY";

/// `$XDG_CONFIG_HOME/speak-weather/google-api-key`, where the config directory defaults to
/// `~/.config`.
fn api_key_path(config_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let config = match config_home.filter(|dir| !dir.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(home?).join(".config"),
    };
    Some(config.join("speak-weather").join("google-api-key"))
}

fn read_api_key(path: &Path) -> Result<String, String> {
    let contents = fs::read_to_string(path).map_err(|e| {
        format!(
            "No Google API key: set {API_KEY_VAR} or put the key in {} ({e})",
            path.display()
        )
    })?;
    if let Ok(metadata) = fs::metadata(path)
        && metadata.permissions().mode() & 0o077 != 0
    {
        eprintln!(
            "Warning: {} can be read by other users; chmod 600 it",
            path.display()
        );
    }

    let key = contents.trim();
    if key.is_empty() {
        return Err(format!("{} is empty", path.display()));
    }
    Ok(key.to_string())
}

fn google_api_key() -> Result<String, String> {
    if let Ok(key) = std::env::var(API_KEY_VAR) {
        return Ok(key);
    }
    let path = api_key_path(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )
    .ok_or(format!(
        "No Google API key: set {API_KEY_VAR}, or HOME so the key file can be found"
    ))?;
    read_api_key(&path)
}

fn handle_google(args: GoogleArgs) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = google_api_key()?;

    let announcement = announcement(&args.common)?;
    println!("Announcement text: {}\n", announcement);

    let tts = match args.voice_name {
        Some(name) => GoogleTts::with_voice_name(api_key, &name)?,
        None => GoogleTts::new(api_key, args.voice.into()),
    }
    .with_gain(args.gain)?;
    let audio_format = AudioFormat::resolve(
        args.audio_format,
        args.common.output.as_deref(),
        AudioFormat::Mp3,
    )?;
    execute_tts_output(&tts, &announcement, args.common.output, &audio_format)?;

    Ok(())
}

fn handle_text(args: TextArgs) -> Result<(), Box<dyn std::error::Error>> {
    let announcement = announcement(&args.common)?;

    if let Some(output_path) = args.common.output {
        std::fs::write(&output_path, &announcement)?;
        println!("Text saved to: {}", output_path);
    } else {
        println!("{}", announcement.trim());
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Espeak(args) => handle_espeak(args),
        Commands::Google(args) => handle_google(args),
        Commands::Text(args) => handle_text(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn accepts_the_flags_used_by_allstar_weather() {
        Cli::try_parse_from([
            "speak-weather",
            "google",
            "--voice-name",
            "en-US-Chirp3-HD-Leda",
            "--gain",
            "6",
            "-f",
            "brief",
            "-a",
            "sln16",
            "-o",
            "out.sln16",
            "KSJC",
        ])
        .unwrap();
    }

    #[test]
    fn voice_presets_keep_their_settings_unless_overridden() {
        let preset = create_espeak_voice(Voice::UsMale, None, None, None);
        assert_eq!(preset.pitch, EspeakVoice::us_male().pitch);

        let overridden = create_espeak_voice(Voice::UsMale, Some(200), Some(70), Some(5));
        assert_eq!(
            (overridden.speed, overridden.pitch, overridden.gap),
            (200, 70, 5)
        );
        assert_eq!(overridden.name, EspeakVoice::us_male().name);
    }

    #[test]
    fn audio_format_requires_an_output_file() {
        for engine in ["espeak", "google"] {
            assert!(Cli::try_parse_from(["speak-weather", engine, "-a", "wav", "KSJC"]).is_err());
        }
    }

    #[test]
    fn temperature_unit_can_be_chosen() {
        Cli::try_parse_from(["speak-weather", "text", "-t", "celsius", "KSJC"]).unwrap();
        assert!(Cli::try_parse_from(["speak-weather", "text", "-t", "kelvin", "KSJC"]).is_err());
    }

    fn parse_text(args: &[&str]) -> Result<CommonArgs, clap::Error> {
        let args = ["speak-weather", "text"].iter().chain(args);
        match Cli::try_parse_from(args)?.command {
            Commands::Text(args) => Ok(args.common),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn one_report_is_chosen_and_metar_is_the_default() {
        assert_eq!(parse_text(&["KSJC"]).unwrap().report, Report::Metar);

        let common = parse_text(&[
            "-r",
            "forecast",
            "--periods",
            "4",
            "200 E Santa Clara St, San Jose, CA",
        ])
        .unwrap();
        assert_eq!(common.report, Report::Forecast);
        assert_eq!(common.periods, Some(4));
        assert_eq!(common.location, "200 E Santa Clara St, San Jose, CA");

        assert!(parse_text(&["-r", "taf", "KSJC"]).is_err());
        assert!(parse_text(&["-r", "forecast", "--periods", "0", "KSJC"]).is_err());
    }

    #[test]
    fn periods_are_only_for_the_forecast() {
        for report in ["metar", "alerts"] {
            let common = parse_text(&["-r", report, "--periods", "3", "KSJC"]).unwrap();
            assert!(
                announcement(&common)
                    .unwrap_err()
                    .to_string()
                    .contains("--periods")
            );
        }
    }

    #[test]
    fn api_key_file_is_in_the_config_directory() {
        let key_file = |config_home: Option<&str>, home: Option<&str>| {
            api_key_path(config_home.map(Into::into), home.map(Into::into))
        };
        let expected = |dir: &str| Some(Path::new(dir).join("speak-weather/google-api-key"));

        assert_eq!(key_file(Some("/xdg"), Some("/home/u")), expected("/xdg"));
        assert_eq!(key_file(None, Some("/home/u")), expected("/home/u/.config"));
        assert_eq!(
            key_file(Some(""), Some("/home/u")),
            expected("/home/u/.config")
        );
        assert_eq!(key_file(None, None), None);
    }

    #[test]
    fn api_key_file_is_read_and_trimmed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("google-api-key");

        assert!(read_api_key(&path).unwrap_err().contains(API_KEY_VAR));

        fs::write(&path, "  \n").unwrap();
        assert!(read_api_key(&path).unwrap_err().contains("empty"));

        fs::write(&path, "the-key\n").unwrap();
        assert_eq!(read_api_key(&path).unwrap(), "the-key");
    }
}
