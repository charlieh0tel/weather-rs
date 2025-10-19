use clap::Parser;
use weather::{
    fetch_weather_data,
    tts::{
        AnnouncementFormat, AudioFormat, TtsBackend, TtsPlayer,
        espeak::{EspeakTts, EspeakVoice},
        generate_weather_announcement,
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
    format: OutputFormat,

    /// Save audio to file instead of speaking
    #[arg(short, long)]
    output: Option<String>,

    /// Voice to use for speech
    #[arg(short, long, value_enum, default_value = "default")]
    voice: VoiceType,

    /// Audio format for output
    #[arg(short = 'a', long, value_enum, default_value = "wav")]
    audio_format: AudioFormatArg,

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

#[derive(clap::ValueEnum, Clone, Debug)]
enum OutputFormat {
    /// Direct speech output
    Speech,
    /// Brief announcement format
    Brief,
    /// Detailed weather report
    Detailed,
    /// Aviation radio style
    Aviation,
}

impl From<OutputFormat> for AnnouncementFormat {
    fn from(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Speech => AnnouncementFormat::Speech,
            OutputFormat::Brief => AnnouncementFormat::Brief,
            OutputFormat::Detailed => AnnouncementFormat::Detailed,
            OutputFormat::Aviation => AnnouncementFormat::Aviation,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum VoiceType {
    /// Default voice
    Default,
    /// US English female voice
    UsFemale,
    /// US English male voice
    UsMale,
    /// UK English female voice
    UkFemale,
    /// UK English male voice
    UkMale,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum AudioFormatArg {
    /// WAV format (native eSpeak output)
    Wav,
    /// MP3 format (requires conversion)
    Mp3,
    /// OGG format (requires conversion)
    Ogg,
    /// MULAW format (requires conversion)
    Mulaw,
    /// ALAW format (requires conversion)
    Alaw,
    /// GSM format (requires conversion)
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

fn create_espeak_voice(voice_type: VoiceType, speed: u32, pitch: u32, gap: u32) -> EspeakVoice {
    let mut voice = match voice_type {
        VoiceType::Default => EspeakVoice::default(),
        VoiceType::UsFemale => EspeakVoice::us_female(),
        VoiceType::UsMale => EspeakVoice::us_male(),
        VoiceType::UkFemale => EspeakVoice::uk_female(),
        VoiceType::UkMale => EspeakVoice::uk_male(),
    };

    voice.speed = speed;
    voice.pitch = pitch;
    voice.gap = gap;

    voice
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

    let announcement = generate_weather_announcement(&metar, &args.format.into());
    println!("Announcement text: {}\n", announcement);

    let voice = create_espeak_voice(args.voice, args.speed, args.pitch, args.gap);
    let tts = match EspeakTts::new(voice) {
        Ok(tts) => tts,
        Err(e) => {
            eprintln!("Failed to initialize eSpeak: {}", e);
            std::process::exit(1);
        }
    };
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
