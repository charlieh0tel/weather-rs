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
    sox_convert(wav_data, &["-t", sox_format], 8000, format_name)
}

fn sox_convert(
    wav_data: &[u8],
    output_type: &[&str],
    sample_rate: u32,
    format_name: &str,
) -> Result<Vec<u8>, TtsError> {
    let mut temp_file = tempfile::Builder::new()
        .suffix(".wav")
        .tempfile()
        .map_err(|e| {
            TtsError::AudioConversionError(format!("Failed to create temp file: {}", e))
        })?;

    use std::io::Write;
    temp_file
        .write_all(wav_data)
        .map_err(|e| TtsError::AudioConversionError(format!("Failed to write temp WAV: {}", e)))?;

    let temp_path = temp_file.path();

    let sample_rate = sample_rate.to_string();
    let mut sox_args = vec![temp_path.to_str().unwrap()];
    sox_args.extend_from_slice(output_type);
    sox_args.extend_from_slice(&["-r", &sample_rate, "-c", "1", "-"]);
    eprintln!("Running: sox {}", sox_args.join(" "));

    let output = Command::new("sox").args(sox_args).output().map_err(|e| {
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

/// Asterisk's `.sln16` is the samples alone: 16-bit little-endian, mono, 16 kHz.  A WAV already
/// in that form, as Google returns, only loses its header; anything else is resampled by sox.
fn convert_wav_to_sln16(wav_data: &[u8]) -> Result<Vec<u8>, TtsError> {
    let reader = hound::WavReader::new(wav_data)
        .map_err(|e| TtsError::AudioConversionError(format!("Failed to read WAV: {e}")))?;
    let spec = reader.spec();
    if spec.sample_rate != 16000
        || spec.channels != 1
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        return sox_convert(
            wav_data,
            &["-t", "raw", "-e", "signed-integer", "-b", "16", "-L"],
            16000,
            "SLN16",
        );
    }

    let samples = reader
        .into_samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TtsError::AudioConversionError(format!("Failed to read WAV: {e}")))?;
    Ok(samples.into_iter().flat_map(i16::to_le_bytes).collect())
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
        AudioFormat::Sln16 => convert_wav_to_sln16(wav_data),
        _ => Err(TtsError::AudioConversionError(format!(
            "Unsupported telephony format: {}",
            target_format
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tts::AudioFormat;

    fn wav(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = std::io::Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for sample in samples {
            writer.write_sample(*sample).unwrap();
        }
        writer.finalize().unwrap();
        cursor.into_inner()
    }

    #[test]
    fn sln16_from_a_16_khz_wav_is_its_samples_without_a_header() {
        let samples = [0, 1, -1, i16::MAX, i16::MIN, 0x1234];
        let raw = convert_to_raw_telephony(&wav(16000, &samples), &AudioFormat::Sln16).unwrap();
        let expected: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        assert_eq!(raw, expected);
    }

    #[test]
    fn sln16_rejects_data_that_is_not_wav() {
        assert!(convert_to_raw_telephony(b"not a wav file", &AudioFormat::Sln16).is_err());
    }
}
