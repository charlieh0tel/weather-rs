use clap::Parser;
use std::error::Error;
use std::fs::{self, Permissions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::thread;
use std::time::Duration;
use weather::{
    report::{ReportArgs, announcement},
    tts::{
        AudioFormat, TtsBackend, Voice,
        espeak::{EspeakTts, EspeakVoice},
        google_tts::{self, GoogleTts},
    },
};

const DEFAULT_VOICE_NAME: &str = "en-US-Chirp3-HD-Leda";
const DEFAULT_GAIN_DB: f64 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Engine {
    /// Google Cloud TTS (needs an API key)
    Google,
    /// eSpeak NG
    Espeak,
}

/// Announce the weather on an AllStar node.
///
/// Speaks a report and plays it on the node with `rpt localplay`, running
/// `sudo -u asterisk asterisk`, which must not ask for a password.
#[derive(Parser, Debug)]
#[command(name = "allstar-weather", version, mut_arg("format", |arg| arg.default_value("brief")))]
struct Cli {
    /// AllStar node number
    node: u32,

    #[command(flatten)]
    report: ReportArgs,

    /// Text-to-speech engine
    #[arg(short, long, value_enum, default_value = "google")]
    engine: Engine,

    /// Google voice name [default: en-US-Chirp3-HD-Leda]
    #[arg(short, long)]
    voice_name: Option<String>,

    /// Google volume gain in dB, -96 to 16 [default: 6]
    #[arg(short, long, allow_negative_numbers = true)]
    gain: Option<f64>,

    /// Audio format to play (sln16 needs Asterisk's codec_resample module)
    #[arg(short, long, value_enum, default_value = "sln16")]
    audio_format: AudioFormat,

    /// Seconds a busy node gets to open the audio file before it is deleted
    #[arg(short, long, default_value_t = 60)]
    wait: u64,
}

fn tts(cli: &Cli) -> Result<Box<dyn TtsBackend>, Box<dyn Error>> {
    match cli.engine {
        Engine::Google => {
            let voice_name = cli.voice_name.as_deref().unwrap_or(DEFAULT_VOICE_NAME);
            let tts = GoogleTts::with_voice_name(google_tts::api_key()?, voice_name)?
                .with_gain(cli.gain.unwrap_or(DEFAULT_GAIN_DB))?;
            Ok(Box::new(tts))
        }
        Engine::Espeak => {
            if cli.voice_name.is_some() || cli.gain.is_some() {
                return Err("--voice-name and --gain are only for --engine google".into());
            }
            Ok(Box::new(EspeakTts::new(EspeakVoice::from(Voice::Default))?))
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    let tts = tts(&cli)?;
    let text = announcement(&cli.report)?;
    println!("Announcement text: {text}\n");
    let audio = tts.synthesize(&text, &cli.audio_format)?;

    // Asterisk runs as another user, so the file goes where it can read it.
    let mut file = tempfile::Builder::new()
        .prefix("allstar-weather-")
        .suffix(&format!(".{}", cli.audio_format.file_extension()))
        .tempfile_in("/tmp")?;
    file.write_all(&audio)?;
    fs::set_permissions(file.path(), Permissions::from_mode(0o644))?;

    // localplay takes the file name without its extension.
    let sound = file.path().with_extension("");
    let command = format!("rpt localplay {} {}", cli.node, sound.display());
    let status = Command::new("sudo")
        .args(["-u", "asterisk", "asterisk", "-x", &command])
        .status()?;
    if !status.success() {
        return Err(format!("Asterisk failed to run '{command}': {status}").into());
    }

    thread::sleep(Duration::from_secs(cli.wait));
    Ok(())
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
    use weather::report::Report;
    use weather::tts::AnnouncementFormat;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(["allstar-weather"].iter().chain(args))
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
    fn defaults_suit_a_node() {
        let cli = parse(&["65314", "KSJC"]).unwrap();
        assert_eq!(cli.node, 65314);
        assert_eq!(cli.report.location, "KSJC");
        assert_eq!(cli.report.report, Report::Metar);
        assert!(matches!(cli.report.format, AnnouncementFormat::Brief));
        assert_eq!(cli.engine, Engine::Google);
        assert!(matches!(cli.audio_format, AudioFormat::Sln16));
        assert_eq!(cli.wait, 60);
    }

    #[test]
    fn long_and_short_options_are_the_same() {
        let short = parse(&[
            "-r",
            "forecast",
            "-e",
            "google",
            "-v",
            "en-US-Neural2-F",
            "-g",
            "-3",
            "-a",
            "ulaw",
            "-w",
            "5",
            "65314",
            "KSJC",
        ])
        .unwrap();
        let long = parse(&[
            "--report",
            "forecast",
            "--engine",
            "google",
            "--voice-name",
            "en-US-Neural2-F",
            "--gain",
            "-3",
            "--audio-format",
            "ulaw",
            "--wait",
            "5",
            "65314",
            "KSJC",
        ])
        .unwrap();
        assert_eq!(format!("{short:?}"), format!("{long:?}"));
        assert_eq!(long.gain, Some(-3.0));
    }

    #[test]
    fn google_options_are_refused_for_espeak() {
        for flags in [["--gain", "3"], ["--voice-name", "en-US-Neural2-F"]] {
            let args: Vec<&str> = ["-e", "espeak"]
                .into_iter()
                .chain(flags)
                .chain(["65314", "KSJC"])
                .collect();
            let message = tts(&parse(&args).unwrap()).err().unwrap().to_string();
            assert!(message.contains("--engine google"), "{message}");
        }
    }

    #[test]
    fn a_node_and_location_are_required() {
        assert!(parse(&["KSJC"]).is_err());
        assert!(parse(&["node", "KSJC"]).is_err());
    }
}
