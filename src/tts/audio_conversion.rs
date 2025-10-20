use crate::tts::TtsError;

use std::io::Write;
/// Audio format conversion utilities
///
/// This module provides functions to convert between different audio formats,
/// primarily for telephony applications that require specific formats like GSM.
use std::process::{Command, Stdio};

/// Convert WAV audio data to GSM format using sox
pub fn convert_wav_to_gsm(wav_data: &[u8]) -> Result<Vec<u8>, TtsError> {
    convert_wav_to_telephony_format(wav_data, "gsm", "GSM")
}

/// Convert WAV audio data to telephony format using sox
fn convert_wav_to_telephony_format(
    wav_data: &[u8],
    sox_format: &str,
    format_name: &str,
) -> Result<Vec<u8>, TtsError> {
    let mut sox = Command::new("sox")
        .args([
            "-t", "wav", "-", // Read WAV from stdin
            "-t", sox_format, // Output format
            "-r", "8000", // 8kHz sample rate
            "-c", "1", // Mono
            "-", // Write to stdout
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            TtsError::AudioConversionError(format!(
                "Failed to spawn sox for {} conversion: {}",
                format_name, e
            ))
        })?;

    // Write WAV data to sox stdin
    if let Some(stdin) = sox.stdin.take() {
        let wav_data_owned = wav_data.to_vec();
        std::thread::spawn(move || {
            let mut stdin = stdin;
            let _ = stdin.write_all(&wav_data_owned);
        });
    }

    // Get output
    let output = sox.wait_with_output().map_err(|e| {
        TtsError::AudioConversionError(format!("Sox {} conversion failed: {}", format_name, e))
    })?;

    if !output.status.success() {
        return Err(TtsError::AudioConversionError(format!(
            "Sox {} conversion failed: {}",
            format_name,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(output.stdout)
}

/// Convert WAV audio data to µ-law format
pub fn convert_wav_to_ulaw(wav_data: &[u8]) -> Result<Vec<u8>, TtsError> {
    convert_wav_to_telephony_format(wav_data, "ul", "µ-law")
}

/// Convert WAV audio data to A-law format
pub fn convert_wav_to_alaw(wav_data: &[u8]) -> Result<Vec<u8>, TtsError> {
    convert_wav_to_telephony_format(wav_data, "al", "A-law")
}

/// Extract raw audio data from WAV file
fn extract_raw_audio_from_wav(wav_data: &[u8], format_name: &str) -> Result<Vec<u8>, TtsError> {
    use std::io::Cursor;

    let cursor = Cursor::new(wav_data);
    let mut reader = hound::WavReader::new(cursor).map_err(|e| {
        TtsError::AudioConversionError(format!(
            "Failed to read WAV for {} extraction: {}",
            format_name, e
        ))
    })?;

    // µ-law and A-law are 8-bit formats, but hound treats them as i8
    let samples: Result<Vec<i8>, _> = reader.samples::<i8>().collect();
    let samples = samples.map_err(|e| {
        TtsError::AudioConversionError(format!("Failed to extract {} samples: {}", format_name, e))
    })?;

    // Convert i8 to u8 (reinterpret bytes)
    Ok(samples.into_iter().map(|s| s as u8).collect())
}

/// Convert WAV to raw telephony format, handling both RIFF WAV and raw WAV
pub fn convert_to_raw_telephony(
    wav_data: &[u8],
    target_format: &crate::tts::AudioFormat,
) -> Result<Vec<u8>, TtsError> {
    use crate::tts::AudioFormat;

    if wav_data.starts_with(b"RIFF") {
        // Google TTS WAV with telephony encoding - extract raw data
        extract_raw_audio_from_wav(wav_data, &format!("{}", target_format))
    } else {
        // eSpeak WAV - convert using sox
        match target_format {
            AudioFormat::Ulaw => convert_wav_to_telephony_format(wav_data, "ul", "µ-law"),
            AudioFormat::Alaw => convert_wav_to_telephony_format(wav_data, "al", "A-law"),
            AudioFormat::Gsm => convert_wav_to_telephony_format(wav_data, "gsm", "GSM"),
            _ => Err(TtsError::AudioConversionError(format!(
                "Unsupported telephony format: {}",
                target_format
            ))),
        }
    }
}
