use crate::tts::{AudioFormat, TtsBackend, TtsError};
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
            AudioFormat::Mulaw => Ok("MULAW"),
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
}

#[derive(Deserialize)]
struct TtsResponse {
    #[serde(rename = "audioContent")]
    audio_content: String,
}

impl TtsBackend for GoogleTts {
    fn synthesize(&self, text: &str, format: &AudioFormat) -> Result<Vec<u8>, TtsError> {
        // For GSM, we'll synthesize as WAV first, then convert
        let actual_format = if matches!(format, AudioFormat::Gsm) {
            &AudioFormat::Wav
        } else {
            format
        };

        let encoding = self.audio_format_to_google_encoding(actual_format)?;

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

        let mut audio_data = base64::engine::general_purpose::STANDARD
            .decode(&tts_response.audio_content)
            .map_err(|e| TtsError::SynthesisError(format!("Failed to decode audio: {}", e)))?;

        // Convert to GSM if requested
        if matches!(format, AudioFormat::Gsm) {
            audio_data = convert_wav_to_gsm(&audio_data)?;
        }

        Ok(audio_data)
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

// Placeholder for GSM conversion - would need actual implementation
fn convert_wav_to_gsm(_wav_data: &[u8]) -> Result<Vec<u8>, TtsError> {
    Err(TtsError::AudioConversionError(
        "GSM conversion not yet implemented. Install FFmpeg for audio format conversion."
            .to_string(),
    ))
}
