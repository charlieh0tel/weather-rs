use base64::Engine;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use weather::{
    celsius_to_fahrenheit, expand_abbreviations, fetch_weather_data, parse_wmo_codes, MetarData,
};

#[derive(Parser, Debug)]
#[command(author, version, about = "Speak aviation weather from aviationweather.gov", long_about = None)]
struct Args {
    /// ICAO airport identifier (e.g., KJFK, EGLL, KSFO)
    #[arg(value_name = "ICAO")]
    icao: String,

    /// Output format for audio
    #[arg(short, long, value_enum, default_value = "speech")]
    format: OutputFormat,

    /// Save audio to file instead of speaking
    #[arg(short, long)]
    output: Option<String>,

    /// Voice to use for speech
    #[arg(short, long, value_enum, default_value = "default")]
    voice: VoiceType,

    /// Audio format for output
    #[arg(short = 'a', long, value_enum, default_value = "mp3")]
    audio_format: AudioFormat,
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

#[derive(clap::ValueEnum, Clone, Debug)]
enum AudioFormat {
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
}

impl VoiceType {
    fn google_voice_name(&self) -> &str {
        match self {
            VoiceType::Default | VoiceType::UsFemale => "en-US-Neural2-F",
            VoiceType::UsMale => "en-US-Neural2-D",
            VoiceType::UkFemale => "en-GB-Neural2-A",
            VoiceType::UkMale => "en-GB-Neural2-B",
        }
    }

    fn language_code(&self) -> &str {
        match self {
            VoiceType::Default | VoiceType::UsFemale | VoiceType::UsMale => "en-US",
            VoiceType::UkFemale | VoiceType::UkMale => "en-GB",
        }
    }
}

impl AudioFormat {
    fn google_encoding(&self) -> &str {
        match self {
            AudioFormat::Mp3 => "MP3",
            AudioFormat::Wav => "LINEAR16",
            AudioFormat::Ogg => "OGG_OPUS",
            AudioFormat::Mulaw => "MULAW",
            AudioFormat::Alaw => "ALAW",
        }
    }

    fn file_extension(&self) -> &str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Wav => "wav",
            AudioFormat::Ogg => "ogg",
            AudioFormat::Mulaw => "wav",
            AudioFormat::Alaw => "wav",
        }
    }

    fn supports_playback(&self) -> bool {
        match self {
            AudioFormat::Mp3 | AudioFormat::Wav | AudioFormat::Ogg => true,
            AudioFormat::Mulaw | AudioFormat::Alaw => false, // Compressed formats typically not supported by rodio for direct playback
        }
    }
}

#[derive(Serialize)]
struct TtsRequest {
    input: TtsInput,
    voice: TtsVoice,
    #[serde(rename = "audioConfig")]
    audio_config: AudioConfig,
}

#[derive(Serialize)]
struct TtsInput {
    text: String,
}

#[derive(Serialize)]
struct TtsVoice {
    #[serde(rename = "languageCode")]
    language_code: String,
    name: String,
}

#[derive(Serialize)]
struct AudioConfig {
    #[serde(rename = "audioEncoding")]
    audio_encoding: String,
}

#[derive(Deserialize)]
struct TtsResponse {
    #[serde(rename = "audioContent")]
    audio_content: String,
}

fn speak_with_google_tts(
    text: &str,
    voice: &VoiceType,
    audio_format: &AudioFormat,
    api_key: &str,
    output_file: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = TtsRequest {
        input: TtsInput {
            text: text.to_string(),
        },
        voice: TtsVoice {
            language_code: voice.language_code().to_string(),
            name: voice.google_voice_name().to_string(),
        },
        audio_config: AudioConfig {
            audio_encoding: audio_format.google_encoding().to_string(),
        },
    };

    let client = reqwest::blocking::Client::new();
    let url = format!(
        "https://texttospeech.googleapis.com/v1/text:synthesize?key={}",
        api_key
    );

    let response = client.post(&url).json(&request).send()?;

    if !response.status().is_success() {
        let error_text = response.text()?;
        return Err(format!("Google TTS API error: {}", error_text).into());
    }

    let tts_response: TtsResponse = response.json()?;
    let audio_data =
        base64::engine::general_purpose::STANDARD.decode(&tts_response.audio_content)?;

    // Handle output file or playback
    if let Some(output_path) = output_file {
        // Save to file
        let file_path = if output_path.contains('.') {
            output_path.to_string()
        } else {
            format!("{}.{}", output_path, audio_format.file_extension())
        };

        std::fs::write(&file_path, &audio_data)?;
        println!("Audio saved to: {}", file_path);
    } else {
        // Play audio (if format supports playback)
        if audio_format.supports_playback() {
            let (_stream, stream_handle) = rodio::OutputStream::try_default()?;
            let sink = rodio::Sink::try_new(&stream_handle)?;

            let cursor = Cursor::new(audio_data);
            let source = rodio::Decoder::new(cursor)?;
            sink.append(source);
            sink.sleep_until_end();
        } else {
            println!(
                "Note: {} format does not support direct playback. Use --output to save to file.",
                audio_format.google_encoding()
            );
            return Err("Format does not support playback".into());
        }
    }

    Ok(())
}

