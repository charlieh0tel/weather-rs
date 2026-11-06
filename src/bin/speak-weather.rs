use clap::{Args, Parser, Subcommand};
use weather::{
    fetch_weather_data,
    tts::{
        AnnouncementFormat, AudioFormat, Voice,
        espeak::{EspeakTts, EspeakVoice},
        execute_tts_output, generate_weather_announcement,
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

#[derive(Args, Debug)]
struct CommonArgs {
    /// ICAO airport identifier (e.g., KJFK, EGLL, KSFO)
    icao: String,

    /// Output format for announcement
    #[arg(short, long, value_enum, default_value = "speech")]
    format: AnnouncementFormat,

    /// Save output to file instead of speaking/printing
    #[arg(short, long)]
    output: Option<String>,
}

#[derive(Args, Debug)]
struct EspeakArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Audio format for output
    #[arg(short = 'a', long, value_enum, default_value = "wav")]
    audio_format: AudioFormat,

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

    /// Audio format for output
    #[arg(short = 'a', long, value_enum, default_value = "mp3")]
    audio_format: AudioFormat,

    /// Voice to use for speech
    #[arg(short, long, value_enum, default_value = "default")]
    voice: Voice,
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
    println!(
        "Fetching weather for {}...\n",
        args.common.icao.to_uppercase()
    );

    let metar = fetch_weather_data(&args.common.icao)?;
    let announcement = generate_weather_announcement(&metar, &args.common.format);
    println!("Announcement text: {}\n", announcement);

    let voice = create_espeak_voice(args.voice, args.speed, args.pitch, args.gap);
    let tts = EspeakTts::new(voice)?;
    execute_tts_output(&tts, &announcement, args.common.output, &args.audio_format)?;

    Ok(())
}

fn handle_google(args: GoogleArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Fetching weather for {}...\n",
        args.common.icao.to_uppercase()
    );

    let metar = fetch_weather_data(&args.common.icao)?;
    let announcement = generate_weather_announcement(&metar, &args.common.format);
    println!("Announcement text: {}\n", announcement);

    // Get Google Cloud API key from environment
    let api_key = std::env::var("GOOGLE_CLOUD_API_KEY")
        .map_err(|_| "GOOGLE_CLOUD_API_KEY environment variable not set. Please set your Google Cloud TTS API key: export GOOGLE_CLOUD_API_KEY=your_api_key_here")?;

    let tts = GoogleTts::new(api_key, args.voice.into());
    execute_tts_output(&tts, &announcement, args.common.output, &args.audio_format)?;

    Ok(())
}

fn handle_text(args: TextArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Fetching weather for {}...\n",
        args.common.icao.to_uppercase()
    );

    let metar = fetch_weather_data(&args.common.icao)?;
    let announcement = generate_weather_announcement(&metar, &args.common.format);

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
    fn accepts_the_flags_used_by_the_w6otx_script() {
        Cli::try_parse_from([
            "speak-weather",
            "espeak",
            "-v",
            "uk-female",
            "-f",
            "brief",
            "-a",
            "ulaw",
            "-o",
            "out.ulaw",
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
}
