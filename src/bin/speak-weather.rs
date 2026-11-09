use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use weather::{
    report::{ReportArgs, announcement},
    tts::{
        AudioFormat, Voice,
        espeak::{EspeakTts, EspeakVoice},
        execute_tts_output,
        google_tts::{self, GoogleTts},
    },
};

#[derive(Parser, Debug)]
#[command(name = "speak-weather", author, version, about = "Speak aviation weather with multiple TTS engines", long_about = None)]
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

#[derive(Args, Debug)]
struct CommonArgs {
    #[command(flatten)]
    report: ReportArgs,

    /// Save output to file instead of speaking/printing
    #[arg(short, long)]
    output: Option<String>,
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

    /// File holding the Google API key [default: $GOOGLE_CLOUD_API_KEY, else
    /// ~/.config/speak-weather/google-api-key]
    #[arg(long, value_name = "FILE")]
    api_key_file: Option<PathBuf>,
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
    let announcement = announcement(&args.common.report)?;
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

fn handle_google(args: GoogleArgs) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = google_tts::api_key(args.api_key_file.as_deref())?;

    let announcement = announcement(&args.common.report)?;
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
    let announcement = announcement(&args.common.report)?;

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
#[path = "common/man_page.rs"]
mod man_page;

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn man_pages_are_up_to_date() {
        man_page::check(Cli::command());
    }

    #[test]
    fn google_takes_a_voice_name_gain_and_raw_output() {
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
}
