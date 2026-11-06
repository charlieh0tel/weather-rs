use crate::tts::{AudioFormat, TtsBackend, TtsError, Voice};
use espeakng_sys::{
    espeak_AUDIO_OUTPUT_AUDIO_OUTPUT_SYNCHRONOUS, espeak_ERROR, espeak_ERROR_EE_INTERNAL_ERROR,
    espeak_ERROR_EE_OK, espeak_EVENT, espeak_Initialize, espeak_PARAMETER_espeakPITCH,
    espeak_PARAMETER_espeakRATE, espeak_PARAMETER_espeakWORDGAP,
    espeak_POSITION_TYPE_POS_CHARACTER, espeak_SetParameter, espeak_SetSynthCallback,
    espeak_SetVoiceByName, espeak_Synth, espeakCHARS_UTF8, espeakINITIALIZE_DONT_EXIT,
};
use std::ffi::CString;
use std::io::Cursor;
use std::sync::{Mutex, MutexGuard, PoisonError};

// eSpeak keeps its state in globals, so only one session can exist at a time.  The value is the
// sample rate once eSpeak is initialized.  It is initialized once per process and never
// terminated: on eSpeak 1.50 and 1.51 a second cycle hangs in its terminate call, and on 1.52
// terminating straight after initializing can too.
static ENGINE: Mutex<Option<i32>> = Mutex::new(None);
static SAMPLES: Mutex<Vec<i16>> = Mutex::new(Vec::new());

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn check(result: espeak_ERROR, action: &str) -> Result<(), TtsError> {
    if result == espeak_ERROR_EE_OK {
        Ok(())
    } else {
        Err(TtsError::SynthesisError(format!(
            "eSpeak failed to {action} (error {result})"
        )))
    }
}

unsafe extern "C" fn collect_samples(
    wav: *mut i16,
    numsamples: i32,
    _events: *mut espeak_EVENT,
) -> i32 {
    if !wav.is_null() && numsamples > 0 {
        // SAFETY: eSpeak passes a buffer holding `numsamples` samples, valid during this call.
        let samples = unsafe { std::slice::from_raw_parts(wav, numsamples as usize) };
        lock(&SAMPLES).extend_from_slice(samples);
    }
    0
}

/// Exclusive use of the initialized eSpeak library.
struct Session {
    sample_rate: i32,
    _engine: MutexGuard<'static, Option<i32>>,
}

impl Session {
    fn open(voice: &EspeakVoice) -> Result<Self, TtsError> {
        let mut engine = lock(&ENGINE);

        let sample_rate = match *engine {
            Some(sample_rate) => sample_rate,
            None => {
                // SAFETY: the lock gives exclusive use of eSpeak, and a null path means the
                // default data.
                let sample_rate = unsafe {
                    espeak_Initialize(
                        espeak_AUDIO_OUTPUT_AUDIO_OUTPUT_SYNCHRONOUS,
                        0,
                        std::ptr::null(),
                        espeakINITIALIZE_DONT_EXIT as i32,
                    )
                };
                if sample_rate == espeak_ERROR_EE_INTERNAL_ERROR {
                    return Err(TtsError::SynthesisError(
                        "Failed to initialize eSpeak".to_string(),
                    ));
                }
                *engine = Some(sample_rate);
                sample_rate
            }
        };

        let session = Self {
            sample_rate,
            _engine: engine,
        };
        session.configure(voice)?;
        Ok(session)
    }

    fn configure(&self, voice: &EspeakVoice) -> Result<(), TtsError> {
        let name = CString::new(voice.name.as_str())
            .map_err(|e| TtsError::SynthesisError(format!("Invalid voice name: {e}")))?;

        // SAFETY: `name` is a valid C string for the duration of the call.
        check(
            unsafe { espeak_SetVoiceByName(name.as_ptr()) },
            &format!("set voice '{}'", voice.name),
        )?;

        for (parameter, value) in [
            (espeak_PARAMETER_espeakRATE, voice.speed),
            (espeak_PARAMETER_espeakPITCH, voice.pitch),
            (espeak_PARAMETER_espeakWORDGAP, voice.gap),
        ] {
            // SAFETY: plain values, and the session holds exclusive use of eSpeak.
            check(
                unsafe { espeak_SetParameter(parameter, value as i32, 0) },
                "set a voice parameter",
            )?;
        }
        Ok(())
    }

