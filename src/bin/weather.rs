use clap::Parser;
use weather::tts::{AnnouncementFormat, generate_weather_announcement};
use weather::{display_weather, fetch_weather_data};

#[derive(Parser, Debug)]
#[command(author, version, about = "Fetch aviation weather from aviationweather.gov", long_about = None)]
struct Args {
    /// ICAO airport identifier (e.g., KJFK, EGLL, KSFO)
    #[arg(value_name = "ICAO")]
    icao: String,

    /// Output format: 'default' for standard output, or TTS announcement formats
    #[arg(short, long, value_enum)]
    format: Option<AnnouncementFormat>,
}

fn main() {
    let args = Args::parse();

    if args.format.is_none() {
        println!("Fetching weather for {}...\n", args.icao.to_uppercase());
    }

    match fetch_weather_data(&args.icao) {
        Ok(metar) => match args.format {
            None => display_weather(&metar),
            Some(announcement_format) => {
                let announcement = generate_weather_announcement(&metar, &announcement_format);
                println!("{}", announcement.trim());
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
