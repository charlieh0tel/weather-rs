use aviation_weather::{
    celsius_to_fahrenheit, expand_abbreviations, fetch_weather_data, parse_wmo_codes, MetarData,
};
use clap::Parser;
use espeakng_sys::*;
use std::ffi::CString;

#[derive(Parser, Debug)]
#[command(author, version, about = "Speak aviation weather from aviationweather.gov", long_about = None)]
struct Args {
    /// ICAO airport identifier (e.g., KJFK, EGLL, KSFO)
    #[arg(value_name = "ICAO")]
    icao: String,

    /// Output format for audio
    #[arg(short, long, value_enum, default_value = "speech")]
    format: OutputFormat,

    /// Save audio to file instead of speaking
    #[arg(short, long)]
    output: Option<String>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum OutputFormat {
    /// Direct speech output
    Speech,
    /// Brief announcement format
    Brief,
    /// Detailed weather report
    Detailed,
    /// Aviation radio style
    Aviation,
}

fn spell_out_icao(icao: &str) -> String {
    icao.chars()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn generate_weather_announcement(metar: &MetarData, format: &OutputFormat) -> String {
    match format {
        OutputFormat::Speech | OutputFormat::Brief => {
            let mut announcement = format!("Weather for {}... ", spell_out_icao(&metar.icao_id));

            if let Some(ref name) = metar.name {
                announcement.push_str(&format!("{}... ", expand_abbreviations(name)));
            }

            if let Some(temp_c) = metar.temp {
                let temp_f = celsius_to_fahrenheit(temp_c);
                announcement.push_str(&format!(
                    "Temperature... {} degrees fahrenheit... ",
                    temp_f.round() as i32
                ));
            }

            if let Some(ref wx) = metar.wx_string {
                let codes = parse_wmo_codes(wx);
                if !codes.is_empty() {
                    announcement.push_str("Current conditions... ");
                    let conditions: Vec<String> =
                        codes.iter().map(|c| c.description().to_string()).collect();
                    announcement.push_str(&conditions.join("... "));
                    announcement.push_str("...");
                } else {
                    announcement.push_str("Clear conditions...");
                }
            } else {
                announcement.push_str("Clear conditions...");
            }

            announcement
        }

        OutputFormat::Detailed => {
            let mut announcement = format!(
                "Detailed weather report for {}... ",
                spell_out_icao(&metar.icao_id)
            );

            if let Some(ref name) = metar.name {
                announcement.push_str(&format!("{}... ", expand_abbreviations(name)));
            }

            announcement.push_str(&format!("Raw METAR... {}... ", metar.raw_ob));

            if let Some(temp_c) = metar.temp {
                let temp_f = celsius_to_fahrenheit(temp_c);
                announcement.push_str(&format!(
                    "Temperature... {} degrees fahrenheit... {} degrees celsius... ",
                    temp_f.round() as i32,
                    temp_c.round() as i32
                ));
            } else {
                announcement.push_str("Temperature... not available... ");
            }

            if let Some(ref wx) = metar.wx_string {
                announcement.push_str(&format!("Weather string... {}... ", wx));
                let codes = parse_wmo_codes(wx);
                if !codes.is_empty() {
                    announcement.push_str("Weather codes found... ");
                    let code_descriptions: Vec<String> = codes
                        .iter()
                        .map(|c| format!("{} ({})", c.description(), c.code()))
                        .collect();
                    announcement.push_str(&code_descriptions.join("... "));
                    announcement.push_str("...");
                } else {
                    announcement.push_str("No weather codes found...");
                }
            } else {
                announcement
                    .push_str("Weather... clear or not reported... No weather codes found...");
            }

            announcement
        }

        OutputFormat::Aviation => {
            let mut announcement = format!("{} weather... ", spell_out_icao(&metar.icao_id));

            if let Some(temp_c) = metar.temp {
                let temp_f = celsius_to_fahrenheit(temp_c);
                announcement.push_str(&format!(
                    "Temperature {} degrees... ",
                    temp_f.round() as i32
                ));
            }

            if let Some(ref wx) = metar.wx_string {
                let codes = parse_wmo_codes(wx);
                if !codes.is_empty() {
                    for code in codes {
                        announcement.push_str(&format!("{}... ", code.description()));
                    }
                } else {
                    announcement.push_str("Clear... ");
                }
            } else {
                announcement.push_str("Clear... ");
            }

            announcement.push_str("End weather...");
            announcement
        }
    }
}

fn main() {
    let args = Args::parse();

    println!("Fetching weather for {}...\n", args.icao.to_uppercase());

    let metar = match fetch_weather_data(&args.icao) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let announcement = generate_weather_announcement(&metar, &args.format);
    println!("Announcement text: {}\n", announcement);

    if let Some(output_file) = args.output {
        println!("Saving audio to: {}", output_file);
        // TODO: Implement audio file saving
        println!("Note: Audio file saving not yet implemented");
    } else {
        println!("Speaking weather...");

        unsafe {
            // Initialize eSpeak-ng
            let result = espeak_Initialize(
                espeak_AUDIO_OUTPUT_AUDIO_OUTPUT_PLAYBACK,
                0,
                std::ptr::null(),
                0,
            );

            if result < 0 {
                eprintln!("Failed to initialize eSpeak-ng");
                std::process::exit(1);
            }

            // Set speech parameters for better quality
            espeak_SetParameter(espeak_PARAMETER_espeakRATE, 140, 0); // 140 words per minute
            espeak_SetParameter(espeak_PARAMETER_espeakPITCH, 50, 0); // Normal pitch
            espeak_SetParameter(espeak_PARAMETER_espeakRANGE, 50, 0); // Pitch range

            // Convert text to CString
            let text_cstring = match CString::new(announcement) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to convert text to CString: {}", e);
                    std::process::exit(1);
                }
            };

            // Speak the text
            let speak_result = espeak_Synth(
                text_cstring.as_ptr() as *const std::os::raw::c_void,
                text_cstring.as_bytes().len(),
                0,
                espeak_POSITION_TYPE_POS_CHARACTER,
                0,
                espeakCHARS_UTF8 | espeakSSML | espeakPHONEMES,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            if speak_result != espeak_ERROR_EE_OK {
                eprintln!("eSpeak-ng synthesis failed");
                std::process::exit(1);
            }

            // Wait for speech to complete
            espeak_Synchronize();

            // Cleanup
            espeak_Terminate();
        }
    }
}
