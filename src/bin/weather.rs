use clap::Parser;
use weather::{display_weather, fetch_weather_data};

#[derive(Parser, Debug)]
#[command(author, version, about = "Fetch aviation weather from aviationweather.gov", long_about = None)]
struct Args {
    /// ICAO airport identifier (e.g., KJFK, EGLL, KSFO)
    #[arg(value_name = "ICAO")]
    icao: String,
}

fn main() {
    let args = Args::parse();

    println!("Fetching weather for {}...\n", args.icao.to_uppercase());

    match fetch_weather_data(&args.icao) {
        Ok(metar) => display_weather(&metar),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