fn spell_out_icao(icao: &str) -> String {
    icao.chars()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn generate_weather_announcement(metar: &MetarData, format: &OutputFormat) -> String {
    match format {
        OutputFormat::Speech | OutputFormat::Brief => {
            let mut announcement = format!("Weather for {}... ", spell_out_icao(&metar.icao_id));

            if let Some(ref name) = metar.name {
                announcement.push_str(&format!("{}... ", expand_abbreviations(name)));
            }

            if let Some(temp_c) = metar.temp {
                let temp_f = celsius_to_fahrenheit(temp_c);
                announcement.push_str(&format!(
                    "Temperature... {} degrees fahrenheit... ",
                    temp_f.round() as i32
                ));
            }

            if let Some(ref wx) = metar.wx_string {
                let codes = parse_wmo_codes(wx);
                if !codes.is_empty() {
                    announcement.push_str("Current conditions... ");
                    let conditions: Vec<String> =
                        codes.iter().map(|c| c.description().to_string()).collect();
                    announcement.push_str(&conditions.join("... "));
                    announcement.push_str("...");
                } else {
                    announcement.push_str("Clear conditions...");
                }
            } else {
                announcement.push_str("Clear conditions...");
            }

            announcement
        }

        OutputFormat::Detailed => {
            let mut announcement = format!(
                "Detailed weather report for {}... ",
                spell_out_icao(&metar.icao_id)
            );

            if let Some(ref name) = metar.name {
                announcement.push_str(&format!("{}... ", expand_abbreviations(name)));
            }

            announcement.push_str(&format!("Raw METAR... {}... ", metar.raw_ob));

            if let Some(temp_c) = metar.temp {
                let temp_f = celsius_to_fahrenheit(temp_c);
                announcement.push_str(&format!(
                    "Temperature... {} degrees fahrenheit... {} degrees celsius... ",
                    temp_f.round() as i32,
                    temp_c.round() as i32
                ));
            } else {
                announcement.push_str("Temperature... not available... ");
            }

            if let Some(ref wx) = metar.wx_string {
                announcement.push_str(&format!("Weather string... {}... ", wx));
                let codes = parse_wmo_codes(wx);
                if !codes.is_empty() {
                    announcement.push_str("Weather codes found... ");
                    let code_descriptions: Vec<String> = codes
                        .iter()
                        .map(|c| format!("{} ({})", c.description(), c.code()))
                        .collect();
                    announcement.push_str(&code_descriptions.join("... "));
                    announcement.push_str("...");
                } else {
                    announcement.push_str("No weather codes found...");
                }
            } else {
                announcement
                    .push_str("Weather... clear or not reported... No weather codes found...");
            }

            announcement
        }

        OutputFormat::Aviation => {
            let mut announcement = format!("{} weather... ", spell_out_icao(&metar.icao_id));

            if let Some(temp_c) = metar.temp {
                let temp_f = celsius_to_fahrenheit(temp_c);
                announcement.push_str(&format!(
                    "Temperature {} degrees... ",
                    temp_f.round() as i32
                ));
            }

            if let Some(ref wx) = metar.wx_string {
                let codes = parse_wmo_codes(wx);
                if !codes.is_empty() {
                    for code in codes {
                        announcement.push_str(&format!("{}... ", code.description()));
                    }
                } else {
                    announcement.push_str("Clear... ");
                }
            } else {
                announcement.push_str("Clear... ");
            }

            announcement.push_str("End weather...");
            announcement
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

    if args.output.is_some() {
        println!("Generating audio file...");
    } else {
        println!("Speaking weather...");
    }

    if let Err(e) = speak_with_google_tts(
        &announcement,
        &args.voice,
        &args.audio_format,
        &api_key,
        args.output.as_deref(),
    ) {
        eprintln!("TTS Error: {}", e);
        std::process::exit(1);
    }
}
