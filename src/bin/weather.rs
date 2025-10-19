use clap::{Parser, ValueEnum};
use weather::tts::{AnnouncementFormat, generate_weather_announcement};
use weather::{display_weather, fetch_weather_data};

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    /// Standard formatted output (default)
    Default,
    /// TTS-friendly text output
    Tts,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Fetch aviation weather from aviationweather.gov", long_about = None)]
struct Args {
    /// ICAO airport identifier (e.g., KJFK, EGLL, KSFO)
    #[arg(value_name = "ICAO")]
    icao: String,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Default)]
    format: OutputFormat,
}

fn main() {
    let args = Args::parse();

    match args.format {
        OutputFormat::Default => {
            println!("Fetching weather for {}...\n", args.icao.to_uppercase());
        }
        OutputFormat::Tts => {} // No status message for TTS output
    }

    match fetch_weather_data(&args.icao) {
        Ok(metar) => match args.format {
            OutputFormat::Default => display_weather(&metar),
            OutputFormat::Tts => {
                let announcement =
                    generate_weather_announcement(&metar, &AnnouncementFormat::Speech);
                println!("{}", announcement.trim());
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
