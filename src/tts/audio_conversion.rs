use crate::tts::TtsError;

use std::io::Write;
/// Audio format conversion utilities
///
/// This module provides functions to convert between different audio formats,
/// primarily for telephony applications that require specific formats like GSM.
use std::process::{Command, Stdio};

/// Convert WAV audio data to GSM format using external tools
pub fn convert_wav_to_gsm(wav_data: &[u8]) -> Result<Vec<u8>, TtsError> {
    // Try ffmpeg first, then sox as fallback
    let result = try_ffmpeg_conversion(wav_data).or_else(|_| try_sox_conversion(wav_data));

    result.map_err(|e| {
        TtsError::AudioConversionError(format!(
            "GSM conversion failed. Install ffmpeg or sox for audio conversion. Error: {}",
            e
        ))
    })
}

fn try_ffmpeg_conversion(wav_data: &[u8]) -> Result<Vec<u8>, String> {
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-f", "wav", // Input format
            "-i", "-", // Read from stdin
            "-f", "gsm", // Output format
            "-ar", "8000", // 8kHz sample rate for GSM
            "-ac", "1",  // Mono
            "-y", // Overwrite output
            "-",  // Write to stdout
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;

    // Write WAV data to ffmpeg stdin
    if let Some(stdin) = ffmpeg.stdin.take() {
        let wav_data_owned = wav_data.to_vec();
        std::thread::spawn(move || {
            let mut stdin = stdin;
            let _ = stdin.write_all(&wav_data_owned);
        });
    }

    // Get output
    let output = ffmpeg
        .wait_with_output()
        .map_err(|e| format!("FFmpeg execution failed: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "FFmpeg failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(output.stdout)
}

fn try_sox_conversion(wav_data: &[u8]) -> Result<Vec<u8>, String> {
    let mut sox = Command::new("sox")
        .args([
            "-t", "wav", "-", // Read WAV from stdin
            "-t", "gsm", // Output GSM format
            "-r", "8000", // 8kHz sample rate
            "-c", "1", // Mono
            "-", // Write to stdout
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn sox: {}", e))?;

    // Write WAV data to sox stdin
    if let Some(stdin) = sox.stdin.take() {
        let wav_data_owned = wav_data.to_vec();
        std::thread::spawn(move || {
            let mut stdin = stdin;
            let _ = stdin.write_all(&wav_data_owned);
        });
    }

    // Get output
    let output = sox
        .wait_with_output()
        .map_err(|e| format!("Sox execution failed: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Sox failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(output.stdout)
}

// Future: Add more conversion functions
// pub fn convert_wav_to_alaw(wav_data: &[u8]) -> Result<Vec<u8>, TtsError>
// pub fn convert_wav_to_ulaw(wav_data: &[u8]) -> Result<Vec<u8>, TtsError>
