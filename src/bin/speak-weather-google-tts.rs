use clap::Parser;
use weather::{
    fetch_weather_data,
    tts::{
        AnnouncementFormat, AudioFormat, TtsBackend, TtsPlayer, generate_weather_announcement,
        google_tts::{GoogleTts, GoogleVoice},
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
    voice: VoiceType,

    /// Audio format for output
    #[arg(short = 'a', long, value_enum, default_value = "mp3")]
    audio_format: AudioFormatArg,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum VoiceType {
    /// Default neural voice
    Default,
    /// US English female neural voice
    UsFemale,
    /// US English male neural voice
    UsMale,
    /// UK English female neural voice
    UkFemale,
    /// UK English male neural voice
    UkMale,
}

impl From<VoiceType> for GoogleVoice {
    fn from(voice: VoiceType) -> Self {
        match voice {
            VoiceType::Default => GoogleVoice::Default,
            VoiceType::UsFemale => GoogleVoice::UsFemale,
            VoiceType::UsMale => GoogleVoice::UsMale,
            VoiceType::UkFemale => GoogleVoice::UkFemale,
            VoiceType::UkMale => GoogleVoice::UkMale,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum AudioFormatArg {
    /// MP3 format (best for playback)
    Mp3,
    /// WAV format (uncompressed)
    Wav,
    /// OGG format (open source)
    Ogg,
    /// MULAW format (8-bit G.711)
    Mulaw,
    /// ALAW format (8-bit G.711)
    Alaw,
    /// GSM format (telephony) - requires conversion
    Gsm,
}

impl From<AudioFormatArg> for AudioFormat {
    fn from(format: AudioFormatArg) -> Self {
        match format {
            AudioFormatArg::Mp3 => AudioFormat::Mp3,
            AudioFormatArg::Wav => AudioFormat::Wav,
            AudioFormatArg::Ogg => AudioFormat::Ogg,
            AudioFormatArg::Mulaw => AudioFormat::Mulaw,
            AudioFormatArg::Alaw => AudioFormat::Alaw,
            AudioFormatArg::Gsm => AudioFormat::Gsm,
        }
    }
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
    let audio_format = args.audio_format.into();

    if args.output.is_some() {
        println!("Generating audio file...");
    } else {
        println!("Speaking weather...");
    }

    // Handle output
    if let Some(output_path) = args.output {
        // File output mode - synthesize audio data
        let audio_data = match tts.synthesize(&announcement, &audio_format) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("TTS Error: {}", e);
                std::process::exit(1);
            }
        };

        if let Err(e) = TtsPlayer::save_audio_file(&audio_data, &output_path, &audio_format) {
            eprintln!("File Error: {}", e);
            std::process::exit(1);
        }
    } else {
        // Speaking mode - use direct speech
        if let Err(e) = tts.speak(&announcement) {
            eprintln!("TTS Error: {}", e);
            std::process::exit(1);
        }
    }
}
