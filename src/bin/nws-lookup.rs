use clap::Parser;
use weather::nws::{Location, NwsClient};

#[derive(Parser, Debug)]
#[command(
    name = "nws-lookup",
    author,
    version,
    about = "Find the National Weather Service identifiers for a location",
    long_about = None
)]
struct Args {
    /// ICAO station (KSJC), latitude,longitude (37.3592,-121.9242) or US street address
    #[arg(value_name = "LOCATION", required = true)]
    location: Vec<String>,

    /// Number of nearby observation stations to list
    #[arg(short = 'n', long, default_value_t = 5)]
    stations: usize,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let location: Location = args.location.join(" ").parse()?;

    let nws = NwsClient::new()?;
    let place = nws.locate(&location)?;
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

    #[test]
    fn joins_an_unquoted_address() {
        let args = Args::try_parse_from([
            "nws-lookup",
            "200",
            "E",
            "Santa",
            "Clara",
            "St,",
            "San",
            "Jose,",
            "CA",
        ])
        .unwrap();

        assert_eq!(
            args.location.join(" "),
            "200 E Santa Clara St, San Jose, CA"
        );
    }
}
