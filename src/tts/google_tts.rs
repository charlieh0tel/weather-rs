use crate::tts::{AudioFormat, TtsBackend, TtsError, Voice};
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum GoogleVoice {
    /// Default neural voice (US English Female)
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

impl GoogleVoice {
    pub fn google_voice_name(&self) -> &str {
        match self {
            GoogleVoice::Default | GoogleVoice::UsFemale => "en-US-Neural2-F",
            GoogleVoice::UsMale => "en-US-Neural2-D",
            GoogleVoice::UkFemale => "en-GB-Neural2-A",
            GoogleVoice::UkMale => "en-GB-Neural2-B",
        }
    }

    pub fn language_code(&self) -> &str {
        match self {
            GoogleVoice::Default | GoogleVoice::UsFemale | GoogleVoice::UsMale => "en-US",
            GoogleVoice::UkFemale | GoogleVoice::UkMale => "en-GB",
        }
    }
}

impl From<Voice> for GoogleVoice {
    fn from(voice: Voice) -> Self {
        match voice {
            Voice::Default => GoogleVoice::Default,
            Voice::UsFemale => GoogleVoice::UsFemale,
            Voice::UsMale => GoogleVoice::UsMale,
            Voice::UkFemale => GoogleVoice::UkFemale,
            Voice::UkMale => GoogleVoice::UkMale,
        }
    }
}

pub struct GoogleTts {
    api_key: String,
    voice: GoogleVoice,
}

impl GoogleTts {
    pub fn new(api_key: String, voice: GoogleVoice) -> Self {
        Self { api_key, voice }
    }

    fn audio_format_to_google_encoding(&self, format: &AudioFormat) -> Result<&str, TtsError> {
        match format {
            AudioFormat::Mp3 => Ok("MP3"),
            AudioFormat::Wav => Ok("LINEAR16"),
            AudioFormat::Ogg => Ok("OGG_OPUS"),
            AudioFormat::Ulaw => Ok("MULAW"),
            AudioFormat::Alaw => Ok("ALAW"),
            AudioFormat::Gsm => Err(TtsError::AudioConversionError(
                "GSM format not directly supported by Google TTS. Use WAV and convert to GSM."
                    .to_string(),
            )),
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
    #[serde(rename = "sampleRateHertz")]
    sample_rate_hertz: u32,
}

#[derive(Deserialize)]
struct TtsResponse {
    #[serde(rename = "audioContent")]
    audio_content: String,
}

impl TtsBackend for GoogleTts {
    fn synthesize(&self, text: &str, format: &AudioFormat) -> Result<Vec<u8>, TtsError> {
        // For telephony formats, generate WAV and convert to raw format
        let (google_format, needs_conversion) = if format.is_telephony_format() {
            (&AudioFormat::Wav, true)
        } else {
            (format, false)
        };

        let encoding = self.audio_format_to_google_encoding(google_format)?;

        // Use appropriate sample rate for the format
        let sample_rate = google_format.telephony_sample_rate();

        let request = TtsRequest {
            input: TtsInput {
                text: text.to_string(),
            },
            voice: TtsVoice {
                language_code: self.voice.language_code().to_string(),
                name: self.voice.google_voice_name().to_string(),
            },
            audio_config: AudioConfig {
                audio_encoding: encoding.to_string(),
                sample_rate_hertz: sample_rate,
            },
        };

        let client = reqwest::blocking::Client::new();
        let url = format!(
            "https://texttospeech.googleapis.com/v1/text:synthesize?key={}",
            self.api_key
        );

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .map_err(|e| TtsError::SynthesisError(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TtsError::SynthesisError(format!(
                "Google TTS API error: {}",
                error_text
            )));
        }

        let tts_response: TtsResponse = response
            .json()
            .map_err(|e| TtsError::SynthesisError(format!("Failed to parse response: {}", e)))?;

        let audio_data = base64::engine::general_purpose::STANDARD
            .decode(&tts_response.audio_content)
            .map_err(|e| TtsError::SynthesisError(format!("Failed to decode audio: {}", e)))?;

        // Convert format if needed using centralized conversion
        if needs_conversion {
            crate::tts::TtsPlayer::convert_audio_format(&audio_data, google_format, format)
        } else {
            Ok(audio_data)
        }
    }

    fn speak(&self, text: &str) -> Result<(), TtsError> {
        // For Google TTS, generate audio and play it back
        let audio_data = self.synthesize(text, &AudioFormat::Mp3)?;
        crate::tts::TtsPlayer::play_audio(&audio_data, &AudioFormat::Mp3)
    }

    fn backend_name(&self) -> &str {
        "Google Cloud TTS"
    }
}