    fn say(&self, text: &str) -> Result<(), TtsError> {
        let text = CString::new(text)
            .map_err(|e| TtsError::SynthesisError(format!("Invalid text: {e}")))?;

        // SAFETY: `text` is NUL-terminated, and the size includes the terminator as eSpeak
        // requires.  In synchronous mode this returns once synthesis has finished, so there
        // must be no call to Synchronize, which never returns in this mode on eSpeak 1.50.
        check(
            unsafe {
                espeak_Synth(
                    text.as_ptr().cast(),
                    text.as_bytes_with_nul().len(),
                    0,
                    espeak_POSITION_TYPE_POS_CHARACTER,
                    0,
                    espeakCHARS_UTF8,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            "synthesize speech",
        )
    }
}

fn wav_bytes(samples: &[i16], sample_rate: u32) -> Result<Vec<u8>, TtsError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut cursor, spec)
        .map_err(|e| TtsError::FileError(format!("Failed to create WAV writer: {e}")))?;
    for sample in samples {
        writer
            .write_sample(*sample)
            .map_err(|e| TtsError::FileError(format!("Failed to write sample: {e}")))?;
    }
    writer
        .finalize()
        .map_err(|e| TtsError::FileError(format!("Failed to finalize WAV: {e}")))?;

    Ok(cursor.into_inner())
}

#[derive(Debug, Clone)]
pub struct EspeakVoice {
    pub name: String,
    pub speed: u32,
    pub pitch: u32,
    pub gap: u32,
}

impl Default for EspeakVoice {
    fn default() -> Self {
        Self {
            name: "en-us".to_string(),
            speed: 120,
            pitch: 50,
            gap: 0,
        }
    }
}

impl EspeakVoice {
    pub fn us_female() -> Self {
        Self {
            name: "en-us+f3".to_string(),
            speed: 120,
            pitch: 60,
            gap: 0,
        }
    }

    pub fn us_male() -> Self {
        Self {
            name: "en-us+m3".to_string(),
            speed: 120,
            pitch: 40,
            gap: 0,
        }
    }

    pub fn uk_female() -> Self {
        Self {
            name: "en+f3".to_string(),
            ..Self::us_female()
        }
    }

    pub fn uk_male() -> Self {
        Self {
            name: "en+m3".to_string(),
            ..Self::us_male()
        }
    }
}

impl From<Voice> for EspeakVoice {
    fn from(voice: Voice) -> Self {
        match voice {
            Voice::Default => EspeakVoice::default(),
            Voice::UsFemale => EspeakVoice::us_female(),
            Voice::UsMale => EspeakVoice::us_male(),
            Voice::UkFemale => EspeakVoice::uk_female(),
            Voice::UkMale => EspeakVoice::uk_male(),
        }
    }
}

pub struct EspeakTts {
    voice: EspeakVoice,
}

impl EspeakTts {
    pub fn new(voice: EspeakVoice) -> Result<Self, TtsError> {
        Ok(Self { voice })
    }
}

impl TtsBackend for EspeakTts {
    fn synthesize(&self, text: &str, format: &AudioFormat) -> Result<Vec<u8>, TtsError> {
        let session = Session::open(&self.voice)?;

        lock(&SAMPLES).clear();
        // SAFETY: the callback has the signature eSpeak expects and stays valid for the program.
        unsafe { espeak_SetSynthCallback(Some(collect_samples)) };
        session.say(text)?;

        // Taken while the session still gives exclusive use of the buffer.
        let samples = std::mem::take(&mut *lock(&SAMPLES));
        let sample_rate = session.sample_rate as u32;
        drop(session);

        if samples.is_empty() {
            return Err(TtsError::SynthesisError(
                "No audio data generated by eSpeak".to_string(),
            ));
        }

        let wav_data = wav_bytes(&samples, sample_rate)?;
        crate::tts::TtsPlayer::convert_audio_format(&wav_data, &AudioFormat::Wav, format)
    }

