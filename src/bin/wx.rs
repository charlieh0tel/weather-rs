use clap::{ArgGroup, Args, Parser, Subcommand};
use std::error::Error;
use std::fs::{self, Permissions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command as Process;
use std::thread;
use std::time::Duration;
use weather::{
    nws::{Location, NwsClient},
    report,
    tts::{
        AnnouncementFormat, AudioFormat, TemperatureUnit, TtsBackend, Voice,
        espeak::{EspeakTts, EspeakVoice},
        execute_tts_output, generate_weather_announcement,
        google_tts::{self, GoogleTts},
    },
};

const DEFAULT_VOICE: &str = "en-US-Chirp3-HD-Leda";
const DEFAULT_GAIN_DB: f64 = 6.0;
const ESPEAK: &str = "espeak";

/// Weather reports, printed, spoken or played on an AllStar node.
///
/// Each report takes a place, and prints its announcement unless told to speak it (--say),
/// write it to an audio file (--out) or play it on an AllStar node (--node).
#[derive(Parser, Debug)]
#[command(name = "wx", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Current conditions from the nearest METAR station, from aviationweather.gov
    Metar(MetarArgs),
    /// The National Weather Service forecast (US only)
    Nws(NwsArgs),
    /// Active National Weather Service alerts (US only)
    Alerts(AlertsArgs),
    /// The National Weather Service office, grid, zones and nearest stations for a place
    Where(WhereArgs),
}

#[derive(Args, Debug)]
struct PlaceArgs {
    /// A station (KSJC), latitude,longitude, NWS grid square (MTR 99,82) or zone (CAZ513), US
    /// street address or place name ("Mountain View, CA"); quotes are optional
    #[arg(value_name = "PLACE", required = true, num_args = 1.., allow_negative_numbers = true)]
    place: Vec<String>,
}

impl PlaceArgs {
    fn location(&self) -> Result<Location, weather::WeatherError> {
        self.place.join(" ").parse()
    }
}

#[derive(Args, Debug)]
#[command(group(ArgGroup::new("destination").args(["say", "out", "node"])))]
struct OutputArgs {
    /// Speak the announcement on this machine
    #[arg(long)]
    say: bool,

    /// Write the announcement as audio to FILE, in the format its extension names
    #[arg(short, long, value_name = "FILE")]
    out: Option<PathBuf>,

    /// Play the announcement on this AllStar node
    #[arg(long, value_name = "NODE")]
    node: Option<u32>,

    /// With --node: send it to linked nodes too, not only the node's own transmitter
    #[arg(short = 'l', long, requires = "node")]
    to_links: bool,

    /// Audio format [default: from the file's extension with --out, sln16 with --node]
    #[arg(short, long, value_enum)]
    audio_format: Option<AudioFormat>,

    /// With --node: seconds the node gets to open the audio file before it is deleted
    #[arg(short, long, default_value_t = 60, requires = "node")]
    wait: u64,
}

#[derive(Args, Debug)]
struct VoiceArgs {
    /// A Google voice name, or espeak
    #[arg(short, long, default_value = DEFAULT_VOICE)]
    voice: String,

    /// Google volume gain in dB, -96 to 16 [default: 6]
    #[arg(short, long, allow_negative_numbers = true)]
    gain: Option<f64>,

    /// File holding the Google API key [default: $GOOGLE_CLOUD_API_KEY, else
    /// ~/.config/weather-rs/google-api-key]
    #[arg(long, value_name = "FILE")]
    api_key_file: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct MetarArgs {
    #[command(flatten)]
    place: PlaceArgs,

    /// How to word the announcement [default: brief with --node, else full]
    #[arg(short, long, value_enum)]
    style: Option<AnnouncementFormat>,

    /// Temperature unit (the detailed style always gives both)
    #[arg(short, long, value_enum, default_value = "fahrenheit")]
    temp_unit: TemperatureUnit,

    /// Print the raw METAR instead of an announcement
    #[arg(long, conflicts_with_all = ["say", "out", "node"])]
    raw: bool,

    #[command(flatten)]
    output: OutputArgs,

    #[command(flatten)]
    voice: VoiceArgs,
}

#[derive(Args, Debug)]
struct NwsArgs {
    #[command(flatten)]
    place: PlaceArgs,

    /// Number of half-day forecast periods
    #[arg(short, long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(1..))]
    periods: u8,

    /// Leave out active alerts, which are otherwise read before the forecast (nothing is said
    /// about them when there are none)
    #[arg(long)]
    no_alerts: bool,

    #[command(flatten)]
    output: OutputArgs,

