use clap::{Args, Parser, Subcommand};
use std::error::Error;
use weather::{
    nws::{Location, NwsClient},
    report,
    tts::{AnnouncementFormat, TemperatureUnit, generate_weather_announcement},
};

/// Weather reports, printed, spoken or played on an AllStar node.
#[derive(Parser, Debug)]
#[command(name = "wx", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Current conditions from the nearest METAR station, from aviationweather.gov
    Metar(MetarArgs),
    /// The National Weather Service forecast (US only)
    Nws(NwsArgs),
    /// Active National Weather Service alerts (US only)
    Alerts(AlertsArgs),
    /// The National Weather Service office, grid, zones and nearest stations for a place
    Where(WhereArgs),
}

#[derive(Args, Debug)]
struct PlaceArgs {
    /// A station (KSJC), latitude,longitude, US street address or place name ("Mountain View,
    /// CA"); quotes are optional
    #[arg(value_name = "PLACE", required = true, num_args = 1.., allow_negative_numbers = true)]
    place: Vec<String>,
}

impl PlaceArgs {
    fn location(&self) -> Result<Location, weather::WeatherError> {
        self.place.join(" ").parse()
    }
}

#[derive(Args, Debug)]
struct MetarArgs {
    #[command(flatten)]
    place: PlaceArgs,

    /// How to word the announcement
    #[arg(short, long, value_enum, default_value = "full")]
    style: AnnouncementFormat,

    /// Temperature unit (the detailed style always gives both)
    #[arg(short, long, value_enum, default_value = "fahrenheit")]
    temp_unit: TemperatureUnit,

    /// Print the raw METAR instead of an announcement
    #[arg(long)]
    raw: bool,
}

#[derive(Args, Debug)]
struct NwsArgs {
    #[command(flatten)]
    place: PlaceArgs,

    /// Number of half-day forecast periods
    #[arg(short, long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(1..))]
    periods: u8,
}

#[derive(Args, Debug)]
struct AlertsArgs {
    #[command(flatten)]
    place: PlaceArgs,

    /// Announce nothing when no alerts are in effect, instead of saying so
    #[arg(long)]
    only_if_any: bool,
}

#[derive(Args, Debug)]
struct WhereArgs {
    #[command(flatten)]
    place: PlaceArgs,

    /// Number of nearby observation stations to list
    #[arg(short = 'n', long, default_value_t = 5)]
    stations: usize,
}

fn print_where(args: &WhereArgs) -> Result<(), Box<dyn Error>> {
    let nws = NwsClient::new()?;
    let place = nws.locate(&args.place.location()?)?;
    let point = nws.point(place.coordinates)?;
    let stations = if args.stations > 0 {
        nws.stations(&point, args.stations)?
    } else {
        Vec::new()
    };

    let unknown = || "unknown".to_string();
    let field = |label: &str, value: &str| println!("{label:<17}{value}");

    match &place.name {
        Some(name) => field("Location:", &format!("{} ({name})", place.coordinates)),
        None => field("Location:", &place.coordinates.to_string()),
    }
    if let Some(city) = point.nearest_city() {
        field("Nearest city:", &format!("{}, {}", city.city, city.state));
    }
    field("Forecast office:", &point.office);
    field("Grid:", &point.grid());
    field(
        "Forecast zone:",
        &point.forecast_zone().map_or_else(unknown, str::to_string),
    );
    field(
        "County zone:",
        &point.county_zone().map_or_else(unknown, str::to_string),
    );
    if let Some(time_zone) = &point.time_zone {
        field("Time zone:", time_zone);
    }
    for (i, station) in stations.iter().enumerate() {
        let label = if i == 0 { "Stations:" } else { "" };
        let distance = station
            .distance
            .map(|metres| format!(" ({:.1} km)", metres / 1000.0))
            .unwrap_or_default();
        field(
            label,
            &format!(
                "{:<6} {}{distance}",
                station.station_identifier, station.name
            ),
        );
    }
    Ok(())
}

/// The text to announce, or `None` when there is nothing to announce.
fn announcement(command: &Command) -> Result<Option<String>, Box<dyn Error>> {
    Ok(match command {
        Command::Metar(args) => {
            let metar = report::metar(&args.place.location()?)?;
            Some(if args.raw {
                metar.raw_ob
            } else {
                generate_weather_announcement(&metar, &args.style, args.temp_unit)
            })
        }
        Command::Nws(args) => Some(report::nws_forecast(
            &args.place.location()?,
            args.periods.into(),
        )?),
        Command::Alerts(args) => report::alerts(&args.place.location()?, args.only_if_any)?,
        Command::Where(_) => unreachable!("where prints its own output"),
    })
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    if let Command::Where(args) = &cli.command {
        return print_where(args);
    }
    match announcement(&cli.command)? {
        Some(text) => println!("{}", text.trim()),
        None => eprintln!("{}", report::NOTHING_TO_ANNOUNCE),
    }
    Ok(())
}

fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
#[path = "common/man_page.rs"]
mod man_page;

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(["wx"].iter().chain(args))
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn man_pages_are_up_to_date() {
        man_page::check(Cli::command());
    }

    #[test]
    fn places_need_no_quotes() {
        let Command::Metar(args) = parse(&["metar", "Mountain", "View,", "CA"])
            .unwrap()
            .command
        else {
            panic!()
        };
        assert_eq!(
            args.place.location().unwrap(),
            Location::Name("Mountain View, CA".to_string())
        );

        let Command::Nws(args) = parse(&["nws", "37.3,", "-121.9"]).unwrap().command else {
            panic!()
        };
        assert!(matches!(
            args.place.location().unwrap(),
            Location::Coordinates(_)
        ));
    }

    #[test]
    fn options_belong_to_their_report() {
        assert!(parse(&["nws", "--periods", "3", "KSJC"]).is_ok());
        assert!(parse(&["alerts", "--only-if-any", "KSJC"]).is_ok());
        assert!(parse(&["metar", "--periods", "3", "KSJC"]).is_err());
        assert!(parse(&["nws", "--only-if-any", "KSJC"]).is_err());
        assert!(parse(&["alerts", "--style", "brief", "KSJC"]).is_err());
        assert!(parse(&["nws", "--periods", "0", "KSJC"]).is_err());
        assert!(parse(&["metar"]).is_err());
    }
}
