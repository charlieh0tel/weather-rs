//! Conversion from WAV to the headerless formats Asterisk plays.

use crate::tts::{AudioFormat, TtsError};
use std::io::Write;
use std::process::Command;

/// Converts WAV audio to a raw format.  A WAV already holding `.sln16`'s samples, as Google
/// returns, only loses its header; everything else goes through sox.
pub fn wav_to_raw(wav_data: &[u8], format: &AudioFormat) -> Result<Vec<u8>, TtsError> {
    let sox_type: &[&str] = match format {
        AudioFormat::Ulaw => &["-t", "ul"],
        AudioFormat::Alaw => &["-t", "al"],
        AudioFormat::Gsm => &["-t", "gsm"],
        AudioFormat::Sln16 => {
            if let Some(samples) = sln16_samples(wav_data)? {
                return Ok(samples);
            }
            &["-t", "raw", "-e", "signed-integer", "-b", "16", "-L"]
        }
        _ => {
            return Err(TtsError::AudioConversionError(format!(
                "{format} is not a raw format"
            )));
        }
    };
    sox(wav_data, sox_type, format)
}

/// The samples of a WAV that is already 16-bit mono at `.sln16`'s rate, else `None`.
fn sln16_samples(wav_data: &[u8]) -> Result<Option<Vec<u8>>, TtsError> {
    let read_error = |e| TtsError::AudioConversionError(format!("Failed to read WAV: {e}"));

    let reader = hound::WavReader::new(wav_data).map_err(read_error)?;
    let spec = reader.spec();
    if spec.sample_rate != AudioFormat::Sln16.sample_rate()
        || spec.channels != 1
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        return Ok(None);
    }

    let samples = reader
        .into_samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(read_error)?;
    Ok(Some(
        samples.into_iter().flat_map(i16::to_le_bytes).collect(),
    ))
}

fn sox(wav_data: &[u8], sox_type: &[&str], format: &AudioFormat) -> Result<Vec<u8>, TtsError> {
    let error = |message: String| TtsError::AudioConversionError(message);

    let mut input = tempfile::Builder::new()
        .suffix(".wav")
        .tempfile()
        .map_err(|e| error(format!("Failed to create temp file: {e}")))?;
    input
        .write_all(wav_data)
        .map_err(|e| error(format!("Failed to write temp WAV: {e}")))?;

    let input_path = input
        .path()
        .to_str()
        .ok_or_else(|| error("Temp file path is not UTF-8".to_string()))?;
    let sample_rate = format.sample_rate().to_string();
    let mut args = vec![input_path];
    args.extend_from_slice(sox_type);
    args.extend_from_slice(&["-r", &sample_rate, "-c", "1", "-"]);
    eprintln!("Running: sox {}", args.join(" "));

    let output = Command::new("sox")
        .args(&args)
        .output()
        .map_err(|e| error(format!("Failed to run sox for {format}: {e}")))?;
    if !output.status.success() {
        return Err(error(format!(
            "sox failed to convert to {format}: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let raw = wav_to_raw(&wav(16000, &samples), &AudioFormat::Sln16).unwrap();
        let expected: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        assert_eq!(raw, expected);
    }

    #[test]
    fn sln16_rejects_data_that_is_not_wav() {
        assert!(wav_to_raw(b"not a wav file", &AudioFormat::Sln16).is_err());
    }

    #[test]
    fn only_raw_formats_are_converted() {
        assert!(wav_to_raw(&wav(16000, &[0]), &AudioFormat::Mp3).is_err());
    }
}
