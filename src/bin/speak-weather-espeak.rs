use clap::Parser;
use weather::{
    fetch_weather_data,
    tts::{
        AnnouncementFormat, AudioFormat, Voice,
        espeak::{EspeakTts, EspeakVoice},
        execute_tts_output, generate_weather_announcement,
    },
};

#[derive(Parser, Debug)]
#[command(author, version, about = "Speak aviation weather using eSpeak TTS", long_about = None)]
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
    #[arg(short = 'a', long, value_enum, default_value = "wav")]
    audio_format: AudioFormat,

    /// Speech speed in words per minute
    #[arg(short, long, default_value = "120")]
    speed: u32,

    /// Voice pitch (0-99)
    #[arg(short = 'p', long, default_value = "50")]
    pitch: u32,

    /// Gap between words in 10ms units
    #[arg(short = 'g', long, default_value = "15")]
    gap: u32,
}

fn create_espeak_voice(voice: Voice, speed: u32, pitch: u32, gap: u32) -> EspeakVoice {
    let mut espeak_voice: EspeakVoice = voice.into();
    espeak_voice.speed = speed;
    espeak_voice.pitch = pitch;
    espeak_voice.gap = gap;
    espeak_voice
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

    let voice = create_espeak_voice(args.voice, args.speed, args.pitch, args.gap);
    let tts = match EspeakTts::new(voice) {
        Ok(tts) => tts,
        Err(e) => {
            eprintln!("Failed to initialize eSpeak: {}", e);
            std::process::exit(1);
        }
    };
    if let Err(e) = execute_tts_output(&tts, &announcement, args.output, &args.audio_format) {
        eprintln!("TTS Error: {}", e);
        std::process::exit(1);
    }
}
