use clap::Parser;
use weather::{display_weather, fetch_weather_data};

#[derive(Parser, Debug)]
#[command(name = "metar", author, version, about = "Fetch aviation weather from aviationweather.gov", long_about = None)]
struct Args {
    /// ICAO airport identifier (e.g., KJFK, EGLL, KSFO)
    #[arg(value_name = "ICAO")]
    icao: String,
}

fn main() {
    let args = Args::parse();

    eprintln!("Fetching weather for {}...\n", args.icao.to_uppercase());

    match fetch_weather_data(&args.icao) {
        Ok(metar) => display_weather(&metar),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
#[path = "common/man_page.rs"]
mod man_page;

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Args::command().debug_assert();
    }

    #[test]
    fn man_pages_are_up_to_date() {
        man_page::check(Args::command());
    }
}
