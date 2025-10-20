use crate::tts::TtsError;

/// Audio format conversion utilities
///
/// This module provides functions to convert between different audio formats,
/// primarily for telephony applications that require specific formats like GSM.
use std::process::Command;

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
    let mut temp_file = tempfile::Builder::new()
        .suffix(".wav")
        .tempfile()
        .map_err(|e| {
            TtsError::AudioConversionError(format!("Failed to create temp file: {}", e))
        })?;

    use std::io::Write;
    temp_file.write_all(wav_data).map_err(|e| {
        TtsError::AudioConversionError(format!("Failed to write temp WAV: {}", e))
    })?;

    let temp_path = temp_file.path();

    let sox_args = [
        temp_path.to_str().unwrap(),
        "-t", sox_format,
        "-r", "8000",
        "-c", "1",
        "-",
    ];
    eprintln!("Running: sox {}", sox_args.join(" "));

    let output = Command::new("sox")
        .args(sox_args)
        .output()
        .map_err(|e| {
            TtsError::AudioConversionError(format!(
                "Failed to spawn sox for {} conversion: {}",
                format_name, e
            ))
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


/// Convert WAV to raw telephony format using sox
pub fn convert_to_raw_telephony(
    wav_data: &[u8],
    target_format: &crate::tts::AudioFormat,
) -> Result<Vec<u8>, TtsError> {
    use crate::tts::AudioFormat;

    // Always use sox conversion - works for both Google TTS and eSpeak WAV files
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