    #[command(flatten)]
    voice: VoiceArgs,
}

#[derive(Args, Debug)]
struct AlertsArgs {
    #[command(flatten)]
    place: PlaceArgs,

    /// Announce nothing when no alerts are in effect, instead of saying so
    #[arg(long)]
    only_if_any: bool,

    #[command(flatten)]
    output: OutputArgs,

    #[command(flatten)]
    voice: VoiceArgs,
}

#[derive(Args, Debug)]
struct WhereArgs {
    #[command(flatten)]
    place: PlaceArgs,

    /// Number of nearby observation stations to list
    #[arg(short = 'n', long, default_value_t = 5)]
    stations: usize,
}

fn print_where(args: &WhereArgs) -> Result<(), Box<dyn Error>> {
    let nws = NwsClient::new()?;
    let place = nws.locate(&args.place.location()?)?;
    let point = nws.point(place.coordinates)?;
    let stations = if args.stations > 0 {
        nws.stations(&point, args.stations)?
    } else {
        Vec::new()
    };

    let unknown = || "unknown".to_string();
    let field = |label: &str, value: &str| println!("{label:<17}{value}");

    match &place.name {
        Some(name) => field("Location:", &format!("{} ({name})", place.coordinates)),
        None => field("Location:", &place.coordinates.to_string()),
    }
    if let Some(city) = point.nearest_city() {
        field("Nearest city:", &format!("{}, {}", city.city, city.state));
    }
    field("Forecast office:", &point.office);
    field("Grid:", &point.grid());
    field(
        "Forecast zone:",
        &point.forecast_zone().map_or_else(unknown, str::to_string),
    );
    field(
        "County zone:",
        &point.county_zone().map_or_else(unknown, str::to_string),
    );
    if let Some(time_zone) = &point.time_zone {
        field("Time zone:", time_zone);
    }
    for (i, station) in stations.iter().enumerate() {
        let label = if i == 0 { "Stations:" } else { "" };
        let distance = station
            .distance
            .map(|metres| format!(" ({:.1} km)", metres / 1000.0))
            .unwrap_or_default();
        field(
            label,
            &format!(
                "{:<6} {}{distance}",
                station.station_identifier, station.name
            ),
        );
    }
    Ok(())
}

fn tts(voice: &VoiceArgs) -> Result<Box<dyn TtsBackend>, Box<dyn Error>> {
    if voice.voice == ESPEAK {
        if voice.gain.is_some() || voice.api_key_file.is_some() {
            return Err("--gain and --api-key-file are only for Google voices".into());
        }
        return Ok(Box::new(EspeakTts::new(EspeakVoice::from(Voice::Default))?));
    }
    let api_key = google_tts::api_key(voice.api_key_file.as_deref())?;
    let tts = GoogleTts::with_voice_name(api_key, &voice.voice)?
        .with_gain(voice.gain.unwrap_or(DEFAULT_GAIN_DB))?;
    Ok(Box::new(tts))
}

/// Plays audio on an AllStar node, by running `sudo -u asterisk asterisk`.
fn play_on_node(
    audio: &[u8],
    format: &AudioFormat,
    output: &OutputArgs,
    node: u32,
) -> Result<(), Box<dyn Error>> {
    // Asterisk runs as another user, so the file goes where it can read it.
    let mut file = tempfile::Builder::new()
        .prefix("wx-")
        .suffix(&format!(".{}", format.file_extension()))
        .tempfile_in("/tmp")?;
    file.write_all(audio)?;
    fs::set_permissions(file.path(), Permissions::from_mode(0o644))?;

    // localplay plays on the node's own transmitter; playback also sends to linked nodes.  Both
    // take the file name without its extension.
    let play = if output.to_links {
        "playback"
    } else {
        "localplay"
    };
    let command = format!(
        "rpt {play} {node} {}",
        file.path().with_extension("").display()
    );
    // From a directory the asterisk user can read, or Asterisk warns about it.
    let status = Process::new("sudo")
        .args(["-u", "asterisk", "asterisk", "-x", &command])
        .current_dir("/")
        .status()?;
    if !status.success() {
        return Err(format!("Asterisk failed to run '{command}': {status}").into());
    }

    thread::sleep(Duration::from_secs(output.wait));
    Ok(())
}

fn deliver(text: &str, output: &OutputArgs, voice: &VoiceArgs) -> Result<(), Box<dyn Error>> {
    if !(output.say || output.out.is_some() || output.node.is_some()) {
        if output.audio_format.is_some() {
            return Err("--audio-format needs --out or --node".into());
        }
        println!("{}", text.trim());
        return Ok(());
    }

    let tts = tts(voice)?;
    eprintln!("Announcement text: {text}\n");
    if let Some(node) = output.node {
        let format = output.audio_format.clone().unwrap_or(AudioFormat::Sln16);
        let audio = tts.synthesize(text, &format)?;
        return play_on_node(&audio, &format, output, node);
    }
    let path = output
        .out
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned());
    let default = if voice.voice == ESPEAK {
        AudioFormat::Wav
    } else {
        AudioFormat::Mp3
    };
    let format = AudioFormat::resolve(output.audio_format.clone(), path.as_deref(), default)?;
    execute_tts_output(&*tts, text, path, &format)?;
    Ok(())
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    let (text, output, voice) = match &cli.command {
        Command::Where(args) => return print_where(args),
        Command::Metar(args) => {
            let metar = report::metar(&args.place.location()?)?;
            let text = if args.raw {
                metar.raw_ob
            } else {
                let style = args.style.clone().unwrap_or(match args.output.node {
                    Some(_) => AnnouncementFormat::Brief,
                    None => AnnouncementFormat::Full,
                });
                generate_weather_announcement(&metar, &style, args.temp_unit)
            };
            (Some(text), &args.output, &args.voice)
        }
        Command::Nws(args) => (
            Some(report::nws_forecast(
                &args.place.location()?,
                args.periods.into(),
                !args.no_alerts,
            )?),
            &args.output,
            &args.voice,
        ),
        Command::Alerts(args) => (
            report::alerts(&args.place.location()?, args.only_if_any)?,
            &args.output,
            &args.voice,
        ),
    };
    match text {
        Some(text) => deliver(&text, output, voice),
        None => {
            eprintln!("{}", report::NOTHING_TO_ANNOUNCE);
            Ok(())
        }
    }
}

fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
#[path = "common/man_page.rs"]
mod man_page;

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(["wx"].iter().chain(args))
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn man_pages_are_up_to_date() {
        man_page::check(Cli::command());
    }

    #[test]
    fn places_need_no_quotes() {
        let Command::Metar(args) = parse(&["metar", "Mountain", "View,", "CA"])
            .unwrap()
            .command
        else {
            panic!()
        };
        assert_eq!(
            args.place.location().unwrap(),
            Location::Name("Mountain View, CA".to_string())
        );

        let Command::Nws(args) = parse(&["nws", "37.3,", "-121.9"]).unwrap().command else {
            panic!()
        };
        assert!(matches!(
            args.place.location().unwrap(),
            Location::Coordinates(_)
        ));
    }

    #[test]
    fn options_belong_to_their_report() {
        assert!(parse(&["nws", "--periods", "3", "KSJC"]).is_ok());
        assert!(parse(&["nws", "--no-alerts", "KSJC"]).is_ok());
        assert!(parse(&["metar", "--no-alerts", "KSJC"]).is_err());
        assert!(parse(&["alerts", "--only-if-any", "KSJC"]).is_ok());
        assert!(parse(&["metar", "--periods", "3", "KSJC"]).is_err());
        assert!(parse(&["nws", "--only-if-any", "KSJC"]).is_err());
        assert!(parse(&["alerts", "--style", "brief", "KSJC"]).is_err());
        assert!(parse(&["nws", "--periods", "0", "KSJC"]).is_err());
        assert!(parse(&["metar"]).is_err());
    }

    #[test]
    fn one_destination_at_a_time() {
        assert!(parse(&["metar", "KSJC", "--node", "65314", "--to-links"]).is_ok());
        assert!(parse(&["metar", "KSJC", "--say", "--node", "65314"]).is_err());
        assert!(parse(&["metar", "KSJC", "--say", "-o", "x.mp3"]).is_err());
        assert!(parse(&["metar", "KSJC", "--to-links"]).is_err());
        assert!(parse(&["metar", "KSJC", "--wait", "5"]).is_err());
        assert!(parse(&["metar", "KSJC", "--raw", "--node", "65314"]).is_err());
    }

    #[test]
    fn google_options_are_refused_for_espeak() {
        for flags in [["--gain", "3"], ["--api-key-file", "key"]] {
            let args: Vec<&str> = ["metar", "KSJC", "--voice", "espeak"]
                .into_iter()
                .chain(flags)
                .collect();
            let Command::Metar(args) = parse(&args).unwrap().command else {
                panic!()
            };
            let message = tts(&args.voice).err().unwrap().to_string();
            assert!(message.contains("only for Google voices"), "{message}");
        }
    }
}
