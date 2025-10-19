use clap::Parser;
use weather::{
    fetch_weather_data,
    tts::{
        AnnouncementFormat, AudioFormat, Voice, execute_tts_output, generate_weather_announcement,
        google_tts::GoogleTts,
    },
};

#[derive(Parser, Debug)]
#[command(author, version, about = "Speak aviation weather using Google Cloud TTS", long_about = None)]
struct Args {
    /// ICAO airport identifier (e.g., KJFK, EGLL, KSFO)
    #[arg(value_name = "ICAO")]
    icao: String,

    /// Output format for announcement
    #[arg(short, long, value_enum, default_value = "speech")]
    format: AnnouncementFormat,

    /// Save audio to file instead of speaking
    #[arg(short, long)]
    output: Option<String>,

    /// Voice to use for speech
    #[arg(short, long, value_enum, default_value = "default")]
    voice: Voice,

    /// Audio format for output
    #[arg(short = 'a', long, value_enum, default_value = "mp3")]
    audio_format: AudioFormat,
}

fn main() {
    let args = Args::parse();

    println!("Fetching weather for {}...\n", args.icao.to_uppercase());

    let metar = match fetch_weather_data(&args.icao) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let announcement = generate_weather_announcement(&metar, &args.format);
    println!("Announcement text: {}\n", announcement);

    // Get Google Cloud API key from environment
    let api_key = match std::env::var("GOOGLE_CLOUD_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Error: GOOGLE_CLOUD_API_KEY environment variable not set");
            eprintln!("Please set your Google Cloud TTS API key:");
            eprintln!("export GOOGLE_CLOUD_API_KEY=your_api_key_here");
            std::process::exit(1);
        }
    };

    let tts = GoogleTts::new(api_key, args.voice.into());

    if let Err(e) = execute_tts_output(&tts, &announcement, args.output, &args.audio_format) {
        eprintln!("TTS Error: {}", e);
        std::process::exit(1);
    }
}
