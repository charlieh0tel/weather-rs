use crate::tts::{AudioFormat, TtsBackend, TtsError, Voice};
use base64::Engine;
use serde::{Deserialize, Serialize};

const SYNTHESIZE_URL: &str = "https://texttospeech.googleapis.com/v1/text:synthesize";

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
    voice_name: String,
    language_code: String,
    gain_db: f64,
}

/// The range of gains Google accepts.
const GAIN_DB_RANGE: std::ops::RangeInclusive<f64> = -96.0..=16.0;

impl GoogleTts {
    pub fn new(api_key: String, voice: GoogleVoice) -> Self {
        Self {
            api_key,
            voice_name: voice.google_voice_name().to_string(),
            language_code: voice.language_code().to_string(),
            gain_db: 0.0,
        }
    }

    /// Makes the voice louder or softer than its native volume by `gain_db` decibels.
    pub fn with_gain(self, gain_db: f64) -> Result<Self, TtsError> {
        if !GAIN_DB_RANGE.contains(&gain_db) {
            return Err(TtsError::SynthesisError(format!(
                "Gain must be from {} to {} dB, not {gain_db}",
                GAIN_DB_RANGE.start(),
                GAIN_DB_RANGE.end()
            )));
        }
        Ok(Self { gain_db, ..self })
    }

    /// Uses any Google voice by its full name, such as `en-US-Chirp3-HD-Leda`.
    pub fn with_voice_name(api_key: String, voice_name: &str) -> Result<Self, TtsError> {
        let language_code = language_code_of(voice_name).ok_or_else(|| {
            TtsError::SynthesisError(format!(
                "'{voice_name}' is not a Google voice name, which looks like en-US-Chirp3-HD-Leda"
            ))
        })?;
        Ok(Self {
            api_key,
            voice_name: voice_name.to_string(),
            language_code: language_code.to_string(),
            gain_db: 0.0,
        })
    }

    /// The key goes in a header because reqwest includes the URL in its error messages.
    fn request_builder(
        &self,
        client: &reqwest::blocking::Client,
        request: &TtsRequest,
    ) -> reqwest::blocking::RequestBuilder {
        client
            .post(SYNTHESIZE_URL)
            .header("x-goog-api-key", &self.api_key)
            .json(request)
    }
}

/// Raw formats are made from a WAV, which is what Google returns for LINEAR16.
fn google_encoding(format: &AudioFormat) -> &'static str {
    match format {
        AudioFormat::Mp3 => "MP3",
        AudioFormat::Ogg => "OGG_OPUS",
        AudioFormat::Wav
        | AudioFormat::Ulaw
        | AudioFormat::Alaw
        | AudioFormat::Gsm
        | AudioFormat::Sln16 => "LINEAR16",
    }
}

/// The language code is the first two parts of a voice name: "en-US" in "en-US-Neural2-F".
fn language_code_of(voice_name: &str) -> Option<&str> {
    let mut parts = voice_name.splitn(3, '-');
    let (language, region, variant) = (parts.next()?, parts.next()?, parts.next()?);
    if language.is_empty() || region.is_empty() || variant.is_empty() {
        return None;
    }
    Some(&voice_name[..language.len() + 1 + region.len()])
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
    #[serde(rename = "volumeGainDb")]
    volume_gain_db: f64,
}

#[derive(Deserialize)]
struct TtsResponse {
    #[serde(rename = "audioContent")]
    audio_content: String,
}

impl TtsBackend for GoogleTts {
    fn synthesize(&self, text: &str, format: &AudioFormat) -> Result<Vec<u8>, TtsError> {
        // Google resamples, so a WAV to be converted already has the target format's rate.
        let request = TtsRequest {
            input: TtsInput {
                text: text.to_string(),
            },
            voice: TtsVoice {
                language_code: self.language_code.clone(),
                name: self.voice_name.clone(),
            },
            audio_config: AudioConfig {
                audio_encoding: google_encoding(format).to_string(),
                sample_rate_hertz: format.sample_rate(),
                volume_gain_db: self.gain_db,
            },
        };

        let response = self
            .request_builder(&reqwest::blocking::Client::new(), &request)
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

        if format.is_raw() {
            crate::tts::TtsPlayer::convert_audio_format(&audio_data, &AudioFormat::Wav, format)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_is_sent_in_a_header_and_not_the_url() {
        const KEY: &str = "not-a-real-key";
        let tts = GoogleTts::new(KEY.to_string(), GoogleVoice::Default);
        let request = TtsRequest {
            input: TtsInput {
                text: "text".to_string(),
            },
            voice: TtsVoice {
                language_code: "en-US".to_string(),
                name: "name".to_string(),
            },
            audio_config: AudioConfig {
                audio_encoding: "MP3".to_string(),
                sample_rate_hertz: 22050,
                volume_gain_db: 0.0,
            },
        };

        let built = tts
            .request_builder(&reqwest::blocking::Client::new(), &request)
            .build()
            .unwrap();

        assert!(!built.url().as_str().contains(KEY));
        assert_eq!(built.headers()["x-goog-api-key"], KEY);
    }

    #[test]
    fn voice_names_give_their_language_code() {
        let cases = [
            ("en-US-Chirp3-HD-Leda", Some("en-US")),
            ("en-GB-Neural2-A", Some("en-GB")),
            ("cmn-CN-Wavenet-A", Some("cmn-CN")),
            ("Leda", None),
            ("en-US", None),
            ("en-US-", None),
            ("-US-Leda", None),
            ("", None),
        ];
        for (name, expected) in cases {
            assert_eq!(language_code_of(name), expected, "name: {name:?}");
        }
    }

    #[test]
    fn a_named_voice_is_used_in_the_request() {
        let tts = GoogleTts::with_voice_name("key".to_string(), "en-US-Chirp3-HD-Leda").unwrap();
        assert_eq!(tts.voice_name, "en-US-Chirp3-HD-Leda");
        assert_eq!(tts.language_code, "en-US");
        assert!(GoogleTts::with_voice_name("key".to_string(), "Leda").is_err());
    }

    #[test]
    fn gain_must_be_in_googles_range() {
        let tts = || GoogleTts::new("key".to_string(), GoogleVoice::Default);
        assert_eq!(tts().with_gain(6.0).unwrap().gain_db, 6.0);
        assert_eq!(tts().with_gain(-96.0).unwrap().gain_db, -96.0);
        assert!(tts().with_gain(16.5).is_err());
        assert!(tts().with_gain(f64::NAN).is_err());
    }
}
