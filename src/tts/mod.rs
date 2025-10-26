use std::error::Error;
use std::fmt;

pub mod announcements;
pub mod audio_conversion;
pub mod espeak;
pub mod google_tts;

pub use announcements::{AnnouncementFormat, generate_weather_announcement};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Voice {
    /// Default voice
    Default,
    /// US English female voice
    UsFemale,
    /// US English male voice
    UsMale,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum AudioFormat {
    /// MP3 format
    Mp3,
    /// WAV format (uncompressed)
    Wav,
    /// OGG format
    Ogg,
    /// ULAW format (8-bit G.711)
    Ulaw,
    /// ALAW format (8-bit G.711)
    Alaw,
    /// GSM format (telephony)
    Gsm,
}

impl AudioFormat {
    /// Get the appropriate sample rate for telephony compatibility
    pub fn telephony_sample_rate(&self) -> u32 {
        match self {
            AudioFormat::Ulaw | AudioFormat::Alaw | AudioFormat::Gsm => 8000,
            _ => 22050, // Standard quality for other formats
        }
    }

    /// Check if this format requires telephony-compatible sample rate
    pub fn is_telephony_format(&self) -> bool {
        matches!(
            self,
            AudioFormat::Ulaw | AudioFormat::Alaw | AudioFormat::Gsm
        )
    }

    pub fn file_extension(&self) -> &str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Wav => "wav",
            AudioFormat::Ogg => "ogg",
            AudioFormat::Ulaw => "wav",
            AudioFormat::Alaw => "wav",
            AudioFormat::Gsm => "gsm",
        }
    }

    pub fn supports_direct_playback(&self) -> bool {
        matches!(self, AudioFormat::Mp3 | AudioFormat::Wav | AudioFormat::Ogg)
    }
}

impl fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            AudioFormat::Mp3 => "MP3",
            AudioFormat::Wav => "WAV",
            AudioFormat::Ogg => "OGG",
            AudioFormat::Ulaw => "ULAW",
            AudioFormat::Alaw => "ALAW",
            AudioFormat::Gsm => "GSM",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug)]
pub enum TtsError {
    SynthesisError(String),
    AudioConversionError(String),
    PlaybackError(String),
    FileError(String),
}

impl fmt::Display for TtsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TtsError::SynthesisError(msg) => write!(f, "Speech synthesis error: {}", msg),
            TtsError::AudioConversionError(msg) => write!(f, "Audio conversion error: {}", msg),
            TtsError::PlaybackError(msg) => write!(f, "Audio playback error: {}", msg),
            TtsError::FileError(msg) => write!(f, "File operation error: {}", msg),
        }
    }
}

impl Error for TtsError {}

pub trait TtsBackend {
    /// Synthesize speech to audio data in the requested format
    fn synthesize(&self, text: &str, format: &AudioFormat) -> Result<Vec<u8>, TtsError>;

    /// Synthesize speech for direct playback (no audio data returned)
    fn speak(&self, text: &str) -> Result<(), TtsError>;

    /// Get the name of this TTS backend
    fn backend_name(&self) -> &str;
}

/// Common TTS operations shared by all backends
pub struct TtsPlayer;

impl TtsPlayer {
    pub fn save_audio_file(
        audio_data: &[u8],
        output_path: &str,
        _format: &AudioFormat,
    ) -> Result<(), TtsError> {
        let file_path = output_path.to_string();

        std::fs::write(&file_path, audio_data)
            .map_err(|e| TtsError::FileError(format!("Failed to write {}: {}", file_path, e)))?;

        println!("Audio saved to: {}", file_path);
        Ok(())
    }

    /// Convert audio data from one format to another
    /// This centralizes all audio conversion logic
    pub fn convert_audio_format(
        audio_data: &[u8],
        from_format: &AudioFormat,
        to_format: &AudioFormat,
    ) -> Result<Vec<u8>, TtsError> {
        // If formats are the same, no conversion needed
        if std::mem::discriminant(from_format) == std::mem::discriminant(to_format) {
            return Ok(audio_data.to_vec());
        }

        // Support conversions from WAV to telephony formats
        match (from_format, to_format) {
            (AudioFormat::Wav, target) if target.is_telephony_format() => {
                crate::tts::audio_conversion::convert_to_raw_telephony(audio_data, target)
            }
            _ => Err(TtsError::AudioConversionError(format!(
                "Conversion from {} to {} is not yet supported",
                from_format, to_format
            ))),
        }
    }

    pub fn play_audio(audio_data: &[u8], format: &AudioFormat) -> Result<(), TtsError> {
        if !format.supports_direct_playback() {
            return Err(TtsError::PlaybackError(format!(
                "{} format does not support direct playback. Use --output to save to file.",
                format
            )));
        }

        use std::io::Cursor;
        let (_stream, stream_handle) = rodio::OutputStream::try_default().map_err(|e| {
            TtsError::PlaybackError(format!("Failed to create audio stream: {}", e))
        })?;

        let sink = rodio::Sink::try_new(&stream_handle)
            .map_err(|e| TtsError::PlaybackError(format!("Failed to create audio sink: {}", e)))?;

        // Clone the audio data to avoid lifetime issues
        let audio_data_owned = audio_data.to_vec();
        let cursor = Cursor::new(audio_data_owned);
        let source = rodio::Decoder::new(cursor)
            .map_err(|e| TtsError::PlaybackError(format!("Failed to decode audio: {}", e)))?;

        sink.append(source);
        sink.sleep_until_end();
        Ok(())
    }
}

pub fn execute_tts_output<T: TtsBackend>(
    tts: &T,
    announcement: &str,
    output_path: Option<String>,
    audio_format: &AudioFormat,
) -> Result<(), TtsError> {
    if output_path.is_some() {
        println!("Generating audio file...");
    } else {
        println!("Speaking weather...");
    }

    if let Some(path) = output_path {
        let audio_data = tts.synthesize(announcement, audio_format)?;
        TtsPlayer::save_audio_file(&audio_data, &path, audio_format)?;
    } else {
        tts.speak(announcement)?;
    }

    Ok(())
}