    fn speak(&self, text: &str) -> Result<(), TtsError> {
        let wav_data = self.synthesize(text, &AudioFormat::Wav)?;
        crate::tts::TtsPlayer::play_audio(&wav_data, &AudioFormat::Wav)
    }

    fn backend_name(&self) -> &str {
        "eSpeak-NG"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "Weather for K S F O... Temperature... 68 degrees fahrenheit...";

    fn synthesize_samples(tts: &EspeakTts, text: &str) -> Vec<i16> {
        let wav = tts.synthesize(text, &AudioFormat::Wav).unwrap();
        hound::WavReader::new(wav.as_slice())
            .unwrap()
            .samples::<i16>()
            .map(|sample| sample.unwrap())
            .collect()
    }

    fn default_tts() -> EspeakTts {
        EspeakTts::new(EspeakVoice::default()).unwrap()
    }

    #[test]
    fn speaks_only_the_given_text_whatever_is_on_the_heap() {
        let tts = default_tts();
        let expected = synthesize_samples(&tts, TEXT).len();
        // Repeated runs naturally vary by about 1%; speaking extra text adds far more.
        let tolerance = expected / 20;

        for _ in 0..10 {
            // Leaves non-zero freed chunks of the size eSpeak allocates for its copy of the text.
            let garbage: Vec<Vec<u8>> = (0..64).map(|_| vec![b'x'; TEXT.len()]).collect();
            drop(garbage);

            let actual = synthesize_samples(&tts, TEXT).len();
            assert!(
                actual.abs_diff(expected) <= tolerance,
                "expected about {expected} samples, got {actual}"
            );
        }
    }

    #[test]
    fn concurrent_synthesis_is_safe() {
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| assert!(!synthesize_samples(&default_tts(), TEXT).is_empty()));
            }
        });
    }

    #[test]
    fn unknown_voice_is_an_error() {
        let tts = EspeakTts::new(EspeakVoice {
            name: "no-such-voice".to_string(),
            ..EspeakVoice::default()
        })
        .unwrap();

        assert!(matches!(
            tts.synthesize(TEXT, &AudioFormat::Wav),
            Err(TtsError::SynthesisError(_))
        ));
    }

    #[test]
    fn every_voice_preset_is_known_to_espeak() {
        for voice in [
            Voice::Default,
            Voice::UsFemale,
            Voice::UsMale,
            Voice::UkFemale,
            Voice::UkMale,
        ] {
            let tts = EspeakTts::new(voice.clone().into()).unwrap();
            assert!(
                tts.synthesize(TEXT, &AudioFormat::Wav).is_ok(),
                "voice: {voice:?}"
            );
        }
    }

    fn tts_with(pitch: u32, gap: u32) -> EspeakTts {
        EspeakTts::new(EspeakVoice {
            pitch,
            gap,
            ..EspeakVoice::default()
        })
        .unwrap()
    }

    fn rms(samples: &[i16]) -> f64 {
        let sum: f64 = samples.iter().map(|s| f64::from(*s).powi(2)).sum();
        (sum / samples.len() as f64).sqrt()
    }

    #[test]
    fn pitch_does_not_change_loudness() {
        let low = rms(&synthesize_samples(&tts_with(0, 0), TEXT));
        let high = rms(&synthesize_samples(&tts_with(99, 0), TEXT));

        assert!(low > 500.0, "low pitch is nearly silent: {low}");
        assert!(high / low < 2.0 && low / high < 2.0, "{low} vs {high}");
    }

    #[test]
    fn pitch_changes_the_voice() {
        assert_ne!(
            synthesize_samples(&tts_with(0, 0), TEXT),
            synthesize_samples(&tts_with(99, 0), TEXT)
        );
    }

    #[test]
    fn gap_lengthens_speech() {
        let without = synthesize_samples(&tts_with(50, 0), TEXT).len();
        let with = synthesize_samples(&tts_with(50, 50), TEXT).len();

        assert!(with > without + without / 10, "{without} vs {with}");
    }

    #[test]
    fn synthesized_wav_can_be_decoded_for_playback() {
        let wav = default_tts().synthesize(TEXT, &AudioFormat::Wav).unwrap();

        assert!(rodio::Decoder::new(Cursor::new(wav)).is_ok());
    }
}
